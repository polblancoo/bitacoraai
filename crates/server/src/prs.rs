//! Integrar: el owner revisa y junta (o rechaza) los Pull Requests que las personas del equipo piden con `cw enviar`,
//! sin salir de la app. Se usa la conexión de GitHub de quien configuró el proyecto; después la carpeta del proyecto
//! se pone al día sola (por el agente de esa máquina).
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    github::{api_get, api_json, api_send, gh, repo_of, token_of, Github},
    gitsetup, routes, AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tm_core::{Id, Role};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects/{id}/prs", get(list))
        .route("/api/projects/{id}/prs/{n}/files", get(files))
        .route("/api/tasks/{id}/diff", get(task_diff))
        .route("/api/projects/{id}/prs/{n}/merge", post(merge))
        .route("/api/projects/{id}/prs/{n}/close", post(close))
}

/// Repositorio del proyecto, cliente de GitHub y el token con el que se opera (el de quien configuró GitHub en el proyecto; si no hay, el de la persona).
async fn ctx(st: &AppState, project: Id, user: &AuthUser, need: Role) -> Result<(Arc<Github>, String, String, Id)> {
    user.human_only()?;
    let team = routes::team_of_project(st, project).await?;
    routes::require(st, team, user.id, need).await?;
    let g = gh(st)?;
    let repo = repo_of(st, project).await?.ok_or_else(|| AppError::bad("El proyecto no tiene repositorio de GitHub"))?;
    let owner: Option<Id> = sqlx::query_scalar("SELECT owner_user FROM project_github_cfg WHERE project_id = ?").bind(project).fetch_optional(&st.db).await?;
    let token = match token_of(st, owner.unwrap_or(user.id)).await {
        Ok(t) => t,
        Err(_) => token_of(st, user.id).await?,
    };
    Ok((g, token, repo, team))
}

async fn name_of(st: &AppState, id: Id) -> String {
    sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = ?").bind(id).fetch_one(&st.db).await.unwrap_or_default()
}

fn by(pr: &Value) -> String {
    let body = pr["body"].as_str().unwrap_or("");
    body.rsplit("— Pedido desde CrewHelm por ").next().filter(|_| body.contains("— Pedido desde CrewHelm por ")).map(|s| s.lines().next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty()).unwrap_or_else(|| pr["user"]["login"].as_str().unwrap_or("").to_string())
}

async fn list(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    let (g, token, repo, team) = ctx(&st, project, &user, Role::Viewer).await?;
    let can_merge = routes::require(&st, team, user.id, Role::Owner).await.is_ok();
    let open = api_get(&g, &token, &format!("/repos/{repo}/pulls?state=open&per_page=20&sort=created&direction=desc")).await?;
    let mut out = Vec::new();
    for p in open.as_array().cloned().unwrap_or_default() {
        let n = p["number"].as_i64().unwrap_or(0);
        let d = api_get(&g, &token, &format!("/repos/{repo}/pulls/{n}")).await.unwrap_or(p.clone());
        out.push(json!({
            "number": n, "title": p["title"], "url": p["html_url"], "branch": p["head"]["ref"], "base": p["base"]["ref"], "draft": p["draft"],
            "by": by(&p), "created": p["created_at"], "files": d["changed_files"], "add": d["additions"], "del": d["deletions"],
            "commits": d["commits"], "mergeable": d["mergeable"], "state": d["mergeable_state"],
        }));
    }
    Ok(Json(json!({ "repo": repo, "can_merge": can_merge, "prs": out })))
}

async fn files(State(st): State<AppState>, user: AuthUser, Path((project, n)): Path<(Id, i64)>) -> Result<Json<Value>> {
    let (g, token, repo, _) = ctx(&st, project, &user, Role::Viewer).await?;
    Ok(Json(json!({ "files": pr_files(&g, &token, &repo, n).await? })))
}

/// Los cambios del Pull Request de una tarea, para revisarlos sin salir de la tarea.
async fn task_diff(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    let project: Id = sqlx::query_scalar("SELECT project_id FROM tasks WHERE id = ?").bind(task).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)?;
    let pr: Option<Option<i64>> = sqlx::query_scalar("SELECT pr_number FROM task_github WHERE task_id = ?").bind(task).fetch_optional(&st.db).await?;
    let Some(Some(n)) = pr else { return Ok(Json(json!({ "pr": null, "files": [] }))) };
    let (g, token, repo, _) = ctx(&st, project, &user, Role::Viewer).await?;
    Ok(Json(json!({ "pr": n, "repo": repo, "files": pr_files(&g, &token, &repo, n).await? })))
}

async fn pr_files(g: &Github, token: &str, repo: &str, n: i64) -> Result<Vec<Value>> {
    let v = api_get(g, token, &format!("/repos/{repo}/pulls/{n}/files?per_page=60")).await?;
    let mut budget = 150_000usize;
    Ok(v.as_array().cloned().unwrap_or_default().into_iter().map(|f| {
        let patch = f["patch"].as_str().unwrap_or("");
        let cut: String = patch.chars().take(8000.min(budget)).collect();
        budget = budget.saturating_sub(cut.len());
        json!({ "name": f["filename"], "status": f["status"], "add": f["additions"], "del": f["deletions"], "patch": cut, "cut": cut.len() < patch.len(), "binary": f["patch"].is_null() })
    }).collect())
}

#[derive(Deserialize)]
struct MergeBody {
    #[serde(default)]
    squash: bool,
}

async fn merge(State(st): State<AppState>, user: AuthUser, Path((project, n)): Path<(Id, i64)>, Json(b): Json<MergeBody>) -> Result<Json<Value>> {
    let (g, token, repo, team) = ctx(&st, project, &user, Role::Owner).await?;
    let pr = api_get(&g, &token, &format!("/repos/{repo}/pulls/{n}")).await?;
    if pr["state"].as_str() != Some("open") {
        return Err(AppError::bad("Este Pull Request ya no está abierto"));
    }
    if pr["draft"].as_bool() == Some(true) {
        return Err(AppError::bad("Es un borrador: todavía no está listo para integrar"));
    }
    if pr["mergeable"].as_bool() == Some(false) {
        return Err(AppError::bad("Tiene conflictos con la rama principal: la persona tiene que actualizar su copia (cw actualizar) y volver a enviarlo"));
    }
    let me = name_of(&st, user.id).await;
    let method = if b.squash { "squash" } else { "merge" };
    api_json(&g, &token, reqwest::Method::PUT, &format!("/repos/{repo}/pulls/{n}/merge"), json!({ "merge_method": method })).await?;
    // la rama de la persona ya no hace falta (solo si es del mismo repositorio)
    let (head, base) = (pr["head"]["ref"].as_str().unwrap_or("").to_string(), pr["base"]["ref"].as_str().unwrap_or("").to_string());
    if pr["head"]["repo"]["full_name"].as_str() == Some(repo.as_str()) && !head.is_empty() && head != base {
        let _ = api_send(&g, &token, reqwest::Method::DELETE, &format!("/repos/{repo}/git/refs/heads/{}", head.replace('/', "%2F").replace("%2F", "/")), json!({})).await;
    }
    // la carpeta del proyecto se pone al día (si el agente no está, no pasa nada: se actualiza en la próxima sincronización)
    let synced = match gitsetup::target(&st, project, team).await {
        Ok((agent, dir)) => gitsetup::dispatch(agent, json!({ "mode": "sync", "dir": dir }), 90).await.ok().map(|v| v["steps"].clone()),
        Err(_) => None,
    };
    let who = by(&pr);
    if let Ok(uid) = sqlx::query_scalar::<_, Id>("SELECT id FROM users WHERE username = ?").bind(&who).fetch_one(&st.db).await {
        crate::notify::notify(&st, uid, "github", Some(team), None, format!("✅ Tu trabajo «{}» fue integrado al proyecto por {}. Corré «cw actualizar» para traerlo a tu copia.", pr["title"].as_str().unwrap_or(""), me)).await;
    }
    tracing::info!("github: {} integró el Pull Request #{n} del proyecto {project}", me);
    Ok(Json(json!({ "merged": true, "synced": synced })))
}

#[derive(Deserialize)]
struct CloseBody {
    #[serde(default)]
    comment: String,
}

async fn close(State(st): State<AppState>, user: AuthUser, Path((project, n)): Path<(Id, i64)>, Json(b): Json<CloseBody>) -> Result<Json<Value>> {
    let (g, token, repo, team) = ctx(&st, project, &user, Role::Owner).await?;
    let pr = api_get(&g, &token, &format!("/repos/{repo}/pulls/{n}")).await?;
    let me = name_of(&st, user.id).await;
    let comment: String = b.comment.trim().chars().take(2000).collect();
    if !comment.is_empty() {
        let _ = api_json(&g, &token, reqwest::Method::POST, &format!("/repos/{repo}/issues/{n}/comments"), json!({ "body": format!("💬 {}: {comment}", me) })).await;
    }
    api_json(&g, &token, reqwest::Method::PATCH, &format!("/repos/{repo}/pulls/{n}"), json!({ "state": "closed" })).await?;
    let who = by(&pr);
    if let Ok(uid) = sqlx::query_scalar::<_, Id>("SELECT id FROM users WHERE username = ?").bind(&who).fetch_one(&st.db).await {
        let why = if comment.is_empty() { String::new() } else { format!(" Motivo: {comment}") };
        crate::notify::notify(&st, uid, "github", Some(team), None, format!("↩ {} devolvió tu trabajo «{}» sin integrarlo.{why}", me, pr["title"].as_str().unwrap_or(""))).await;
    }
    Ok(Json(json!({ "closed": true })))
}

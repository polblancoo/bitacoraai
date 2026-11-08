//! Revisión de trabajo sin GitHub, para los proyectos que guardan todo en el servidor. La rama de la persona ya está en el
//! repositorio de la carpeta del proyecto (la deja `cw enviar`); acá se registra el pedido y los owners lo revisan,
//! lo juntan con la rama principal o lo devuelven, con los mismos botones de «🔀 Integrar». Quien junta y quien devuelve
//! es cualquier owner del equipo. El agente de la máquina del proyecto hace los git (ver cambios, juntar).
use crate::{auth::AuthUser, error::{AppError, Result}, gitsetup, routes, AppState};
use axum::Json;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::{Id, Role};

async fn name_of(st: &AppState, id: Id) -> String {
    sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = ?").bind(id).fetch_one(&st.db).await.unwrap_or_default()
}

/// Un pedido nuevo (lo manda el agente cuando alguien usa «Enviar a revisión»). Si ya había uno abierto para esa rama, se actualiza.
pub async fn create(st: &AppState, project: Id, team: Id, who: &str, branch: &str, title: &str, body: &str, task: Option<Id>) -> Result<Json<Value>> {
    let title: String = title.trim().chars().take(200).collect();
    let title = if title.is_empty() { branch.to_string() } else { title };
    let body: String = body.trim().chars().take(7000).collect();
    let open: Option<Id> = sqlx::query_scalar("SELECT id FROM reviews WHERE project_id = ? AND branch = ? AND status = 'open'").bind(project).bind(branch).fetch_optional(&st.db).await?;
    let id = match open {
        Some(id) => {
            sqlx::query("UPDATE reviews SET title = ?, body = ?, author = ?, task_id = COALESCE(?, task_id) WHERE id = ?").bind(&title).bind(&body).bind(who).bind(task).bind(id).execute(&st.db).await?;
            id
        }
        None => sqlx::query("INSERT INTO reviews (project_id, branch, title, body, author, created_at, task_id) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(project).bind(branch).bind(&title).bind(&body).bind(who).bind(Utc::now().to_rfc3339()).bind(task).execute(&st.db).await?.last_insert_rowid(),
    };
    let author: Option<Id> = sqlx::query_scalar("SELECT id FROM users WHERE username = ?").bind(who).fetch_optional(&st.db).await?;
    if let Some(a) = author {
        crate::audit::record(st, team, Some(project), None, a, "revision_enviada", &format!("{branch}: {title}")).await;
    }
    let owners: Vec<Id> = sqlx::query_scalar("SELECT m.user_id FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND m.role = 'owner' AND u.is_bot = 0 AND m.user_id != ?")
        .bind(team).bind(author.unwrap_or(0)).fetch_all(&st.db).await?;
    for o in owners {
        crate::notify::notify(st, o, "github", Some(team), None, format!("🔀 {who} mandó «{title}» a revisión. Abrí el proyecto y tocá «🔀 Integrar» para revisarlo.")).await;
    }
    tracing::info!("revisión: {who} pidió revisar la rama {branch} del proyecto {project}");
    Ok(Json(json!({ "number": id, "url": "", "base": "" })))
}

async fn review_row(st: &AppState, project: Id, id: Id) -> Result<(String, String, String, String)> {
    let r = sqlx::query("SELECT branch, title, author, status FROM reviews WHERE id = ? AND project_id = ?").bind(id).bind(project).fetch_optional(&st.db).await?.ok_or_else(AppError::not_found)?;
    Ok((r.get(0), r.get(1), r.get(2), r.get(3)))
}

async fn dispatch(st: &AppState, project: Id, team: Id, payload: Value, secs: u64) -> Result<Value> {
    let (agent, dir) = gitsetup::target(st, project, team).await?;
    let mut p = json!({ "mode": "review", "dir": dir });
    for (k, v) in payload.as_object().cloned().unwrap_or_default() {
        p[k] = v;
    }
    gitsetup::dispatch(agent, p, secs).await
}

pub async fn list(st: &AppState, user: &AuthUser, project: Id) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(st, project).await?;
    routes::require(st, team, user.id, Role::Viewer).await?;
    let can_merge = routes::require(st, team, user.id, Role::Owner).await.is_ok();
    let rows = sqlx::query("SELECT id, branch, title, body, author, created_at, task_id FROM reviews WHERE project_id = ? AND status = 'open' ORDER BY id DESC LIMIT 30").bind(project).fetch_all(&st.db).await?;
    let branches: Vec<String> = rows.iter().map(|r| r.get::<String, _>(1)).collect();
    // el estado real de cada rama lo mira el agente; si no está conectado se muestra la lista igual
    let info = if branches.is_empty() { None } else { dispatch(st, project, team, json!({ "action": "info", "branches": branches }), 60).await.ok() };
    let base = info.as_ref().and_then(|i| i["base"].as_str()).unwrap_or("").to_string();
    let find = |b: &str| info.as_ref().and_then(|i| i["branches"].as_array()).and_then(|a| a.iter().find(|x| x["branch"] == b)).cloned();
    let mut cards = std::collections::HashMap::new();
    for r in &rows {
        if let Some(t) = r.get::<Option<Id>, _>(6) {
            cards.insert(t, crate::taskfiles::task_card(st, project, t).await);
        }
    }
    let prs: Vec<Value> = rows.iter().map(|r| {
        let (id, branch, title): (Id, String, String) = (r.get(0), r.get(1), r.get(2));
        let i = find(&branch);
        let gone = i.as_ref().map(|x| x["exists"] == false).unwrap_or(false);
        let state = match (&i, gone) { (None, _) => "unknown", (_, true) => "gone", (Some(x), _) => if x["mergeable"] == true { "clean" } else { "dirty" } };
        json!({ "number": id, "title": title, "url": "", "branch": branch, "base": base, "draft": false, "by": r.get::<String, _>(4), "created": r.get::<String, _>(5),
                "files": i.as_ref().map(|x| x["files"].clone()), "add": i.as_ref().map(|x| x["add"].clone()), "del": i.as_ref().map(|x| x["del"].clone()),
                "commits": i.as_ref().map(|x| x["commits"].clone()), "task": r.get::<Option<Id>, _>(6).and_then(|t| cards.get(&t).cloned().flatten()), "mergeable": if state == "dirty" { json!(false) } else if state == "clean" { json!(true) } else { Value::Null }, "state": state })
    }).collect();
    Ok(Json(json!({ "repo": "este servidor (sin GitHub)", "internal": true, "can_merge": can_merge, "prs": prs })))
}

pub async fn files(st: &AppState, user: &AuthUser, project: Id, id: Id) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(st, project).await?;
    routes::require(st, team, user.id, Role::Viewer).await?;
    let (branch, ..) = review_row(st, project, id).await?;
    Ok(Json(dispatch(st, project, team, json!({ "action": "files", "branch": branch }), 60).await?))
}

pub async fn merge(st: &AppState, user: &AuthUser, project: Id, id: Id, squash: bool) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(st, project).await?;
    routes::require(st, team, user.id, Role::Owner).await?;
    let (branch, title, author, status) = review_row(st, project, id).await?;
    if status != "open" {
        return Err(AppError::bad("Este pedido ya no está abierto"));
    }
    let me = name_of(st, user.id).await;
    dispatch(st, project, team, json!({ "action": "merge", "branch": branch, "title": title, "author": author, "who": me, "squash": squash }), 120).await?;
    sqlx::query("UPDATE reviews SET status = 'merged', decided_by = ?, decided_at = ? WHERE id = ?").bind(&me).bind(Utc::now().to_rfc3339()).bind(id).execute(&st.db).await?;
    crate::audit::record(st, team, Some(project), None, user.id, "revision_integrada", &format!("{branch}: {title} (de {author})")).await;
    if let Ok(uid) = sqlx::query_scalar::<_, Id>("SELECT id FROM users WHERE username = ?").bind(&author).fetch_one(&st.db).await {
        crate::notify::notify(st, uid, "github", Some(team), None, format!("✅ Tu trabajo «{title}» fue integrado al proyecto por {me}. Corré «cw actualizar» para traerlo a tu copia.")).await;
    }
    Ok(Json(json!({ "merged": true, "synced": true })))
}

pub async fn close(st: &AppState, user: &AuthUser, project: Id, id: Id, comment: &str) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(st, project).await?;
    routes::require(st, team, user.id, Role::Owner).await?;
    let (branch, title, author, status) = review_row(st, project, id).await?;
    if status != "open" {
        return Err(AppError::bad("Este pedido ya no está abierto"));
    }
    let me = name_of(st, user.id).await;
    let comment: String = comment.trim().chars().take(2000).collect();
    sqlx::query("UPDATE reviews SET status = 'rejected', decided_by = ?, decided_at = ?, note = ? WHERE id = ?").bind(&me).bind(Utc::now().to_rfc3339()).bind(&comment).bind(id).execute(&st.db).await?;
    crate::audit::record(st, team, Some(project), None, user.id, "revision_devuelta", &format!("{branch}: {title} (de {author})")).await;
    if let Ok(uid) = sqlx::query_scalar::<_, Id>("SELECT id FROM users WHERE username = ?").bind(&author).fetch_one(&st.db).await {
        let why = if comment.is_empty() { String::new() } else { format!(" Motivo: {comment}") };
        crate::notify::notify(st, uid, "github", Some(team), None, format!("↩ {me} devolvió tu trabajo «{title}» sin integrarlo.{why}")).await;
    }
    Ok(Json(json!({ "closed": true })))
}

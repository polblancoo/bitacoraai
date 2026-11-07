//! GitHub por proyecto: ajustes con casillas, sincronización (cada 5 minutos o al instante con webhook), estado de los
//! Pull Requests y de las pruebas (CI), comentarios en los dos sentidos, publicar tareas como issues y abrir Pull Requests.
//! Todo lo automático usa la conexión de quien guardó los ajustes; nada se publica fuera de GitHub si la casilla está apagada.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    github::{api_get, api_json, gh, repo_of, token_of},
    routes, AppState,
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};
use tm_core::{Event, Id, Role, Status};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects/{id}/github/settings", get(get_cfg).put(put_cfg))
        .route("/api/projects/{id}/github/now", post(sync_now))
        .route("/api/projects/{id}/open-folder", post(open_folder))
        .route("/api/tasks/{id}/github/publish", post(publish_task))
        .route("/api/tasks/{id}/github/pr", post(open_pr))
        .route("/api/github/webhook/{id}", post(webhook))
        .route("/api/agent/pr", post(agent_pr))
}

pub fn spawn(st: AppState) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
        loop {
            let ids: Vec<Id> = sqlx::query_scalar("SELECT project_id FROM project_github_cfg WHERE auto_sync = 1").fetch_all(&st.db).await.unwrap_or_default();
            for p in ids {
                if let Err(e) = sync_project(&st, p).await {
                    tracing::warn!("github: proyecto {p}: {}", e.1);
                }
            }
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });
}

struct Cfg {
    owner: Id,
    comments: bool,
    publish: bool,
    publish_since: String,
    ai_pr: bool,
    ci: bool,
    activity: bool,
    last_sha: String,
    ci_state: String,
}

async fn load(st: &AppState, project: Id) -> Result<Option<Cfg>> {
    let r = sqlx::query("SELECT * FROM project_github_cfg WHERE project_id = ?").bind(project).fetch_optional(&st.db).await?;
    Ok(r.map(|r| Cfg {
        owner: r.get("owner_user"),
        comments: r.get::<i64, _>("sync_comments") != 0,
        publish: r.get::<i64, _>("publish_issues") != 0,
        publish_since: r.get("publish_since"),
        ai_pr: r.get::<i64, _>("ai_pr") != 0,
        ci: r.get::<i64, _>("ci_status") != 0,
        activity: r.get::<i64, _>("activity") != 0,
        last_sha: r.get("last_sha"),
        ci_state: r.get("ci_state"),
    }))
}

async fn get_cfg(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    let team = routes::team_of_project(&st, project).await?;
    let role = routes::require(&st, team, user.id, Role::Viewer).await?;
    let r = sqlx::query("SELECT * FROM project_github_cfg WHERE project_id = ?").bind(project).fetch_optional(&st.db).await?;
    let admin = !user.bot && role >= Role::Admin;
    let flag = |k: &str| r.as_ref().map(|r| r.get::<i64, _>(k) != 0).unwrap_or(false);
    Ok(Json(json!({
        "auto_sync": flag("auto_sync"), "sync_comments": flag("sync_comments"), "publish_issues": flag("publish_issues"),
        "ai_pr": flag("ai_pr"), "ci_status": flag("ci_status"), "activity": flag("activity"), "webhook": flag("webhook"),
        "ci_state": r.as_ref().map(|r| r.get::<String, _>("ci_state")).unwrap_or_default(),
        "last_sync": r.as_ref().map(|r| r.get::<String, _>("last_sync")).unwrap_or_default(),
        "last_msg": r.as_ref().map(|r| r.get::<String, _>("last_msg")).unwrap_or_default(),
        "webhook_path": format!("/api/github/webhook/{project}"),
        "webhook_secret": if admin && flag("webhook") { r.as_ref().map(|r| r.get::<String, _>("webhook_secret")) } else { None },
    })))
}

#[derive(Deserialize)]
struct CfgBody {
    #[serde(default)]
    auto_sync: bool,
    #[serde(default)]
    sync_comments: bool,
    #[serde(default)]
    publish_issues: bool,
    #[serde(default)]
    ai_pr: bool,
    #[serde(default)]
    ci_status: bool,
    #[serde(default)]
    activity: bool,
    #[serde(default)]
    webhook: bool,
}

async fn put_cfg(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<CfgBody>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(&st, project).await?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    if repo_of(&st, project).await?.is_none() {
        return Err(AppError::bad("Primero indicá el repositorio del proyecto"));
    }
    token_of(&st, user.id).await?;
    let prev = load(&st, project).await?;
    let since = match &prev {
        Some(p) if p.publish && b.publish_issues => p.publish_since.clone(),
        _ if b.publish_issues => Utc::now().to_rfc3339(),
        _ => String::new(),
    };
    let secret: String = sqlx::query_scalar("SELECT webhook_secret FROM project_github_cfg WHERE project_id = ?")
        .bind(project)
        .fetch_optional(&st.db)
        .await?
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| crate::auth::random_hex(20));
    sqlx::query(
        "INSERT INTO project_github_cfg (project_id, owner_user, auto_sync, sync_comments, publish_issues, publish_since, ai_pr, ci_status, activity, webhook, webhook_secret) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(project_id) DO UPDATE SET owner_user = excluded.owner_user, auto_sync = excluded.auto_sync, \
         sync_comments = excluded.sync_comments, publish_issues = excluded.publish_issues, publish_since = excluded.publish_since, ai_pr = excluded.ai_pr, \
         ci_status = excluded.ci_status, activity = excluded.activity, webhook = excluded.webhook, webhook_secret = excluded.webhook_secret",
    )
    .bind(project)
    .bind(user.id)
    .bind(b.auto_sync)
    .bind(b.sync_comments)
    .bind(b.publish_issues)
    .bind(since)
    .bind(b.ai_pr)
    .bind(b.ci_status)
    .bind(b.activity)
    .bind(b.webhook)
    .bind(&secret)
    .execute(&st.db)
    .await?;
    get_cfg(State(st), user, Path(project)).await
}

async fn sync_now(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(&st, project).await?;
    routes::ensure_writer(&st, team, user.id).await?;
    if load(&st, project).await?.is_none() {
        // sin ajustes guardados: se sincroniza solo lo básico con la conexión de quien toca el botón
        sqlx::query("INSERT INTO project_github_cfg (project_id, owner_user) VALUES (?, ?)").bind(project).bind(user.id).execute(&st.db).await?;
    }
    Ok(Json(sync_project(&st, project).await?))
}

/// Abre la carpeta de trabajo del proyecto en el escritorio de la máquina donde corre el servidor (solo si quien lo pide está en esa misma máquina).
async fn open_folder(
    State(st): State<AppState>,
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    user: AuthUser,
    Path(project): Path<Id>,
) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(&st, project).await?;
    routes::ensure_writer(&st, team, user.id).await?;
    // solo si quien pide está sentado frente a la máquina del servidor (entra por localhost): si no, se abriría en la pantalla de otra persona
    let ip = crate::limits::client_ip(&headers, peer, st.trust_proxy);
    if !ip.is_loopback() {
        return Err(AppError::bad("la carpeta está en la máquina del servidor: solo se puede abrir desde esa máquina"));
    }
    let dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await.unwrap_or_default();
    if dir.trim().is_empty() || !std::path::Path::new(&dir).is_dir() {
        return Err(AppError::bad("la carpeta no existe en la máquina del servidor"));
    }
    std::process::Command::new("xdg-open").arg(&dir)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|_| AppError::bad("no se pudo abrir (falta xdg-open en el servidor)"))?;
    Ok(Json(json!({ "ok": true })))
}

/// Un recorrido completo: issues cerrados, Pull Requests, comentarios, tareas por publicar, pruebas y commits.
pub async fn sync_project(st: &AppState, project: Id) -> Result<Value> {
    let Some(cfg) = load(st, project).await? else { return Ok(json!({})) };
    let g = gh(st)?;
    let repo = repo_of(st, project).await?.ok_or_else(|| AppError::bad("sin repositorio"))?;
    let token = token_of(st, cfg.owner).await?;
    let team = routes::team_of_project(st, project).await?;
    let (mut closed, mut prs, mut comments_in, mut comments_out, mut published) = (0, 0, 0, 0, 0);
    let mut notes: Vec<String> = vec![];

    let rows: Vec<(Id, i64, Option<i64>, String, i64, i64, String, String)> = sqlx::query_as(
        "SELECT g.task_id, g.number, g.pr_number, g.pr_state, g.last_gh_comment, g.last_pushed_comment, t.status, t.title \
         FROM task_github g JOIN tasks t ON t.id = g.task_id WHERE t.project_id = ? AND g.repo = ? LIMIT 200",
    )
    .bind(project)
    .bind(&repo)
    .fetch_all(&st.db)
    .await?;

    // Pull Requests recientes (para el estado de cada tarea)
    let pulls = api_get(&g, &token, &format!("/repos/{repo}/pulls?state=all&per_page=30&sort=updated&direction=desc")).await.unwrap_or(Value::Null);
    let pulls: &[Value] = pulls.as_array().map(|a| a.as_slice()).unwrap_or(&[]);

    for (task, number, pr_number, pr_state, last_in, last_out, status, title) in &rows {
        // 1) ¿el issue se cerró en GitHub?
        if status != "done" {
            if let Ok(i) = api_get(&g, &token, &format!("/repos/{repo}/issues/{number}")).await {
                if i["state"].as_str() == Some("closed") {
                    mark_done(st, team, project, cfg.owner, *task, "se cerró en GitHub").await;
                    closed += 1;
                }
            }
        }
        // 2) Pull Request que menciona #número
        if let Some(pr) = pulls.iter().find(|p| mentions(p["title"].as_str().unwrap_or(""), *number) || mentions(p["body"].as_str().unwrap_or(""), *number)) {
            let n = pr["number"].as_i64().unwrap_or(0);
            let state = if !pr["merged_at"].is_null() { "merged" } else if pr["state"].as_str() == Some("closed") { "closed" } else if pr["draft"].as_bool() == Some(true) { "draft" } else { "open" };
            if Some(n) != *pr_number || state != pr_state {
                sqlx::query("UPDATE task_github SET pr_number = ?, pr_state = ? WHERE task_id = ?").bind(n).bind(state).bind(task).execute(&st.db).await?;
                crate::extras::log_task(st, *task, cfg.owner, format!("🐙 Pull Request #{n} de «{title}»: {}", pr_label(state))).await;
                prs += 1;
                if state == "merged" && status != "done" {
                    mark_done(st, team, project, cfg.owner, *task, "su Pull Request se integró").await;
                }
            }
        }
        // 3) comentarios en los dos sentidos
        if cfg.comments {
            if let Ok(list) = api_get(&g, &token, &format!("/repos/{repo}/issues/{number}/comments?per_page=50")).await {
                let mut top = *last_in;
                for c in list.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
                    let id = c["id"].as_i64().unwrap_or(0);
                    if id <= *last_in {
                        continue;
                    }
                    let body = c["body"].as_str().unwrap_or("");
                    if body.starts_with("💬 ") {
                        top = top.max(id);
                        continue; // lo publicó esta app: no se vuelve a traer
                    }
                    let text: String = format!("🐙 @{}: {}", c["user"]["login"].as_str().unwrap_or("?"), body.trim()).chars().take(crate::limits::COMMENT_MAX).collect();
                    sqlx::query("INSERT INTO comments (task_id, user_id, body, created_at) VALUES (?, ?, ?, ?)").bind(task).bind(cfg.owner).bind(text).bind(Utc::now()).execute(&st.db).await?;
                    top = top.max(id);
                    comments_in += 1;
                }
                if top != *last_in {
                    sqlx::query("UPDATE task_github SET last_gh_comment = ? WHERE task_id = ?").bind(top).bind(task).execute(&st.db).await?;
                }
            }
            let out: Vec<(i64, String, String)> = sqlx::query_as(
                "SELECT c.id, c.body, u.username FROM comments c JOIN users u ON u.id = c.user_id WHERE c.task_id = ? AND c.id > ? AND c.body NOT LIKE '🐙%' AND c.body NOT LIKE '/permitir%' ORDER BY c.id LIMIT 10",
            )
            .bind(task)
            .bind(last_out)
            .fetch_all(&st.db)
            .await?;
            let mut top = *last_out;
            for (id, body, who) in out {
                if api_json(&g, &token, reqwest::Method::POST, &format!("/repos/{repo}/issues/{number}/comments"), json!({ "body": format!("💬 {who}: {body}") })).await.is_err() {
                    break;
                }
                top = id;
                comments_out += 1;
            }
            if top != *last_out {
                sqlx::query("UPDATE task_github SET last_pushed_comment = ? WHERE task_id = ?").bind(top).bind(task).execute(&st.db).await?;
            }
        }
    }

    // 4) tareas nuevas → issues (solo las creadas después de prender la casilla)
    if cfg.publish && !cfg.publish_since.is_empty() {
        let new: Vec<(Id, String, String)> = sqlx::query_as(
            "SELECT id, title, description FROM tasks WHERE project_id = ? AND parent_id IS NULL AND created_at > ? AND id NOT IN (SELECT task_id FROM task_github) ORDER BY id LIMIT 5",
        )
        .bind(project)
        .bind(&cfg.publish_since)
        .fetch_all(&st.db)
        .await?;
        for (task, title, desc) in new {
            if publish(st, &g, &token, &repo, task, &title, &desc).await.is_ok() {
                published += 1;
            }
        }
    }

    // 5) pruebas (CI) del último commit de la rama principal
    if cfg.ci {
        if let Some(state) = ci_state(&g, &token, &repo).await {
            if state != cfg.ci_state {
                sqlx::query("UPDATE project_github_cfg SET ci_state = ? WHERE project_id = ?").bind(&state).bind(project).execute(&st.db).await?;
                if state == "failure" {
                    crate::notify::notify(st, cfg.owner, "github", Some(team), None, format!("🐙 Las pruebas de {repo} fallaron en la rama principal")).await;
                }
                notes.push(format!("pruebas: {state}"));
            }
        }
    }

    // 6) commits nuevos a la bitácora
    if cfg.activity {
        if let Ok(list) = api_get(&g, &token, &format!("/repos/{repo}/commits?per_page=15")).await {
            let items: &[Value] = list.as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            if let Some(first) = items.first().and_then(|c| c["sha"].as_str()) {
                if !cfg.last_sha.is_empty() {
                    let fresh: Vec<&Value> = items.iter().take_while(|c| c["sha"].as_str() != Some(cfg.last_sha.as_str())).collect();
                    for c in fresh.iter().rev() {
                        let msg = c["commit"]["message"].as_str().unwrap_or("").lines().next().unwrap_or("");
                        let sha = c["sha"].as_str().unwrap_or("").chars().take(7).collect::<String>();
                        crate::extras::log(st, project, cfg.owner, format!("🐙 commit {sha}: {msg} ({})", c["commit"]["author"]["name"].as_str().unwrap_or("?"))).await;
                    }
                }
                sqlx::query("UPDATE project_github_cfg SET last_sha = ? WHERE project_id = ?").bind(first).bind(project).execute(&st.db).await?;
            }
        }
    }

    let summary = format!("{closed} cerrados, {prs} PR, {comments_in} comentarios traídos, {comments_out} enviados, {published} publicados");
    sqlx::query("UPDATE project_github_cfg SET last_sync = ?, last_msg = ? WHERE project_id = ?").bind(Utc::now().to_rfc3339()).bind(&summary).bind(project).execute(&st.db).await?;
    Ok(json!({ "closed": closed, "prs": prs, "comments_in": comments_in, "comments_out": comments_out, "published": published, "notes": notes, "summary": summary }))
}

fn pr_label(s: &str) -> &'static str {
    match s {
        "merged" => "integrado ✅",
        "closed" => "cerrado sin integrar",
        "draft" => "borrador",
        _ => "abierto",
    }
}

/// ¿El texto menciona `#n` (y no `#n0`)?
fn mentions(text: &str, n: i64) -> bool {
    let key = format!("#{n}");
    text.match_indices(&key).any(|(i, _)| !text[i + key.len()..].starts_with(|c: char| c.is_ascii_digit()))
}

async fn mark_done(st: &AppState, team: Id, project: Id, actor: Id, task: Id, why: &str) {
    if sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?").bind(Status::Done.as_str()).bind(Utc::now()).bind(task).execute(&st.db).await.is_err() {
        return;
    }
    if let Ok(t) = routes::fetch_task(st, task).await {
        crate::extras::log(st, project, actor, format!("«{}» {why}: pasó a Hecha", t.title)).await;
        st.emit(team, Event::TaskUpdated(t));
    }
}

async fn ci_state(g: &crate::github::Github, token: &str, repo: &str) -> Option<String> {
    let info = api_get(g, token, &format!("/repos/{repo}")).await.ok()?;
    let branch = info["default_branch"].as_str()?.to_string();
    let runs = api_get(g, token, &format!("/repos/{repo}/commits/{branch}/check-runs?per_page=50")).await.ok()?;
    let list: &[Value] = runs["check_runs"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
    if !list.is_empty() {
        if list.iter().any(|r| matches!(r["conclusion"].as_str(), Some("failure" | "timed_out" | "cancelled" | "action_required"))) {
            return Some("failure".into());
        }
        if list.iter().any(|r| r["status"].as_str() != Some("completed")) {
            return Some("pending".into());
        }
        return Some("success".into());
    }
    let s = api_get(g, token, &format!("/repos/{repo}/commits/{branch}/status")).await.ok()?;
    match s["state"].as_str() {
        Some("success") if s["total_count"].as_i64().unwrap_or(0) > 0 => Some("success".into()),
        Some("failure" | "error") => Some("failure".into()),
        Some("pending") if s["total_count"].as_i64().unwrap_or(0) > 0 => Some("pending".into()),
        _ => Some("none".into()),
    }
}

/// Crea el issue de una tarea y la deja enlazada (las tareas que ya vienen de un issue no se publican de nuevo).
async fn publish(st: &AppState, g: &crate::github::Github, token: &str, repo: &str, task: Id, title: &str, desc: &str) -> Result<Value> {
    let body: String = format!("{}\n\n— publicado desde el gestor de tareas", desc.trim()).chars().take(8000).collect();
    let i = api_json(g, token, reqwest::Method::POST, &format!("/repos/{repo}/issues"), json!({ "title": title, "body": body })).await?;
    let (n, url) = (i["number"].as_i64().unwrap_or(0), i["html_url"].as_str().unwrap_or("").to_string());
    let last: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM comments WHERE task_id = ?").bind(task).fetch_one(&st.db).await?;
    sqlx::query("INSERT INTO task_github (task_id, repo, number, url, last_pushed_comment) VALUES (?, ?, ?, ?, ?)").bind(task).bind(repo).bind(n).bind(&url).bind(last).execute(&st.db).await?;
    Ok(json!({ "number": n, "url": url }))
}

async fn publish_task(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_task(&st, task).await?;
    routes::ensure_writer(&st, team, user.id).await?;
    let t = routes::fetch_task(&st, task).await?;
    if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM task_github WHERE task_id = ?").bind(task).fetch_one(&st.db).await? > 0 {
        return Err(AppError::bad("Esta tarea ya está enlazada a un issue"));
    }
    let g = gh(&st)?;
    let repo = repo_of(&st, t.project_id).await?.ok_or_else(|| AppError::bad("El proyecto no tiene repositorio de GitHub"))?;
    let token = token_of(&st, user.id).await?;
    Ok(Json(publish(&st, &g, &token, &repo, task, &t.title, &t.description).await?))
}

#[derive(Deserialize, Default)]
struct PrBody {
    #[serde(default)]
    branch: String,
}

/// Abre un Pull Request con la rama indicada (por defecto `tm/tarea-ID`). Siempre lo dispara una persona.
async fn open_pr(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>, body: Option<Json<PrBody>>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_task(&st, task).await?;
    routes::ensure_writer(&st, team, user.id).await?;
    let t = routes::fetch_task(&st, task).await?;
    let g = gh(&st)?;
    let repo = repo_of(&st, t.project_id).await?.ok_or_else(|| AppError::bad("El proyecto no tiene repositorio de GitHub"))?;
    let token = token_of(&st, user.id).await?;
    let branch = body.map(|b| b.0.branch).unwrap_or_default();
    let branch = if branch.trim().is_empty() { format!("tm/tarea-{task}") } else { branch.trim().to_string() };
    if branch.len() > 120 || !branch.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.')) {
        return Err(AppError::bad("Nombre de rama no válido"));
    }
    // la rama tiene que estar subida a GitHub (la IA o vos con `git push`)
    if let Err(e) = api_get(&g, &token, &format!("/repos/{repo}/branches/{}", branch.replace('/', "%2F"))).await {
        if !e.1.contains("(404)") {
            return Err(e);
        }
        return Err(AppError::bad(&format!("La rama «{branch}» no está en GitHub: hay que subirla con `git push origin {branch}` antes de abrir el Pull Request")));
    }
    let info = api_get(&g, &token, &format!("/repos/{repo}")).await?;
    let base = info["default_branch"].as_str().unwrap_or("main").to_string();
    let linked: Option<i64> = sqlx::query_scalar("SELECT number FROM task_github WHERE task_id = ?").bind(task).fetch_optional(&st.db).await?;
    let text = match linked {
        Some(n) => format!("Cierra #{n}\n\n{}", t.description.trim()),
        None => t.description.trim().to_string(),
    };
    let pr = api_json(&g, &token, reqwest::Method::POST, &format!("/repos/{repo}/pulls"), json!({ "title": t.title, "head": branch, "base": base, "body": text.chars().take(8000).collect::<String>() })).await?;
    let (n, url) = (pr["number"].as_i64().unwrap_or(0), pr["html_url"].as_str().unwrap_or("").to_string());
    sqlx::query("UPDATE task_github SET pr_number = ?, pr_state = 'open' WHERE task_id = ?").bind(n).bind(task).execute(&st.db).await?;
    crate::extras::log_task(&st, task, user.id, format!("🐙 abrió el Pull Request #{n} desde la rama {branch}")).await;
    Ok(Json(json!({ "number": n, "url": url })))
}

#[derive(Deserialize)]
struct AgentPr {
    project: Id,
    who: String,
    branch: String,
    title: String,
    #[serde(default)]
    body: String,
}

/// El agente abre un Pull Request en nombre de una persona del equipo (por ejemplo desde su terminal: `cw enviar`).
/// La persona no necesita cuenta de GitHub: se usa la conexión de quien configuró GitHub en el proyecto y el Pull Request dice quién lo pidió.
async fn agent_pr(State(st): State<AppState>, user: AuthUser, Json(b): Json<AgentPr>) -> Result<Json<Value>> {
    if !user.bot {
        return Err(AppError::forbidden());
    }
    let uid: Id = sqlx::query_scalar("SELECT id FROM users WHERE username = ?").bind(&b.who).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)?;
    let team = routes::team_of_project(&st, b.project).await?;
    routes::ensure_writer(&st, team, uid).await?;
    let branch = b.branch.trim().to_string();
    if branch.is_empty() || branch.len() > 120 || !branch.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.')) || branch.contains("..") {
        return Err(AppError::bad("Nombre de rama no válido"));
    }
    let g = gh(&st)?;
    let repo = repo_of(&st, b.project).await?.ok_or_else(|| AppError::bad("El proyecto no tiene repositorio de GitHub"))?;
    let cfg = load(&st, b.project).await?.ok_or_else(|| AppError::bad("El proyecto no tiene GitHub conectado en CrewHelm"))?;
    let token = token_of(&st, cfg.owner).await?;
    let info = api_get(&g, &token, &format!("/repos/{repo}")).await?;
    let base = info["default_branch"].as_str().unwrap_or("main").to_string();
    let title: String = b.title.trim().chars().take(200).collect();
    let body = format!("{}\n\n— Pedido desde CrewHelm por {}", b.body.trim().chars().take(7000).collect::<String>(), b.who);
    let res = api_json(&g, &token, reqwest::Method::POST, &format!("/repos/{repo}/pulls"), json!({ "title": if title.is_empty() { branch.clone() } else { title }, "head": branch, "base": base, "body": body })).await;
    let pr = match res {
        Ok(pr) => pr,
        Err(e) => {
            // ya había uno abierto para esta rama: se devuelve ese
            let owner = repo.split('/').next().unwrap_or_default();
            let found = api_get(&g, &token, &format!("/repos/{repo}/pulls?state=open&head={owner}:{}", branch.replace('/', "%2F"))).await.ok();
            match found.and_then(|v| v.as_array().and_then(|a| a.first().cloned())) {
                Some(pr) => pr,
                None => return Err(e),
            }
        }
    };
    let (n, url) = (pr["number"].as_i64().unwrap_or(0), pr["html_url"].as_str().unwrap_or("").to_string());
    tracing::info!("github: {} pidió el Pull Request #{n} ({branch}) del proyecto {}", b.who, b.project);
    // aviso a los owners para que lo revisen (botón «🔀 Integrar» del proyecto)
    let owners: Vec<Id> = sqlx::query_scalar("SELECT m.user_id FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND m.role = 'owner' AND u.is_bot = 0 AND m.user_id != ?").bind(team).bind(uid).fetch_all(&st.db).await.unwrap_or_default();
    for o in owners {
        crate::notify::notify(&st, o, "github", Some(team), None, format!("🔀 {} mandó «{}» a revisión. Abrí el proyecto y tocá «🔀 Integrar» para revisarlo.", b.who, if b.title.trim().is_empty() { branch.clone() } else { b.title.trim().chars().take(80).collect() })).await;
    }
    Ok(Json(json!({ "number": n, "url": url, "base": base })))
}

/// Cuando una IA termina una tarea de un proyecto con «Pull Request» activado, se avisa a quien configuró GitHub.
pub async fn after_update(st: &AppState, user: &AuthUser, before: &tm_core::Task, after: &tm_core::Task) {
    if !user.bot || !(after.status == Status::Done || after.waiting) || (before.status == Status::Done || before.waiting) {
        return;
    }
    let Ok(Some(cfg)) = load(st, after.project_id).await else { return };
    if !cfg.ai_pr {
        return;
    }
    let team = routes::team_of_project(st, after.project_id).await.ok();
    crate::notify::notify(st, cfg.owner, "github", team, Some(after.id), format!("🐙 La IA terminó «{}». Si dejó su trabajo en la rama tm/tarea-{}, abrí el Pull Request desde la tarea (botón «Abrir Pull Request»).", after.title, after.id)).await;
}

// ---- webhook: GitHub avisa al instante (necesita que el servidor sea alcanzable desde internet) ----

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let inner = Sha256::new().chain_update(k.map(|b| b ^ 0x36)).chain_update(msg).finalize();
    Sha256::new().chain_update(k.map(|b| b ^ 0x5c)).chain_update(inner).finalize().to_vec()
}

fn last_hook() -> &'static Mutex<HashMap<Id, Instant>> {
    static M: std::sync::OnceLock<Mutex<HashMap<Id, Instant>>> = std::sync::OnceLock::new();
    M.get_or_init(Default::default)
}

async fn webhook(State(st): State<AppState>, Path(project): Path<Id>, headers: HeaderMap, body: Bytes) -> Result<Json<Value>> {
    let secret: Option<String> = sqlx::query_scalar("SELECT webhook_secret FROM project_github_cfg WHERE project_id = ? AND webhook = 1").bind(project).fetch_optional(&st.db).await?;
    let Some(secret) = secret.filter(|s| !s.is_empty()) else { return Err(AppError::not_found()) };
    let sig = headers.get("x-hub-signature-256").and_then(|v| v.to_str().ok()).unwrap_or("").strip_prefix("sha256=").unwrap_or("").to_string();
    let want: String = hmac_sha256(secret.as_bytes(), &body).iter().map(|b| format!("{b:02x}")).collect();
    // comparación en tiempo constante
    let ok = sig.len() == want.len() && sig.bytes().zip(want.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0;
    if !ok {
        return Err(AppError::unauthorized("firma inválida"));
    }
    if headers.get("x-github-event").and_then(|v| v.to_str().ok()) == Some("ping") {
        return Ok(Json(json!({ "pong": true })));
    }
    let go = {
        let mut m = last_hook().lock().map_err(|_| AppError::bad("estado"))?;
        let go = m.get(&project).map(|t| t.elapsed() > Duration::from_secs(10)).unwrap_or(true);
        if go {
            m.insert(project, Instant::now());
        }
        go
    };
    if go {
        let st2 = st.clone();
        tokio::spawn(async move {
            let _ = sync_project(&st2, project).await;
        });
    }
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menciona_numero_exacto() {
        assert!(mentions("Cierra #12", 12));
        assert!(!mentions("Cierra #123", 12));
        assert!(mentions("fix #1, #12 y más", 12));
        assert!(!mentions("sin número", 12));
    }

    #[test]
    fn hmac_conocido() {
        // RFC 4231, caso 2
        let h: String = hmac_sha256(b"Jefe", b"what do ya want for nothing?").iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(h, "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }
}

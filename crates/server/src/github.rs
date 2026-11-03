//! Conexión opcional con GitHub por Device Flow: cada persona vincula su propia cuenta (con sus permisos) sin que el
//! servidor tenga que ser accesible desde internet ni guarde un secreto de la aplicación.
//! Se activa con `TM_GITHUB_CLIENT_ID` (el de una OAuth App con «Enable Device Flow»). `TM_GITHUB_SCOPE` cambia los permisos
//! pedidos (por defecto `read:user repo`); `TM_GITHUB_BASE` y `TM_GITHUB_API` permiten apuntar a otra instancia (GitHub Enterprise, pruebas).
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Mutex, time::{Duration, Instant}};
use tm_core::Id;

pub struct Github {
    client_id: String,
    scope: String,
    base: String,
    pub(crate) api: String,
    pub(crate) http: reqwest::Client,
}

struct Pending {
    device_code: String,
    expires: Instant,
}

fn pending() -> &'static Mutex<HashMap<Id, Pending>> {
    static M: std::sync::OnceLock<Mutex<HashMap<Id, Pending>>> = std::sync::OnceLock::new();
    M.get_or_init(Default::default)
}

impl Github {
    pub fn from_env() -> Option<Self> {
        let var = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        Some(Self {
            client_id: var("TM_GITHUB_CLIENT_ID")?,
            scope: var("TM_GITHUB_SCOPE").unwrap_or_else(|| "read:user repo".into()),
            base: var("TM_GITHUB_BASE").unwrap_or_else(|| "https://github.com".into()).trim_end_matches('/').into(),
            api: var("TM_GITHUB_API").unwrap_or_else(|| "https://api.github.com".into()).trim_end_matches('/').into(),
            http: reqwest::Client::builder().timeout(Duration::from_secs(20)).user_agent("rust-tui-manager").build().ok()?,
        })
    }

    async fn post_form(&self, path: &str, form: &[(&str, &str)]) -> std::result::Result<Value, String> {
        let r = self.http.post(format!("{}{path}", self.base)).header("Accept", "application/json").form(form).send().await.map_err(|e| e.without_url().to_string())?;
        r.json().await.map_err(|e| e.without_url().to_string())
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/github", get(status).delete(unlink))
        .route("/api/github/link", post(link))
        .route("/api/github/poll", post(poll))
        .route("/api/projects/{id}/github", get(project_repo).put(set_project_repo))
        .route("/api/projects/{id}/github/issues", get(list_issues))
        .route("/api/projects/{id}/github/import", post(import_issues))
        .route("/api/projects/{id}/github/sync", post(sync_issues))
        .route("/api/tasks/{id}/github", get(task_link))
        .route("/api/tasks/{id}/github/close", post(close_issue))
}

pub(crate) fn gh(st: &AppState) -> Result<std::sync::Arc<Github>> {
    st.github.clone().ok_or_else(|| AppError::bad("GitHub no está configurado en este servidor"))
}

async fn login_of(st: &AppState, user: Id) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT login FROM github_links WHERE user_id = ?").bind(user).fetch_optional(&st.db).await?)
}

async fn status(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let login = login_of(&st, user.id).await?;
    Ok(Json(json!({ "available": st.github.is_some(), "linked": login.is_some(), "login": login })))
}

/// Pide el código que la persona escribe en github.com/login/device.
async fn link(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let g = gh(&st)?;
    let v = g.post_form("/login/device/code", &[("client_id", &g.client_id), ("scope", &g.scope)]).await.map_err(|e| {
        tracing::warn!("github: pedido de código: {e}");
        AppError::bad("No pude hablar con GitHub")
    })?;
    let (Some(device), Some(code), Some(uri)) = (v["device_code"].as_str(), v["user_code"].as_str(), v["verification_uri"].as_str()) else {
        let why = v["error"].as_str().unwrap_or("respuesta inesperada");
        return Err(if why == "device_flow_disabled" {
            AppError::bad("En la OAuth App de GitHub hay que tildar «Enable Device Flow»")
        } else {
            AppError(axum::http::StatusCode::BAD_REQUEST, format!("GitHub rechazó el pedido ({why})"))
        });
    };
    let expires_in = v["expires_in"].as_u64().unwrap_or(900);
    if let Ok(mut m) = pending().lock() {
        m.insert(user.id, Pending { device_code: device.into(), expires: Instant::now() + Duration::from_secs(expires_in) });
    }
    Ok(Json(json!({ "user_code": code, "verification_uri": uri, "expires_in": expires_in, "interval": v["interval"].as_u64().unwrap_or(5) })))
}

/// La web consulta cada pocos segundos hasta que la persona autoriza en GitHub.
async fn poll(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let g = gh(&st)?;
    let device = {
        let mut m = pending().lock().map_err(|_| AppError::bad("estado"))?;
        match m.get(&user.id) {
            Some(p) if p.expires > Instant::now() => p.device_code.clone(),
            Some(_) => {
                m.remove(&user.id);
                return Ok(Json(json!({ "status": "expired" })));
            }
            None => return Ok(Json(json!({ "status": "none" }))),
        }
    };
    let v = g
        .post_form("/login/oauth/access_token", &[("client_id", &g.client_id), ("device_code", &device), ("grant_type", "urn:ietf:params:oauth:grant-type:device_code")])
        .await
        .map_err(|_| AppError::bad("No pude hablar con GitHub"))?;
    let Some(token) = v["access_token"].as_str() else {
        let status = match v["error"].as_str().unwrap_or("") {
            "authorization_pending" => "pending",
            "slow_down" => "slow_down",
            "expired_token" => "expired",
            "access_denied" => "denied",
            _ => "error",
        };
        if matches!(status, "expired" | "denied") {
            if let Ok(mut m) = pending().lock() {
                m.remove(&user.id);
            }
        }
        return Ok(Json(json!({ "status": status })));
    };
    let me: Value = g
        .http
        .get(format!("{}/user", g.api))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| AppError::bad("No pude leer tu cuenta de GitHub"))?
        .json()
        .await
        .unwrap_or_default();
    let Some(login) = me["login"].as_str() else { return Err(AppError::bad("GitHub no devolvió tu usuario")) };
    sqlx::query(
        "INSERT INTO github_links (user_id, login, token, scope, linked_at) VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(user_id) DO UPDATE SET login = excluded.login, token = excluded.token, scope = excluded.scope, linked_at = excluded.linked_at",
    )
    .bind(user.id)
    .bind(login)
    .bind(st.cred.seal(token))
    .bind(v["scope"].as_str().unwrap_or(""))
    .bind(Utc::now())
    .execute(&st.db)
    .await?;
    if let Ok(mut m) = pending().lock() {
        m.remove(&user.id);
    }
    Ok(Json(json!({ "status": "linked", "login": login })))
}

async fn unlink(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    sqlx::query("DELETE FROM github_links WHERE user_id = ?").bind(user.id).execute(&st.db).await?;
    if let Ok(mut m) = pending().lock() {
        m.remove(&user.id);
    }
    Ok(Json(json!({ "ok": true })))
}

// ---- proyectos ↔ repositorios: importar issues como tareas y cerrarlos desde la tarea ----

const IMPORT_MAX: usize = 30;

pub(crate) fn valid_repo(r: &str) -> bool {
    let mut it = r.split('/');
    let ok = |p: &str| !p.is_empty() && p.len() <= 100 && p.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    matches!((it.next(), it.next(), it.next()), (Some(a), Some(b), None) if ok(a) && ok(b))
}

pub(crate) async fn token_of(st: &AppState, user: Id) -> Result<String> {
    let stored: String = sqlx::query_scalar("SELECT token FROM github_links WHERE user_id = ?")
        .bind(user)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| AppError::bad("Primero conectá tu cuenta de GitHub en Configuración → Avisos y conexiones"))?;
    st.cred.open(&stored).ok_or_else(|| AppError::bad("No pude leer tu conexión con GitHub: desconectá y volvé a conectar tu cuenta"))
}

pub(crate) async fn repo_of(st: &AppState, project: Id) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT repo FROM project_github WHERE project_id = ?").bind(project).fetch_optional(&st.db).await?)
}

async fn gh_err(r: reqwest::Response) -> AppError {
    let status = r.status();
    let msg = r.json::<Value>().await.ok().and_then(|v| v["message"].as_str().map(|m| m.chars().take(160).collect::<String>())).unwrap_or_default();
    let why = match status.as_u16() {
        401 => "tu conexión con GitHub venció: desconectá y volvé a conectar tu cuenta",
        403 => "GitHub no te deja hacer eso (permisos o límite de uso)",
        404 => "no encuentro ese repositorio o issue (revisá que esté bien escrito y que tu cuenta tenga acceso)",
        410 => "ese repositorio tiene los issues desactivados",
        _ => "respuesta inesperada",
    };
    AppError::bad(&format!("GitHub ({}): {why}. {msg}", status.as_u16()))
}

pub(crate) async fn api_get(g: &Github, token: &str, path: &str) -> Result<Value> {
    let r = g
        .http
        .get(format!("{}{path}", g.api))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| AppError::bad("No pude hablar con GitHub"))?;
    if !r.status().is_success() {
        return Err(gh_err(r).await);
    }
    r.json().await.map_err(|_| AppError::bad("Respuesta rara de GitHub"))
}

/// POST/PATCH que devuelve el JSON de GitHub (crear issue o Pull Request).
pub(crate) async fn api_json(g: &Github, token: &str, method: reqwest::Method, path: &str, body: Value) -> Result<Value> {
    let r = g
        .http
        .request(method, format!("{}{path}", g.api))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::bad("No pude hablar con GitHub"))?;
    if !r.status().is_success() {
        return Err(gh_err(r).await);
    }
    r.json().await.map_err(|_| AppError::bad("Respuesta rara de GitHub"))
}

pub(crate) async fn api_send(g: &Github, token: &str, method: reqwest::Method, path: &str, body: Value) -> Result<()> {
    let r = g
        .http
        .request(method, format!("{}{path}", g.api))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::bad("No pude hablar con GitHub"))?;
    if r.status().is_success() { Ok(()) } else { Err(gh_err(r).await) }
}

async fn project_repo(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    let team = crate::routes::team_of_project(&st, project).await?;
    crate::routes::require(&st, team, user.id, tm_core::Role::Viewer).await?;
    let linked = !user.bot && sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM github_links WHERE user_id = ?").bind(user.id).fetch_one(&st.db).await? > 0;
    Ok(Json(json!({ "available": st.github.is_some(), "linked": linked, "repo": repo_of(&st, project).await? })))
}

#[derive(serde::Deserialize)]
struct RepoBody {
    repo: String,
}

async fn set_project_repo(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<RepoBody>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = crate::routes::team_of_project(&st, project).await?;
    crate::routes::ensure_writer(&st, team, user.id).await?;
    let repo = b.repo.trim().trim_start_matches("https://github.com/").trim_end_matches(".git").trim_matches('/').to_string();
    if repo.is_empty() {
        sqlx::query("DELETE FROM project_github WHERE project_id = ?").bind(project).execute(&st.db).await?;
        return Ok(Json(json!({ "repo": null })));
    }
    if !valid_repo(&repo) {
        return Err(AppError::bad("Escribilo como dueño/repositorio, por ejemplo polblancoo/crewhelm"));
    }
    sqlx::query("INSERT INTO project_github (project_id, repo) VALUES (?, ?) ON CONFLICT(project_id) DO UPDATE SET repo = excluded.repo")
        .bind(project)
        .bind(&repo)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "repo": repo })))
}

/// Issues abiertos del repositorio del proyecto, con una marca en los que ya son tarea.
async fn list_issues(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = crate::routes::team_of_project(&st, project).await?;
    crate::routes::ensure_writer(&st, team, user.id).await?;
    let g = gh(&st)?;
    let repo = repo_of(&st, project).await?.ok_or_else(|| AppError::bad("Este proyecto no tiene repositorio: indicá el dueño/repo primero"))?;
    let token = token_of(&st, user.id).await?;
    let v = api_get(&g, &token, &format!("/repos/{repo}/issues?state=open&per_page=50")).await?;
    let done: Vec<i64> = sqlx::query_scalar("SELECT number FROM task_github WHERE repo = ? AND task_id IN (SELECT id FROM tasks WHERE project_id = ?)")
        .bind(&repo)
        .bind(project)
        .fetch_all(&st.db)
        .await?;
    let issues: Vec<Value> = v
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter(|i| i.get("pull_request").is_none())
        .map(|i| {
            let n = i["number"].as_i64().unwrap_or(0);
            json!({
                "number": n,
                "title": i["title"],
                "url": i["html_url"],
                "labels": i["labels"].as_array().map(|l| l.iter().filter_map(|x| x["name"].as_str()).collect::<Vec<_>>()).unwrap_or_default(),
                "imported": done.contains(&n),
            })
        })
        .collect();
    Ok(Json(json!({ "repo": repo, "issues": issues })))
}

#[derive(serde::Deserialize)]
struct ImportBody {
    numbers: Vec<i64>,
}

async fn import_issues(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<ImportBody>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = crate::routes::team_of_project(&st, project).await?;
    crate::routes::ensure_writer(&st, team, user.id).await?;
    if b.numbers.is_empty() || b.numbers.len() > IMPORT_MAX {
        return Err(AppError::bad("Elegí entre 1 y 30 issues por vez"));
    }
    let g = gh(&st)?;
    let repo = repo_of(&st, project).await?.ok_or_else(|| AppError::bad("Este proyecto no tiene repositorio"))?;
    let token = token_of(&st, user.id).await?;
    let mut created = 0;
    for n in b.numbers {
        let taken: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM task_github WHERE repo = ? AND number = ? AND task_id IN (SELECT id FROM tasks WHERE project_id = ?)")
            .bind(&repo)
            .bind(n)
            .bind(project)
            .fetch_one(&st.db)
            .await?;
        if taken > 0 {
            continue;
        }
        let i = api_get(&g, &token, &format!("/repos/{repo}/issues/{n}")).await?;
        let url = i["html_url"].as_str().unwrap_or("").to_string();
        let title: String = format!("{} (#{n})", i["title"].as_str().unwrap_or("Issue")).chars().take(crate::limits::TITLE_MAX).collect();
        let body: String = format!("{}\n\n🔗 {url}", i["body"].as_str().unwrap_or("").trim()).chars().take(crate::limits::DESCRIPTION_MAX).collect();
        let t = crate::routes::create_task(
            State(st.clone()),
            AuthUser { id: user.id, bot: false },
            Path(project),
            Json(tm_core::NewTask { title, description: body, priority: 0, assignee_id: None, due: None, parent_id: None, start: None, recurrence: None }),
        )
        .await?
        .0;
        sqlx::query("INSERT INTO task_github (task_id, repo, number, url) VALUES (?, ?, ?, ?)").bind(t.id).bind(&repo).bind(n).bind(&url).execute(&st.db).await?;
        created += 1;
    }
    Ok(Json(json!({ "created": created })))
}

/// Pasa a «Hecha» las tareas cuyo issue ya se cerró en GitHub.
async fn sync_issues(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = crate::routes::team_of_project(&st, project).await?;
    crate::routes::ensure_writer(&st, team, user.id).await?;
    let g = gh(&st)?;
    let token = token_of(&st, user.id).await?;
    let rows: Vec<(Id, String, i64)> = sqlx::query_as(
        "SELECT g.task_id, g.repo, g.number FROM task_github g JOIN tasks t ON t.id = g.task_id WHERE t.project_id = ? AND t.status != 'done' LIMIT 100",
    )
    .bind(project)
    .fetch_all(&st.db)
    .await?;
    let mut closed = 0;
    for (task, repo, n) in rows {
        let Ok(i) = api_get(&g, &token, &format!("/repos/{repo}/issues/{n}")).await else { continue };
        if i["state"].as_str() == Some("closed") {
            sqlx::query("UPDATE tasks SET status = 'done', updated_at = ? WHERE id = ?").bind(Utc::now()).bind(task).execute(&st.db).await?;
            if let Ok(t) = crate::routes::fetch_task(&st, task).await {
                crate::extras::log(&st, project, user.id, format!("«{}» se cerró en GitHub: pasó a Hecha", t.title)).await;
                st.emit(team, tm_core::Event::TaskUpdated(t));
            }
            closed += 1;
        }
    }
    Ok(Json(json!({ "closed": closed })))
}

async fn task_link(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    let team = crate::routes::team_of_task(&st, task).await?;
    crate::routes::require(&st, team, user.id, tm_core::Role::Viewer).await?;
    let project: Id = sqlx::query_scalar("SELECT project_id FROM tasks WHERE id = ?").bind(task).fetch_one(&st.db).await?;
    let project_repo = repo_of(&st, project).await?;
    let row: Option<(String, i64, String, Option<i64>, String)> = sqlx::query_as("SELECT repo, number, url, pr_number, pr_state FROM task_github WHERE task_id = ?").bind(task).fetch_optional(&st.db).await?;
    Ok(Json(match row {
        Some((repo, number, url, pr, pr_state)) => json!({ "repo": repo, "number": number, "url": url, "pr": pr, "pr_state": pr_state, "project_repo": project_repo }),
        None => json!({ "repo": null, "project_repo": project_repo }),
    }))
}

/// Cierra el issue en GitHub (con un comentario) cuando la tarea está terminada.
async fn close_issue(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = crate::routes::team_of_task(&st, task).await?;
    crate::routes::ensure_writer(&st, team, user.id).await?;
    let g = gh(&st)?;
    let token = token_of(&st, user.id).await?;
    let (repo, n): (String, i64) = sqlx::query_as("SELECT repo, number FROM task_github WHERE task_id = ?")
        .bind(task)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| AppError::bad("Esta tarea no viene de un issue de GitHub"))?;
    let _ = api_send(&g, &token, reqwest::Method::POST, &format!("/repos/{repo}/issues/{n}/comments"), json!({ "body": "Tarea terminada en el gestor de tareas." })).await;
    api_send(&g, &token, reqwest::Method::PATCH, &format!("/repos/{repo}/issues/{n}"), json!({ "state": "closed" })).await?;
    Ok(Json(json!({ "ok": true })))
}

//! Asistente de git del proyecto («⚙ Conexión → Git y GitHub»): traer un repositorio de GitHub, crear uno nuevo o dejar el proyecto
//! solo en la máquina. La carpeta vive en la máquina del agente, así que el servidor le pasa el pedido (con la conexión de GitHub de
//! quien lo pide, solo para esa operación) y espera el resultado. La subida usa una clave de despliegue propia del proyecto.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    github::{gh, token_of, valid_repo},
    routes, term, AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::{LazyLock, Mutex}, time::Duration};
use tm_core::{Id, Role};
use tokio::sync::oneshot;

struct Job {
    agent: Id,
    payload: Value,
    taken: bool,
    tx: Option<oneshot::Sender<Value>>,
}
static JOBS: LazyLock<Mutex<HashMap<String, Job>>> = LazyLock::new(Default::default);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/lan", get(lan))
        .route("/api/projects/{id}/git/setup", post(setup))
        .route("/api/projects/{id}/git/state", get(state))
        .route("/api/projects/{id}/git/request", post(request_repo))
        .route("/api/agent/git/pending", get(agent_pending))
        .route("/api/agent/git/{id}/result", post(agent_result))
}

#[derive(Deserialize)]
struct Body {
    mode: String,
    #[serde(default)]
    repo: String,
    #[serde(default)]
    name: String,
    #[serde(default = "yes")]
    private: bool,
}
fn yes() -> bool {
    true
}

fn valid_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= 80 && !n.starts_with('.') && n.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Pide al agente de la máquina del proyecto que mire o prepare su repositorio.
/// `mode`: status | local | clone | new | attach (clone/new/attach necesitan tu cuenta de GitHub conectada).
async fn setup(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<Body>) -> Result<Json<Value>> {
    user.human_only()?;
    let team: Id = sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)?;
    routes::require(&st, team, user.id, Role::Owner).await?;
    st.user_limiter.hit(&format!("gitsetup:{}", user.id), 12, Duration::from_secs(60)).map_err(|_| AppError::bad("demasiados intentos: esperá un minuto"))?;
    let (agent, dir) = target(&st, project, team).await?;
    let mut payload = json!({ "mode": b.mode, "dir": dir });
    match b.mode.as_str() {
        "status" | "local" => {}
        "clone" | "attach" | "new" => {
            let g = gh(&st)?;
            let token = token_of(&st, user.id).await?;
            let repo = crate::github::clean_repo(&b.repo);
            if b.mode != "new" && !valid_repo(&repo) {
                return Err(AppError::bad("Escribilo como dueño/repositorio, por ejemplo polblancoo/crewhelm"));
            }
            if b.mode == "new" && !valid_name(b.name.trim()) {
                return Err(AppError::bad("El nombre del repositorio puede tener letras, números, guiones y puntos"));
            }
            payload["repo"] = json!(repo);
            payload["name"] = json!(b.name.trim());
            payload["private"] = json!(b.private);
            payload["token"] = json!(token);
            payload["api"] = json!(g.api);
            payload["base"] = json!(g.base);
        }
        _ => return Err(AppError::bad("Opción no válida")),
    }
    let data = dispatch(agent, payload, 170).await?;
    // el repositorio queda vinculado al proyecto (issues, Pull Requests, sincronización)
    if matches!(b.mode.as_str(), "clone" | "attach" | "new") {
        if let Some(repo) = data["repo"].as_str().filter(|r| valid_repo(r)) {
            sqlx::query("INSERT INTO project_github (project_id, repo) VALUES (?, ?) ON CONFLICT(project_id) DO UPDATE SET repo = excluded.repo").bind(project).bind(repo).execute(&st.db).await?;
            sqlx::query("UPDATE projects SET repo = ? WHERE id = ? AND (repo IS NULL OR repo = '')").bind(repo).bind(project).execute(&st.db).await?;
        }
    }
    STATE_CACHE.lock().unwrap().remove(&project);
    tracing::info!("git: usuario {} ejecutó «{}» en el proyecto {project}", user.id, b.mode);
    Ok(Json(data))
}

/// Qué agente atiende la carpeta de este proyecto y cuál es la carpeta. Si el proyecto no eligió agente, se usa el único que esté conectado.
pub(crate) async fn target(st: &AppState, project: Id, team: Id) -> Result<(Id, String)> {
    let chosen: Option<Id> = sqlx::query_scalar::<_, Option<Id>>("SELECT agent_id FROM ssh_targets WHERE project_id = ?").bind(project).fetch_optional(&st.db).await?.flatten();
    let agent = match chosen {
        Some(a) => a,
        None => {
            let bots: Vec<Id> = sqlx::query_scalar("SELECT u.id FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND u.is_bot = 1").bind(team).fetch_all(&st.db).await?;
            let online: Vec<Id> = bots.into_iter().filter(|b| term::agent_online(*b)).collect();
            match online.as_slice() {
                [one] => *one,
                [] => return Err(AppError::bad("No hay ningún agente conectado para la máquina del proyecto")),
                _ => return Err(AppError::bad("Hay varios agentes conectados: elegí el de este proyecto en «⚙ Conexión» (botón «▶ Conectar»)")),
            }
        }
    };
    if !term::agent_online(agent) {
        return Err(AppError::bad("El agente de la máquina del proyecto no está conectado ahora"));
    }
    let dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await.unwrap_or_default();
    if dir.trim().is_empty() {
        return Err(AppError::bad("El proyecto no tiene carpeta de trabajo: definila en los ajustes del proyecto"));
    }
    Ok((agent, dir))
}

/// Manda el pedido al agente y espera su respuesta.
pub(crate) async fn dispatch(agent: Id, payload: Value, secs: u64) -> Result<Value> {
    let (tx, rx) = oneshot::channel();
    let id = uuid::Uuid::new_v4().simple().to_string();
    JOBS.lock().unwrap().insert(id.clone(), Job { agent, payload, taken: false, tx: Some(tx) });
    let res = tokio::time::timeout(Duration::from_secs(secs), rx).await;
    JOBS.lock().unwrap().remove(&id);
    let v = match res {
        Ok(Ok(v)) => v,
        _ => return Err(AppError::bad("El agente no contestó a tiempo. Revisá que siga conectado y probá de nuevo.")),
    };
    if v["ok"].as_bool() != Some(true) {
        return Err(AppError::bad(v["error"].as_str().unwrap_or("No se pudo completar")));
    }
    Ok(v["data"].clone())
}

static STATE_CACHE: LazyLock<Mutex<HashMap<Id, (std::time::Instant, Value)>>> = LazyLock::new(Default::default);
static ASKED: LazyLock<Mutex<HashMap<(Id, Id), std::time::Instant>>> = LazyLock::new(Default::default);

/// ¿La carpeta del proyecto ya es un repositorio? Lo puede preguntar cualquier miembro (la terminal lo usa para avisar).
/// Dirección de esta máquina en la red local (para armar la invitación cuando el admin entra por localhost).
async fn lan(_user: AuthUser) -> Json<Value> {
    let ip = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|s| { s.connect("192.0.2.1:9")?; s.local_addr() }).ok().map(|a| a.ip().to_string());
    Json(json!({ "ip": ip }))
}

async fn state(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team: Id = sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)?;
    routes::require(&st, team, user.id, Role::Member).await?;
    if let Some((t, v)) = STATE_CACHE.lock().unwrap().get(&project) {
        if t.elapsed() < Duration::from_secs(20) {
            return Ok(Json(v.clone()));
        }
    }
    let (agent, dir) = target(&st, project, team).await?;
    let s = dispatch(agent, json!({ "mode": "status", "dir": dir }), 20).await?;
    let v = json!({ "is_repo": s["is_repo"], "commits": s["commits"], "connected": s["origin"].as_str().is_some_and(|o| !o.is_empty()) });
    STATE_CACHE.lock().unwrap().insert(project, (std::time::Instant::now(), v.clone()));
    Ok(Json(v))
}

/// Un miembro le pide a los owners que creen el repositorio del proyecto.
async fn request_repo(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team: Id = sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)?;
    routes::require(&st, team, user.id, Role::Member).await?;
    {
        let mut m = ASKED.lock().unwrap();
        if m.get(&(project, user.id)).is_some_and(|t| t.elapsed() < Duration::from_secs(600)) {
            return Err(AppError::bad("Ya les avisaste hace poco a los owners. Esperá un rato antes de volver a pedirlo."));
        }
        m.insert((project, user.id), std::time::Instant::now());
    }
    let (me, pname): (String, String) = (
        sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?,
        sqlx::query_scalar("SELECT name FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await?,
    );
    let owners: Vec<Id> = sqlx::query_scalar("SELECT user_id FROM team_members WHERE team_id = ? AND role = 'owner'").bind(team).fetch_all(&st.db).await?;
    for o in &owners {
        crate::notify::notify(&st, *o, "github", Some(team), None, format!("🙋 {me} pide que se cree el repositorio del proyecto «{pname}». Sin repositorio nadie del equipo puede editar ni enviar su trabajo. Abrí 🖥️ Terminales → ⚙ Conexión → «🐙 Git y GitHub».")).await;
    }
    Ok(Json(json!({ "notified": owners.len() })))
}

async fn agent_pending(user: AuthUser) -> Result<Json<Value>> {
    if !user.bot {
        return Err(AppError::forbidden());
    }
    let mut g = JOBS.lock().unwrap();
    let list: Vec<Value> = g
        .iter_mut()
        .filter(|(_, j)| j.agent == user.id && !j.taken)
        .map(|(id, j)| {
            j.taken = true;
            json!({ "id": id, "payload": j.payload })
        })
        .collect();
    Ok(Json(Value::Array(list)))
}

async fn agent_result(user: AuthUser, Path(id): Path<String>, Json(v): Json<Value>) -> Result<Json<Value>> {
    if !user.bot {
        return Err(AppError::forbidden());
    }
    let tx = {
        let mut g = JOBS.lock().unwrap();
        match g.get_mut(&id) {
            Some(j) if j.agent == user.id => j.tx.take(),
            _ => None,
        }
    };
    if let Some(tx) = tx {
        let _ = tx.send(v);
    }
    Ok(Json(json!({ "ok": true })))
}

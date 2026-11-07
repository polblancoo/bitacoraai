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
        .route("/api/projects/{id}/git/setup", post(setup))
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
    let agent: Id = sqlx::query_scalar::<_, Option<Id>>("SELECT agent_id FROM ssh_targets WHERE project_id = ?")
        .bind(project)
        .fetch_optional(&st.db)
        .await?
        .flatten()
        .ok_or_else(|| AppError::bad("Primero conectá el agente de la máquina del proyecto (botón «▶ Conectar»)"))?;
    if !term::agent_online(agent) {
        return Err(AppError::bad("El agente de la máquina del proyecto no está conectado ahora"));
    }
    let dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await.unwrap_or_default();
    if dir.trim().is_empty() {
        return Err(AppError::bad("El proyecto no tiene carpeta de trabajo: definila en los ajustes del proyecto"));
    }
    let mut payload = json!({ "mode": b.mode, "dir": dir });
    match b.mode.as_str() {
        "status" | "local" => {}
        "clone" | "attach" | "new" => {
            let g = gh(&st)?;
            let token = token_of(&st, user.id).await?;
            let repo = b.repo.trim().trim_start_matches("https://github.com/").trim_end_matches(".git").trim_matches('/').to_string();
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
    let (tx, rx) = oneshot::channel();
    let id = uuid::Uuid::new_v4().simple().to_string();
    JOBS.lock().unwrap().insert(id.clone(), Job { agent, payload, taken: false, tx: Some(tx) });
    let res = tokio::time::timeout(Duration::from_secs(170), rx).await;
    JOBS.lock().unwrap().remove(&id);
    let v = match res {
        Ok(Ok(v)) => v,
        _ => return Err(AppError::bad("El agente no contestó a tiempo. Revisá que siga conectado y probá de nuevo.")),
    };
    if v["ok"].as_bool() != Some(true) {
        return Err(AppError::bad(v["error"].as_str().unwrap_or("No se pudo completar")));
    }
    let data = v["data"].clone();
    // el repositorio queda vinculado al proyecto (issues, Pull Requests, sincronización)
    if matches!(b.mode.as_str(), "clone" | "attach" | "new") {
        if let Some(repo) = data["repo"].as_str().filter(|r| valid_repo(r)) {
            sqlx::query("INSERT INTO project_github (project_id, repo) VALUES (?, ?) ON CONFLICT(project_id) DO UPDATE SET repo = excluded.repo").bind(project).bind(repo).execute(&st.db).await?;
        }
    }
    tracing::info!("git: usuario {} ejecutó «{}» en el proyecto {project}", user.id, b.mode);
    Ok(Json(data))
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

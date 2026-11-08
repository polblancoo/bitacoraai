//! Dónde vive el código, por proyecto: «GitHub permitido» o «todo en el servidor». Para consultorios, clínicas y estudios
//! jurídicos nada del trabajo puede salir a un servicio externo: con «todo en el servidor» se cortan GitHub (repositorios,
//! issues, Pull Requests, publicación) y las IAs con aislamiento dejan de llegar a github.com y gitlab.com en ese proyecto.
//! Hay un valor por defecto para todos los proyectos y cada proyecto puede tener el suyo. Antes de cortar un proyecto que
//! ya está en GitHub, se baja todo (ramas y etiquetas) a la carpeta del servidor; si no se puede, no se corta.
use crate::{auth::AuthUser, error::{AppError, Result}, gitsetup, license, sysops, AppState};
use axum::{extract::State, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::{collections::HashMap, sync::{atomic::Ordering, LazyLock, RwLock}};
use tm_core::Id;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/admin/git-mode", get(read).put(save)).route("/api/git-mode", get(mine))
}

pub const BLOCKED_MSG: &str = "El administrador del servidor desactivó GitHub para este proyecto: el código queda solo en este servidor";

/// Proyectos con un modo propio (true = todo en el servidor). Los demás siguen el valor por defecto.
static OVERRIDE: LazyLock<RwLock<HashMap<Id, bool>>> = LazyLock::new(Default::default);

/// Valor por defecto del servidor.
pub fn internal(st: &AppState) -> bool {
    st.git_internal.load(Ordering::Relaxed)
}

/// ¿Este proyecto guarda todo en el servidor?
pub fn internal_for(st: &AppState, project: Id) -> bool {
    OVERRIDE.read().ok().and_then(|m| m.get(&project).copied()).unwrap_or_else(|| internal(st))
}

pub async fn load(st: &AppState) {
    let v = license::meta_get(&st.db, "git_mode").await.unwrap_or_default();
    st.git_internal.store(v == "interno", Ordering::Relaxed);
    let rows = sqlx::query("SELECT key, value FROM meta WHERE key LIKE 'gitmode_p%'").fetch_all(&st.db).await.unwrap_or_default();
    let mut m = HashMap::new();
    for r in rows {
        let (k, v): (String, String) = (r.get(0), r.get(1));
        if let Some(id) = k.strip_prefix("gitmode_p").and_then(|n| n.parse::<Id>().ok()) {
            match v.as_str() {
                "interno" => { m.insert(id, true); }
                "github" => { m.insert(id, false); }
                _ => {}
            }
        }
    }
    if let Ok(mut w) = OVERRIDE.write() {
        *w = m;
    }
}

/// Lo que cualquier persona necesita para saber qué botones mostrar.
async fn mine(State(st): State<AppState>, _user: AuthUser) -> Result<Json<Value>> {
    let ids: Vec<Id> = sqlx::query_scalar("SELECT id FROM projects").fetch_all(&st.db).await?;
    let internal_ids: Vec<Id> = ids.into_iter().filter(|p| internal_for(&st, *p)).collect();
    Ok(Json(json!({ "default": if internal(&st) { "interno" } else { "github" }, "internal": internal_ids })))
}

async fn read(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    sysops::need(&st, &user, false).await?;
    let rows = sqlx::query("SELECT p.id, p.name, t.name, COALESCE((SELECT repo FROM project_github g WHERE g.project_id = p.id), '') FROM projects p JOIN teams t ON t.id = p.team_id ORDER BY t.name, p.name LIMIT 500")
        .fetch_all(&st.db).await?;
    let own = OVERRIDE.read().map(|m| m.clone()).unwrap_or_default();
    let projects: Vec<Value> = rows.iter().map(|r| {
        let id: Id = r.get(0);
        json!({ "id": id, "name": r.get::<String, _>(1), "team": r.get::<String, _>(2), "github": r.get::<String, _>(3),
                "mode": if internal_for(&st, id) { "interno" } else { "github" }, "own": own.contains_key(&id) })
    }).collect();
    Ok(Json(json!({ "default": if internal(&st) { "interno" } else { "github" }, "can_edit": sysops::need(&st, &user, true).await.is_ok(), "projects": projects })))
}

#[derive(Deserialize)]
struct Mode {
    mode: String,
    /// Si viene, el cambio es solo para ese proyecto. `mode` puede ser «heredar» para volver al valor por defecto.
    project: Option<Id>,
    /// Cortar aunque no se haya podido bajar todo de GitHub.
    #[serde(default)]
    force: bool,
}

/// Baja a la carpeta del servidor todo lo que el proyecto tiene en GitHub y le quita la subida. Devuelve lo que bajó.
async fn mirror(st: &AppState, project: Id, lock: bool) -> Result<Value> {
    let repo: Option<String> = sqlx::query_scalar("SELECT repo FROM project_github WHERE project_id = ?").bind(project).fetch_optional(&st.db).await?;
    if repo.as_deref().unwrap_or("").is_empty() {
        return Ok(json!({ "skipped": true }));
    }
    let team = crate::routes::team_of_project(st, project).await?;
    let (agent, dir) = gitsetup::target(st, project, team).await?;
    gitsetup::dispatch(agent, json!({ "mode": "mirror", "dir": dir, "lock": lock }), 170).await
}

async fn audit(st: &AppState, user: Id, project: Option<Id>, text: &str) {
    let teams: Vec<Id> = match project {
        Some(p) => crate::routes::team_of_project(st, p).await.ok().into_iter().collect(),
        None => sqlx::query("SELECT team_id FROM team_members WHERE user_id = ?").bind(user).fetch_all(&st.db).await.unwrap_or_default().iter().map(|r| r.get(0)).collect(),
    };
    for t in teams {
        crate::audit::record(st, t, project, None, user, "politica_git", text).await;
    }
}

async fn save(State(st): State<AppState>, user: AuthUser, Json(b): Json<Mode>) -> Result<Json<Value>> {
    sysops::need(&st, &user, true).await?;
    let mut report = json!({});
    match b.project {
        Some(p) => {
            if !matches!(b.mode.as_str(), "github" | "interno" | "heredar") {
                return Err(AppError::bad("modo inválido"));
            }
            let name: String = sqlx::query_scalar("SELECT name FROM projects WHERE id = ?").bind(p).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)?;
            let was = internal_for(&st, p);
            let will = match b.mode.as_str() { "interno" => true, "github" => false, _ => internal(&st) };
            if will && !was {
                // primero se baja todo al servidor; si no se puede, no se corta
                match mirror(&st, p, true).await {
                    Ok(v) => report = v,
                    Err(e) if b.force => report = json!({ "warning": e.1 }),
                    Err(e) => return Err(AppError::bad(&format!("No pude bajar «{name}» de GitHub al servidor, así que no corté nada ({}). Revisá que el agente del proyecto esté conectado.", e.1))),
                }
            } else if !will && was {
                let _ = mirror(&st, p, false).await;
            }
            license::meta_set(&st.db, &format!("gitmode_p{p}"), if b.mode == "heredar" { "" } else { &b.mode }).await?;
            load(&st).await;
            if was != will {
                audit(&st, user.id, Some(p), &format!("código de «{name}»: {}", if will { "todo en el servidor, GitHub desactivado" } else { "GitHub permitido" })).await;
            }
        }
        None => {
            if !matches!(b.mode.as_str(), "github" | "interno") {
                return Err(AppError::bad("modo inválido"));
            }
            let was = internal(&st);
            if b.mode == "interno" && !was {
                // los proyectos que siguen el valor por defecto y están en GitHub se bajan primero
                let ids: Vec<(Id, String)> = sqlx::query_as("SELECT p.id, p.name FROM projects p JOIN project_github g ON g.project_id = p.id WHERE g.repo != ''").fetch_all(&st.db).await?;
                let mut failed = Vec::new();
                let mut done = 0;
                for (pid, pname) in ids {
                    if OVERRIDE.read().map(|m| m.contains_key(&pid)).unwrap_or(false) {
                        continue;
                    }
                    match mirror(&st, pid, true).await {
                        Ok(_) => done += 1,
                        Err(_) => failed.push(pname),
                    }
                }
                if !failed.is_empty() && !b.force {
                    return Err(AppError::bad(&format!("No pude bajar de GitHub al servidor: {}. No corté nada; conectá el agente de esos proyectos o cambialos uno por uno.", failed.join(", "))));
                }
                report = json!({ "mirrored": done, "failed": failed });
            }
            license::meta_set(&st.db, "git_mode", &b.mode).await?;
            load(&st).await;
            if was != internal(&st) {
                audit(&st, user.id, None, &format!("código (por defecto): {}", if internal(&st) { "todo en el servidor, GitHub desactivado" } else { "GitHub permitido" })).await;
            }
        }
    }
    tracing::info!("git: modo {} {:?} (cambiado por #{})", b.mode, b.project, user.id);
    let mut out = read(State(st), user).await?.0;
    out["report"] = report;
    Ok(Json(out))
}

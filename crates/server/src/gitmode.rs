//! Dónde vive el código: «GitHub permitido» o «todo en el servidor». Para consultorios, clínicas y estudios jurídicos nada
//! del trabajo puede salir a un servicio externo: con «todo en el servidor» se cortan GitHub (cuenta, repositorios, issues,
//! Pull Requests, publicación) y las IAs con aislamiento dejan de poder llegar a github.com y gitlab.com.
use crate::{auth::AuthUser, error::Result, license, sysops, AppState};
use axum::{extract::State, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::atomic::Ordering;
use tm_core::Id;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/admin/git-mode", get(read).put(save))
}

pub const BLOCKED_MSG: &str = "El administrador del servidor desactivó GitHub: el código de los proyectos queda solo en este servidor";

pub fn internal(st: &AppState) -> bool {
    st.git_internal.load(Ordering::Relaxed)
}

pub async fn load(st: &AppState) {
    let v = license::meta_get(&st.db, "git_mode").await.unwrap_or_default();
    st.git_internal.store(v == "interno", Ordering::Relaxed);
}

async fn read(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    sysops::need(&st, &user, false).await?;
    Ok(Json(json!({ "mode": if internal(&st) { "interno" } else { "github" }, "can_edit": sysops::need(&st, &user, true).await.is_ok() })))
}

#[derive(Deserialize)]
struct Mode {
    mode: String,
}

async fn save(State(st): State<AppState>, user: AuthUser, Json(b): Json<Mode>) -> Result<Json<Value>> {
    sysops::need(&st, &user, true).await?;
    if !matches!(b.mode.as_str(), "github" | "interno") {
        return Err(crate::error::AppError::bad("modo inválido"));
    }
    let was = internal(&st);
    license::meta_set(&st.db, "git_mode", &b.mode).await?;
    load(&st).await;
    if was != internal(&st) {
        let teams: Vec<Id> = sqlx::query("SELECT team_id FROM team_members WHERE user_id = ?").bind(user.id).fetch_all(&st.db).await?.iter().map(|r| r.get(0)).collect();
        for t in teams {
            crate::audit::record(&st, t, None, None, user.id, "politica_git", &format!("código: {}", if internal(&st) { "todo en el servidor, GitHub desactivado" } else { "GitHub permitido" })).await;
        }
        tracing::info!("git: modo {} (cambiado por #{})", b.mode, user.id);
    }
    read(State(st), user).await
}

//! Integraciones del equipo: enlaces a herramientas externas (videollamada con Jitsi, pizarras, editores…) que se abren en una
//! pestaña propia dentro de la app o en una ventana nueva. Las configura un administrador; todo el equipo las usa.
//! La URL admite {sala} (un código difícil de adivinar, distinto por equipo), {equipo} y {usuario}.
use crate::{auth::AuthUser, error::{AppError, Result}, routes, AppState};
use axum::{extract::{Path, State}, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tm_core::{Id, Role};

const MAX_ITEMS: usize = 12;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/teams/{id}/integrations", get(list).put(save))
}

/// Código de sala por equipo: hash de un secreto del servidor, para que nadie entre a la reunión adivinando el nombre.
async fn room(st: &AppState, team: Id) -> String {
    let saved: Option<String> = sqlx::query_scalar("SELECT value FROM meta WHERE key = 'room_secret'").fetch_optional(&st.db).await.ok().flatten();
    let secret = match saved {
        Some(s) => s,
        None => {
            let mut b = [0u8; 24];
            let _ = getrandom::fill(&mut b);
            let s: String = b.iter().map(|x| format!("{x:02x}")).collect();
            let _ = sqlx::query("INSERT OR IGNORE INTO meta (key, value) VALUES ('room_secret', ?)").bind(&s).execute(&st.db).await;
            s
        }
    };
    let h = Sha256::digest(format!("{secret}:{team}").as_bytes());
    h.iter().take(10).map(|x| format!("{x:02x}")).collect()
}

async fn list(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Viewer).await?;
    let rows = sqlx::query("SELECT id, name, icon, url, mode FROM integrations WHERE team_id = ? ORDER BY position, id").bind(team).fetch_all(&st.db).await?;
    let tname: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await.unwrap_or_default();
    Ok(Json(json!({
        "room": room(&st, team).await,
        "team": tname,
        "items": rows.iter().map(|r| json!({ "id": r.get::<i64, _>(0), "name": r.get::<String, _>(1), "icon": r.get::<String, _>(2), "url": r.get::<String, _>(3), "mode": r.get::<String, _>(4) })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct Item {
    name: String,
    #[serde(default)]
    icon: String,
    url: String,
    #[serde(default)]
    mode: String,
}
#[derive(Deserialize)]
struct Save {
    items: Vec<Item>,
}

async fn save(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<Save>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    if b.items.len() > MAX_ITEMS {
        return Err(AppError::bad("máximo 12 integraciones"));
    }
    let mut clean = Vec::new();
    for i in &b.items {
        let name = i.name.trim();
        let url = i.url.trim();
        if name.is_empty() || name.chars().count() > 40 {
            return Err(AppError::bad("cada integración necesita un nombre de hasta 40 caracteres"));
        }
        if !(url.starts_with("https://") || url.starts_with("http://")) || url.len() > 400 || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(AppError::bad("la dirección tiene que empezar con https:// (o http:// en una red interna) y no llevar espacios"));
        }
        let icon: String = i.icon.trim().chars().take(4).collect();
        clean.push((name.to_string(), if icon.is_empty() { "🔌".to_string() } else { icon }, url.to_string(), if i.mode == "panel" { "panel" } else { "tab" }));
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM integrations WHERE team_id = ?").bind(team).execute(&mut *tx).await?;
    for (n, (name, icon, url, mode)) in clean.iter().enumerate() {
        sqlx::query("INSERT INTO integrations (team_id, name, icon, url, mode, position) VALUES (?, ?, ?, ?, ?, ?)").bind(team).bind(name).bind(icon).bind(url).bind(mode).bind(n as i64).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    list(State(st), user, Path(team)).await
}

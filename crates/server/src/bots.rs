//! Cuentas de IA: un usuario marcado como bot con un token de API propio (`tmk_…`), sumado a un equipo con
//! rol limitado. Lo crea y lo revoca una persona admin; el bot no puede administrar equipos ni invitaciones.
use crate::{
    auth::{hash_password, random_hex, sha256_hex, AuthUser, API_TOKEN_PREFIX},
    error::{AppError, Result},
    limits,
    routes::{require, role_of},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::{Id, Role, User};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/teams/{id}/bots", get(list_bots).post(create_bot))
        .route("/api/bots/{uid}/token", post(rotate).delete(revoke))
        .route("/api/bots/{uid}", axum::routing::patch(update_bot))
        .route("/api/me/deny", get(my_deny))
}

#[derive(Deserialize)]
struct NewBot {
    name: String,
    /// `viewer` o `member` (por defecto).
    role: Option<Role>,
    /// Etiqueta de qué IA es, por ejemplo `claude:Opus` (solo informativa).
    #[serde(default)]
    model: String,
}

#[derive(Deserialize)]
struct UpdateBot {
    model: Option<String>,
    /// Comandos prohibidos para esta IA, uno por línea (por ejemplo `Bash(rm:*)`).
    deny: Option<String>,
}

fn new_token() -> String {
    format!("{API_TOKEN_PREFIX}{}", random_hex(24))
}

async fn store_token(st: &AppState, bot: Id, by: Id) -> Result<String> {
    let token = new_token();
    sqlx::query("INSERT OR REPLACE INTO api_tokens (user_id, hash, created_by, created_at) VALUES (?, ?, ?, ?)")
        .bind(bot)
        .bind(sha256_hex(&token))
        .bind(by)
        .bind(Utc::now())
        .execute(&st.db)
        .await?;
    Ok(token)
}

/// El llamante debe ser admin+ en algún equipo al que pertenezca el bot.
async fn manages_bot(st: &AppState, caller: Id, bot: Id) -> Result<()> {
    let is_bot: Option<i64> = sqlx::query_scalar("SELECT is_bot FROM users WHERE id = ?").bind(bot).fetch_optional(&st.db).await?;
    if is_bot != Some(1) {
        return Err(AppError::not_found());
    }
    let teams: Vec<Id> = sqlx::query_scalar("SELECT team_id FROM team_members WHERE user_id = ?").bind(bot).fetch_all(&st.db).await?;
    for t in teams {
        if role_of(st, t, caller).await.is_ok_and(|r| r >= Role::Admin) {
            return Ok(());
        }
    }
    Err(AppError::forbidden())
}

async fn list_bots(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    require(&st, team, user.id, Role::Viewer).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.username, u.model, u.deny, m.role, m.joined_at, cb.username AS created_by, t.last_used, t.id AS tok \
         FROM team_members m JOIN users u ON u.id = m.user_id \
         LEFT JOIN api_tokens t ON t.user_id = u.id LEFT JOIN users cb ON cb.id = t.created_by \
         WHERE m.team_id = ? AND u.is_bot = 1 ORDER BY u.username",
    )
    .bind(team)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(Value::Array(
        rows.iter()
            .map(|r| {
                json!({
                    "id": r.get::<Id, _>("id"),
                    "username": r.get::<String, _>("username"),
                    "model": r.get::<String, _>("model"),
                    "deny": r.get::<String, _>("deny"),
                    "role": r.get::<String, _>("role"),
                    "joined_at": r.get::<Option<String>, _>("joined_at"),
                    "created_by": r.get::<Option<String>, _>("created_by"),
                    "last_used": r.get::<Option<String>, _>("last_used"),
                    "active": r.get::<Option<Id>, _>("tok").is_some(),
                })
            })
            .collect(),
    )))
}

async fn create_bot(
    State(st): State<AppState>,
    user: AuthUser,
    Path(team): Path<Id>,
    Json(b): Json<NewBot>,
) -> Result<Json<Value>> {
    user.human_only()?;
    require(&st, team, user.id, Role::Admin).await?;
    let role = b.role.unwrap_or(Role::Member);
    if !matches!(role, Role::Viewer | Role::Member) {
        return Err(AppError::bad("un bot solo puede ser lector o miembro"));
    }
    let name = b.name.trim();
    limits::username(name)?;
    limits::max_len("modelo", &b.model, 40)?;
    let hash = hash_password(&random_hex(32))?;
    let res = sqlx::query("INSERT INTO users (username, password_hash, is_bot, model) VALUES (?, ?, 1, ?)")
        .bind(name)
        .bind(hash)
        .bind(b.model.trim())
        .execute(&st.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => AppError(StatusCode::CONFLICT, "el usuario ya existe".into()),
            e => e.into(),
        })?;
    let id = res.last_insert_rowid();
    crate::routes::insert_member(&st, team, &User { id, username: name.into() }, role, Some(user.id), "bot", None).await?;
    let token = store_token(&st, id, user.id).await?;
    Ok(Json(json!({ "user": { "id": id, "username": name }, "model": b.model.trim(), "role": role.as_str(), "token": token })))
}

async fn rotate(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    manages_bot(&st, user.id, uid).await?;
    Ok(Json(json!({ "token": store_token(&st, uid, user.id).await? })))
}

async fn revoke(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>) -> Result<StatusCode> {
    user.human_only()?;
    manages_bot(&st, user.id, uid).await?;
    sqlx::query("DELETE FROM api_tokens WHERE user_id = ?").bind(uid).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_bot(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>, Json(b): Json<UpdateBot>) -> Result<StatusCode> {
    user.human_only()?;
    manages_bot(&st, user.id, uid).await?;
    if let Some(model) = &b.model {
        limits::max_len("modelo", model, 40)?;
        sqlx::query("UPDATE users SET model = ? WHERE id = ?").bind(model.trim()).bind(uid).execute(&st.db).await?;
    }
    if let Some(deny) = &b.deny {
        limits::max_len("lista de comandos prohibidos", deny, 4000)?;
        let clean = deny.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
        sqlx::query("UPDATE users SET deny = ? WHERE id = ?").bind(clean).bind(uid).execute(&st.db).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// La IA lee su propia lista de comandos prohibidos (la edita un admin desde la app).
async fn my_deny(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    let deny: String = sqlx::query_scalar("SELECT deny FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(json!({ "deny": deny })))
}

//! Perfil, verificación de email, buscador de usuarios y bandeja de novedades.
use crate::{
    auth::{random_hex, sha256_hex, AuthUser},
    error::{AppError, Result},
    limits::{self, max_len},
    notify::valid_email,
    routes::{fetch_task, require, TASK_SELECT},
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::Row;
use std::time::Duration;
use tm_core::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/me", get(me))
        .route("/api/me/email", put(set_email).delete(clear_email))
        .route("/api/me/email/verify", post(verify_email))
        .route("/api/me/telegram", delete(telegram_unlink))
        .route("/api/me/telegram/link", post(telegram_link))
        .route("/api/teams/{id}/messages", post(team_message))
        .route("/api/users/search", get(search_users))
        .route("/api/inbox", get(inbox))
        .route("/api/notifications/read", post(mark_read))
        .route("/api/notifications/{id}", delete(delete_notification))
}

async fn me(State(st): State<AppState>, user: AuthUser) -> Result<Json<Me>> {
    let r = sqlx::query("SELECT id, username, email, email_verified, telegram_chat_id FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(Me {
        id: r.get("id"),
        username: r.get("username"),
        email: r.get("email"),
        email_verified: r.get::<i64, _>("email_verified") == 1,
        email_available: st.mailer.is_some(),
        telegram_linked: r.get::<Option<i64>, _>("telegram_chat_id").is_some(),
        telegram_available: st.telegram.is_some(),
        telegram_bot: st.telegram.as_ref().map(|t| t.bot.clone()),
    }))
}

/// Guarda el email como NO verificado y manda un código de 6 dígitos a esa dirección.
/// Solo los emails verificados reciben novedades: así nadie puede usar el servidor para
/// escribirle a un tercero.
async fn set_email(State(st): State<AppState>, user: AuthUser, Json(b): Json<SetEmail>) -> Result<StatusCode> {
    let Some(mailer) = st.mailer.clone() else {
        return Err(AppError(StatusCode::SERVICE_UNAVAILABLE, "el servidor no tiene el envío de emails configurado".into()));
    };
    let email = b.email.trim().to_lowercase();
    max_len("email", &email, limits::EMAIL_MAX)?;
    if !valid_email(&email) {
        return Err(AppError::bad("email inválido"));
    }
    st.user_limiter.hit(&format!("email-code:{}", user.id), 3, Duration::from_secs(3600))?;
    let code = format!("{:06}", u32::from_str_radix(&random_hex(4), 16).unwrap_or(0) % 1_000_000);
    sqlx::query("UPDATE users SET email = ?, email_verified = 0, email_code_hash = ?, email_code_expires = ? WHERE id = ?")
        .bind(&email)
        .bind(sha256_hex(&format!("{}:{code}", user.id)))
        .bind(Utc::now() + chrono::Duration::minutes(15))
        .bind(user.id)
        .execute(&st.db)
        .await?;
    let sent = mailer
        .send(&email, "[Rust TUI Manager] Código de verificación", format!("Tu código de verificación es {code}. Vence en 15 minutos.\n\nSi no lo pediste, ignorá este mensaje."))
        .await;
    if let Err(e) = sent {
        tracing::warn!("verificación de email: {e}");
        return Err(AppError(StatusCode::BAD_GATEWAY, "no se pudo enviar el email de verificación".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn verify_email(State(st): State<AppState>, user: AuthUser, Json(b): Json<VerifyEmail>) -> Result<StatusCode> {
    st.user_limiter.hit(&format!("email-verify:{}", user.id), 5, Duration::from_secs(900))?;
    max_len("código", &b.code, 12)?;
    let r = sqlx::query("SELECT email_code_hash, email_code_expires FROM users WHERE id = ? AND email IS NOT NULL").bind(user.id).fetch_optional(&st.db).await?;
    let ok = r
        .and_then(|r| {
            let hash: Option<String> = r.get("email_code_hash");
            let exp: Option<chrono::DateTime<Utc>> = r.get("email_code_expires");
            Some((hash?, exp?))
        })
        .is_some_and(|(hash, exp)| exp > Utc::now() && hash == sha256_hex(&format!("{}:{}", user.id, b.code.trim())));
    if !ok {
        return Err(AppError::bad("código inválido o vencido"));
    }
    sqlx::query("UPDATE users SET email_verified = 1, email_code_hash = NULL, email_code_expires = NULL WHERE id = ?").bind(user.id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clear_email(State(st): State<AppState>, user: AuthUser) -> Result<StatusCode> {
    sqlx::query("UPDATE users SET email = NULL, email_verified = 0, email_code_hash = NULL, email_code_expires = NULL WHERE id = ?").bind(user.id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    team_id: Id,
}

/// Busca usuarios por prefijo para agregarlos a un equipo. Solo para admins del equipo,
/// con tope de consultas, y excluye a quienes ya son miembros.
async fn search_users(State(st): State<AppState>, user: AuthUser, Query(p): Query<SearchParams>) -> Result<Json<Vec<User>>> {
    require(&st, p.team_id, user.id, Role::Admin).await?;
    st.user_limiter.hit(&format!("search:{}", user.id), 120, Duration::from_secs(60))?;
    let q = p.q.trim();
    max_len("búsqueda", q, limits::USERNAME_MAX)?;
    if q.is_empty() {
        return Ok(Json(vec![]));
    }
    // escapa los comodines de LIKE para que el texto se tome literal; primero los que empiezan igual
    let like = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let prefix = format!("{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let rows = sqlx::query(
        "SELECT id, username FROM users WHERE username LIKE ? ESCAPE '\\' \
         AND id NOT IN (SELECT user_id FROM team_members WHERE team_id = ?) \
         ORDER BY username LIKE ? ESCAPE '\\' DESC, username LIMIT 8",
    )
    .bind(&like)
    .bind(p.team_id)
    .bind(&prefix)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(rows.iter().map(|r| User { id: r.get("id"), username: r.get("username") }).collect()))
}

fn notification_from_row(r: &sqlx::sqlite::SqliteRow) -> Notification {
    Notification {
        id: r.get("id"),
        user_id: r.get("user_id"),
        kind: r.get("kind"),
        team_id: r.get("team_id"),
        task_id: r.get("task_id"),
        text: r.get("text"),
        read: r.get::<i64, _>("read") == 1,
        created_at: r.get("created_at"),
    }
}

/// Lo primero que se ve al iniciar sesión: novedades sin leer y mis tareas abiertas.
async fn inbox(State(st): State<AppState>, user: AuthUser) -> Result<Json<Inbox>> {
    let unread: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = ? AND read = 0").bind(user.id).fetch_one(&st.db).await?;
    let notes = sqlx::query("SELECT id, user_id, kind, team_id, task_id, text, read, created_at FROM notifications WHERE user_id = ? ORDER BY read, id DESC LIMIT 50")
        .bind(user.id)
        .fetch_all(&st.db)
        .await?;
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{TASK_SELECT} WHERE t.assignee_id = ? AND t.status != 'done' AND t.parent_id IS NULL \
         AND t.project_id IN (SELECT p.id FROM projects p JOIN team_members m ON m.team_id = p.team_id WHERE m.user_id = ?) \
         ORDER BY t.priority DESC, t.due IS NULL, t.due LIMIT 100"
    )))
    .bind(user.id)
    .bind(user.id)
    .fetch_all(&st.db)
    .await?;
    let mut tasks = Vec::new();
    for r in &rows {
        let id: Id = r.get("id");
        let project_id: Id = r.get("project_id");
        let task = fetch_task(&st, id).await?;
        let info = sqlx::query("SELECT p.name AS project, t.name AS team FROM projects p JOIN teams t ON t.id = p.team_id WHERE p.id = ?")
            .bind(project_id)
            .fetch_one(&st.db)
            .await?;
        tasks.push(InboxTask { task, project: info.get("project"), team: info.get("team") });
    }
    Ok(Json(Inbox { unread, notifications: notes.iter().map(notification_from_row).collect(), tasks }))
}

async fn mark_read(State(st): State<AppState>, user: AuthUser, Json(b): Json<MarkRead>) -> Result<StatusCode> {
    if b.ids.len() > 200 {
        return Err(AppError::bad("demasiados ids"));
    }
    if b.ids.is_empty() {
        sqlx::query("UPDATE notifications SET read = 1 WHERE user_id = ?").bind(user.id).execute(&st.db).await?;
    } else {
        for id in &b.ids {
            sqlx::query("UPDATE notifications SET read = 1 WHERE user_id = ? AND id = ?").bind(user.id).bind(id).execute(&st.db).await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_notification(State(st): State<AppState>, user: AuthUser, axum::extract::Path(id): axum::extract::Path<Id>) -> Result<StatusCode> {
    sqlx::query("DELETE FROM notifications WHERE user_id = ? AND id = ?").bind(user.id).bind(id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Genera un código de un solo uso (15 min) para vincular Telegram: el usuario se lo manda al bot.
async fn telegram_link(State(st): State<AppState>, user: AuthUser) -> Result<Json<TelegramLink>> {
    let Some(tg) = st.telegram.clone() else {
        return Err(AppError(StatusCode::SERVICE_UNAVAILABLE, "el servidor no tiene Telegram configurado".into()));
    };
    st.user_limiter.hit(&format!("tg-code:{}", user.id), 5, Duration::from_secs(3600))?;
    let code = random_hex(5);
    sqlx::query("UPDATE users SET tg_code_hash = ?, tg_code_expires = ? WHERE id = ?")
        .bind(crate::telegram::code_hash(&code))
        .bind(Utc::now() + chrono::Duration::minutes(15))
        .bind(user.id)
        .execute(&st.db)
        .await?;
    Ok(Json(TelegramLink { code, bot: tg.bot.clone(), expires_minutes: 15 }))
}

async fn telegram_unlink(State(st): State<AppState>, user: AuthUser) -> Result<StatusCode> {
    sqlx::query("UPDATE users SET telegram_chat_id = NULL, tg_code_hash = NULL, tg_code_expires = NULL WHERE id = ?").bind(user.id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Mensaje del equipo: llega a la bandeja de cada destinatario y además por email y Telegram si los tiene.
async fn team_message(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<TeamMessage>) -> Result<Json<MessageSent>> {
    require(&st, team, user.id, Role::Member).await?;
    st.user_limiter.hit(&format!("msg:{}", user.id), 20, Duration::from_secs(3600))?;
    let text = b.text.trim();
    if text.is_empty() {
        return Err(AppError::bad("el mensaje está vacío"));
    }
    max_len("mensaje", text, 1000)?;
    if b.to.len() > limits::MAX_MEMBERS_PER_MESSAGE {
        return Err(AppError::bad("demasiados destinatarios"));
    }
    let members: Vec<Id> = sqlx::query_scalar("SELECT user_id FROM team_members WHERE team_id = ? AND user_id != ?")
        .bind(team)
        .bind(user.id)
        .fetch_all(&st.db)
        .await?;
    let targets: Vec<Id> = if b.to.is_empty() { members } else {
        if b.to.iter().any(|t| !members.contains(t)) {
            return Err(AppError::bad("hay destinatarios que no son del equipo"));
        }
        b.to.clone()
    };
    let who = crate::notify::username_of(&st, user.id).await;
    let team_name: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    for t in &targets {
        crate::notify::notify(&st, *t, "message", Some(team), None, format!("{who} en «{team_name}»: {text}")).await;
    }
    Ok(Json(MessageSent { sent: targets.len() as i64 }))
}

//! Primeros pasos del servidor: estado de cada pieza (correo, Telegram, GitHub, Google, HTTPS) y asistentes para
//! conectar Telegram, GitHub y Google desde la app. Cada dato se prueba contra el proveedor antes de guardarse,
//! los secretos se guardan cifrados y el cambio rige al instante (sin editar archivos ni reiniciar).
//! Lo que ya estaba en variables de entorno sigue andando; lo guardado acá lo reemplaza.
use crate::{auth::AuthUser, error::{AppError, Result}, github::Github, google::Google, sysops::need, telegram::Telegram, AppState};
use axum::{extract::State, routing::put, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/setup", axum::routing::get(status))
        .route("/api/admin/setup/telegram", put(tg_save).delete(tg_clear))
        .route("/api/admin/setup/github", put(gh_save).delete(gh_clear))
        .route("/api/admin/setup/google", put(gg_save).delete(gg_clear))
}

async fn meta(st: &AppState, k: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM meta WHERE key = ?").bind(format!("cfg_{k}")).fetch_optional(&st.db).await.ok().flatten().filter(|v| !v.is_empty())
}

async fn put_meta(st: &AppState, k: &str, v: &str) -> Result<()> {
    sqlx::query("INSERT INTO meta(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value").bind(format!("cfg_{k}")).bind(v).execute(&st.db).await?;
    Ok(())
}

async fn del_meta(st: &AppState, prefix: &str) -> Result<()> {
    sqlx::query("DELETE FROM meta WHERE key LIKE ? ESCAPE '\\'").bind(format!("cfg\\_{prefix}\\_%")).execute(&st.db).await?;
    Ok(())
}

/// Al arrancar: lo guardado en la app reemplaza a las variables de entorno.
pub async fn load(st: &AppState) {
    if let Some(tok) = meta(st, "tg_token").await.and_then(|t| st.cred.open(&t)) {
        match Telegram::from_token(&tok).await {
            Ok(t) => {
                tracing::info!("Telegram habilitado desde la app (bot @{})", t.bot);
                st.telegram.set(Some(Arc::new(t)));
            }
            Err(e) => tracing::error!("el token de Telegram guardado no funciona: {e}"),
        }
    }
    if let Some(id) = meta(st, "gh_id").await {
        st.github.set(Github::from_parts(&id, &meta(st, "gh_scope").await.unwrap_or_default()).map(Arc::new));
        tracing::info!("GitHub habilitado desde la app");
    }
    if let (Some(id), Some(sec), Some(red)) = (meta(st, "gg_id").await, meta(st, "gg_secret").await.and_then(|t| st.cred.open(&t)), meta(st, "gg_redirect").await) {
        st.google.set(Google::from_parts(&id, &sec, &red).map(Arc::new));
        tracing::info!("Google Calendar habilitado desde la app");
    }
}

/// Cada pieza: si está activa y de dónde viene («app» = guardada acá, «env» = variable de entorno).
async fn status(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let src = |saved: bool, env: &str| if saved { "app" } else if std::env::var(env).is_ok() { "env" } else { "none" };
    let tg_saved = meta(&st, "tg_token").await.is_some();
    let gh_saved = meta(&st, "gh_id").await;
    let gg_saved = meta(&st, "gg_id").await;
    let mail_saved = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM meta WHERE key = 'smtp_host'").fetch_one(&st.db).await.unwrap_or(0) > 0;
    Ok(Json(json!({
        "https": st.tls,
        "mail": { "active": st.mailer.is_some(), "source": src(mail_saved, "TM_SMTP_URL") },
        "telegram": { "active": st.telegram.is_some(), "source": src(tg_saved, "TM_TELEGRAM_TOKEN"), "bot": st.telegram.get().map(|t| t.bot.clone()) },
        "github": { "active": st.github.is_some(), "source": src(gh_saved.is_some(), "TM_GITHUB_CLIENT_ID"), "client_id": gh_saved, "scope": meta(&st, "gh_scope").await },
        "google": { "active": st.google.is_some(), "source": src(gg_saved.is_some(), "TM_GOOGLE_CLIENT_ID"), "client_id": gg_saved, "redirect": meta(&st, "gg_redirect").await, "has_secret": meta(&st, "gg_secret").await.is_some() },
    })))
}

#[derive(Deserialize)]
struct TgIn {
    token: String,
}

async fn tg_save(State(st): State<AppState>, user: AuthUser, Json(b): Json<TgIn>) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let token = b.token.trim();
    if !token.contains(':') || token.len() < 20 || token.contains(char::is_whitespace) {
        return Ok(Json(json!({ "ok": false, "msg": "Ese no parece un token de Telegram: tiene la forma 123456789:AAF… (lo da @BotFather al crear el bot). Copialo completo." })));
    }
    match Telegram::from_token(token).await {
        Ok(t) => {
            let bot = t.bot.clone();
            put_meta(&st, "tg_token", &st.cred.seal(token)).await?;
            st.telegram.set(Some(Arc::new(t)));
            Ok(Json(json!({ "ok": true, "bot": bot, "msg": format!("Listo: el bot @{bot} quedó conectado. Cada persona lo vincula desde Configuración → Telegram.") })))
        }
        Err(e) => Ok(Json(json!({ "ok": false, "msg": format!("Telegram rechazó el token ({e}). Revisá que lo copiaste completo; si lo regeneraste en @BotFather, usá el nuevo.") }))),
    }
}

async fn tg_clear(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    del_meta(&st, "tg").await?;
    st.telegram.set(Telegram::from_env().await.map(Arc::new));
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct GhIn {
    client_id: String,
    #[serde(default)]
    scope: String,
}

async fn gh_save(State(st): State<AppState>, user: AuthUser, Json(b): Json<GhIn>) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let id = b.client_id.trim();
    if id.len() < 10 || id.contains(char::is_whitespace) {
        return Ok(Json(json!({ "ok": false, "msg": "Ese Client ID no es válido: es un texto corto, sin espacios, como Ov23li… (está en la página de tu OAuth App)." })));
    }
    let g = Github::from_parts(id, &b.scope).ok_or_else(|| AppError::bad("no pude preparar la conexión con GitHub"))?;
    if let Err(e) = g.check().await {
        return Ok(Json(json!({ "ok": false, "msg": format!("No se pudo: {e}") })));
    }
    put_meta(&st, "gh_id", id).await?;
    put_meta(&st, "gh_scope", b.scope.trim()).await?;
    st.github.set(Some(Arc::new(g)));
    Ok(Json(json!({ "ok": true, "msg": "Listo: GitHub quedó habilitado. Cada persona conecta su cuenta desde Configuración → GitHub." })))
}

async fn gh_clear(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    del_meta(&st, "gh").await?;
    st.github.set(Github::from_env().map(Arc::new));
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct GgIn {
    client_id: String,
    /// vacío = conservar el secreto ya guardado
    #[serde(default)]
    secret: String,
    redirect: String,
}

async fn gg_save(State(st): State<AppState>, user: AuthUser, Json(b): Json<GgIn>) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let id = b.client_id.trim();
    let redirect = b.redirect.trim();
    let mut secret = b.secret.trim().to_string();
    if secret.is_empty() {
        secret = meta(&st, "gg_secret").await.and_then(|t| st.cred.open(&t)).unwrap_or_default();
    }
    if id.is_empty() || secret.is_empty() {
        return Ok(Json(json!({ "ok": false, "msg": "Faltan el ID de cliente y el secreto de la app de Google." })));
    }
    if !(redirect.starts_with("https://") || redirect.starts_with("http://localhost") || redirect.starts_with("http://127.0.0.1")) || !redirect.ends_with("/api/google/callback") {
        return Ok(Json(json!({ "ok": false, "msg": "La dirección de redirección tiene que ser https://TU-SERVIDOR/api/google/callback (Google solo acepta HTTPS, salvo localhost)." })));
    }
    let g = Google::from_parts(id, &secret, redirect).ok_or_else(|| AppError::bad("no pude preparar la conexión con Google"))?;
    if let Err(e) = g.check().await {
        return Ok(Json(json!({ "ok": false, "msg": format!("No se pudo: {e}") })));
    }
    put_meta(&st, "gg_id", id).await?;
    put_meta(&st, "gg_secret", &st.cred.seal(&secret)).await?;
    put_meta(&st, "gg_redirect", redirect).await?;
    st.google.set(Some(Arc::new(g)));
    Ok(Json(json!({ "ok": true, "msg": "Listo: Google Calendar quedó habilitado. Cada persona conecta su cuenta desde Configuración → Google." })))
}

async fn gg_clear(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    del_meta(&st, "gg").await?;
    st.google.set(Google::from_env().map(Arc::new));
    Ok(Json(json!({ "ok": true })))
}

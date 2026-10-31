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
    extract::State,
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
    api: String,
    http: reqwest::Client,
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
}

fn gh(st: &AppState) -> Result<std::sync::Arc<Github>> {
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
    .bind(token)
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

//! Sincronización opcional con Google Calendar (un solo sentido: del gestor al calendario).
//! Se activa con `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET` y `TM_GOOGLE_REDIRECT`
//! (`https://tu-servidor/api/google/callback`). Cada usuario conecta su cuenta y sus tareas
//! asignadas (o sin asignar, de sus equipos) con fecha de inicio o vencimiento aparecen como
//! eventos de día completo.
//! `TM_GOOGLE_AUTH_URL`, `TM_GOOGLE_TOKEN_URL` y `TM_GOOGLE_API` permiten apuntar a otra API (pruebas).
use crate::{
    auth::{sha256_hex, AuthUser},
    error::{AppError, Result},
    AppState,
};
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration as Days, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::{collections::HashMap, time::Duration};
use tm_core::Id;

const SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";

pub struct Google {
    client_id: String,
    secret: String,
    redirect: String,
    auth_url: String,
    token_url: String,
    api: String,
    http: reqwest::Client,
    /// Serializa las sincronizaciones para no crear eventos duplicados.
    lock: tokio::sync::Mutex<()>,
}

#[derive(Serialize, Deserialize)]
struct StateClaims {
    sub: Id,
    purpose: String,
    exp: usize,
}

#[derive(Deserialize)]
struct TokenReply {
    access_token: String,
    refresh_token: Option<String>,
}

impl Google {
    pub fn from_env() -> Option<Self> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let http = reqwest::Client::builder().timeout(Duration::from_secs(20)).build().ok()?;
        Some(Self {
            client_id: var("TM_GOOGLE_CLIENT_ID")?,
            secret: var("TM_GOOGLE_CLIENT_SECRET")?,
            redirect: var("TM_GOOGLE_REDIRECT")?,
            auth_url: var("TM_GOOGLE_AUTH_URL").unwrap_or_else(|| "https://accounts.google.com/o/oauth2/v2/auth".into()),
            token_url: var("TM_GOOGLE_TOKEN_URL").unwrap_or_else(|| "https://oauth2.googleapis.com/token".into()),
            api: var("TM_GOOGLE_API").unwrap_or_else(|| "https://www.googleapis.com/calendar/v3".into()),
            http,
            lock: Default::default(),
        })
    }

    fn auth_link(&self, state: &str) -> Result<String> {
        reqwest::Url::parse_with_params(
            &self.auth_url,
            [
                ("client_id", self.client_id.as_str()),
                ("redirect_uri", &self.redirect),
                ("response_type", "code"),
                ("scope", SCOPE),
                ("access_type", "offline"),
                ("prompt", "consent"),
                ("state", state),
            ],
        )
        .map(String::from)
        .map_err(|_| AppError::bad("TM_GOOGLE_AUTH_URL inválida"))
    }

    async fn token(&self, form: &[(&str, &str)]) -> std::result::Result<TokenReply, String> {
        let mut params = vec![("client_id", self.client_id.as_str()), ("client_secret", self.secret.as_str())];
        params.extend_from_slice(form);
        let r = self.http.post(&self.token_url).form(&params).send().await.map_err(|e| e.without_url().to_string())?;
        if !r.status().is_success() {
            let body: Value = r.json().await.unwrap_or_default();
            return Err(body["error"].as_str().unwrap_or("error de Google").to_string());
        }
        r.json().await.map_err(|e| e.without_url().to_string())
    }

    async fn access_token(&self, refresh: &str) -> std::result::Result<String, String> {
        Ok(self.token(&[("grant_type", "refresh_token"), ("refresh_token", refresh)]).await?.access_token)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/google", get(status).delete(unlink))
        .route("/api/google/link", post(link))
        .route("/api/google/sync", post(sync_now))
        .route("/api/google/callback", get(callback))
}

fn google(st: &AppState) -> Result<std::sync::Arc<Google>> {
    st.google.clone().ok_or_else(|| AppError::bad("Google Calendar no está configurado en este servidor"))
}

async fn is_linked(st: &AppState, user: Id) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM google_links WHERE user_id = ?").bind(user).fetch_one(&st.db).await? > 0)
}

async fn status(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM google_events WHERE user_id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(json!({ "available": st.google.is_some(), "linked": is_linked(&st, user.id).await?, "events": events })))
}

async fn link(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    let g = google(&st)?;
    let claims = StateClaims { sub: user.id, purpose: "gcal".into(), exp: (Utc::now() + Days::minutes(10)).timestamp() as usize };
    let state = encode(&Header::default(), &claims, &EncodingKey::from_secret(st.secret.as_bytes())).map_err(|_| AppError::bad("state"))?;
    Ok(Json(json!({ "url": g.auth_link(&state)? })))
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn page(ok: bool, msg: &str) -> Response {
    let icon = if ok { "✅" } else { "⚠️" };
    let body = format!("<!doctype html><meta charset=utf-8><title>Google Calendar</title><body style=\"font-family:sans-serif;text-align:center;margin-top:20vh\"><h2>{icon} {msg}</h2><p>Podés cerrar esta pestaña y volver a Rust TUI Manager.</p>");
    (if ok { StatusCode::OK } else { StatusCode::BAD_REQUEST }, [(header::CONTENT_TYPE, "text/html; charset=utf-8")], body).into_response()
}

/// Google redirige acá (sin cabecera de sesión): el `state` firmado identifica al usuario.
async fn callback(State(st): State<AppState>, Query(p): Query<CallbackParams>) -> Response {
    let Some(g) = st.google.clone() else { return page(false, "Google Calendar no está configurado") };
    if let Some(e) = p.error {
        return page(false, &format!("Google respondió: {}", e.chars().take(60).filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ' ').collect::<String>()));
    }
    let (Some(code), Some(state)) = (p.code, p.state) else { return page(false, "Faltan datos en la respuesta de Google") };
    let Ok(claims) = decode::<StateClaims>(&state, &DecodingKey::from_secret(st.secret.as_bytes()), &Validation::default()).map(|d| d.claims) else {
        return page(false, "El enlace venció: volvé a intentar desde Configuración");
    };
    if claims.purpose != "gcal" {
        return page(false, "Enlace inválido");
    }
    let reply = match g.token(&[("grant_type", "authorization_code"), ("code", &code), ("redirect_uri", &g.redirect)]).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("google: canje de código: {e}");
            return page(false, "Google rechazó el código");
        }
    };
    let Some(refresh) = reply.refresh_token else { return page(false, "Google no entregó permiso permanente: quitá el acceso de la app en tu cuenta y reintentá") };
    let saved = sqlx::query("INSERT INTO google_links (user_id, refresh_token, linked_at) VALUES (?, ?, ?) ON CONFLICT(user_id) DO UPDATE SET refresh_token = excluded.refresh_token, linked_at = excluded.linked_at")
        .bind(claims.sub)
        .bind(&refresh)
        .bind(Utc::now())
        .execute(&st.db)
        .await;
    if saved.is_err() {
        return page(false, "No se pudo guardar la conexión");
    }
    let (st2, uid) = (st.clone(), claims.sub);
    tokio::spawn(async move {
        if let Err(e) = sync_user(&st2, uid).await {
            tracing::warn!("google: primera sincronización: {e}");
        }
    });
    page(true, "Google Calendar conectado")
}

async fn sync_now(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    google(&st)?;
    if !is_linked(&st, user.id).await? {
        return Err(AppError::bad("Todavía no conectaste Google Calendar"));
    }
    st.user_limiter.hit(&format!("gsync:{}", user.id), 10, Duration::from_secs(60))?;
    let (created, updated, deleted) = sync_user(&st, user.id).await.map_err(|e| AppError::bad(&format!("Google: {e}")))?;
    Ok(Json(json!({ "created": created, "updated": updated, "deleted": deleted })))
}

async fn unlink(State(st): State<AppState>, user: AuthUser) -> Result<StatusCode> {
    if st.google.is_some() && is_linked(&st, user.id).await? {
        // se borran los eventos que creamos; si Google falla igual se desconecta
        let _ = purge(&st, user.id).await;
    }
    sqlx::query("DELETE FROM google_events WHERE user_id = ?").bind(user.id).execute(&st.db).await?;
    sqlx::query("DELETE FROM google_links WHERE user_id = ?").bind(user.id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

struct Want {
    task_id: Id,
    fingerprint: String,
    body: Value,
}

async fn wanted(st: &AppState, user: Id) -> Result<Vec<Want>> {
    let rows = sqlx::query(
        "SELECT t.id, t.title, t.description, t.start, t.due, p.name AS project, tm.name AS team FROM tasks t \
         JOIN projects p ON p.id = t.project_id JOIN teams tm ON tm.id = p.team_id \
         WHERE (t.assignee_id = ? OR (t.assignee_id IS NULL AND p.team_id IN (SELECT team_id FROM team_members WHERE user_id = ?))) AND t.status != 'done' AND (t.start IS NOT NULL OR t.due IS NOT NULL) LIMIT 2000",
    )
    .bind(user)
    .bind(user)
    .fetch_all(&st.db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            let (start, due): (Option<DateTime<Utc>>, Option<DateTime<Utc>>) = (r.get("start"), r.get("due"));
            let first = start.or(due).expect("filtrado por SQL").date_naive();
            let last = due.or(start).expect("filtrado por SQL").date_naive().max(first);
            let desc: String = r.get("description");
            let body = json!({
                "summary": r.get::<String, _>("title"),
                "description": format!("{} · {}\n{}", r.get::<String, _>("team"), r.get::<String, _>("project"), desc.chars().take(500).collect::<String>()).trim().to_string(),
                "start": { "date": first.to_string() },
                "end": { "date": (last + Days::days(1)).to_string() },
            });
            Want { task_id: r.get("id"), fingerprint: sha256_hex(&body.to_string()), body }
        })
        .collect())
}

fn gone(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE
}

/// Deja el calendario del usuario igual a sus tareas. Devuelve (creados, actualizados, borrados).
pub async fn sync_user(st: &AppState, user: Id) -> std::result::Result<(usize, usize, usize), String> {
    let g = st.google.clone().ok_or("Google no configurado")?;
    let _guard = g.lock.lock().await;
    let db_err = |e: sqlx::Error| e.to_string();
    let Some(refresh): Option<String> = sqlx::query_scalar("SELECT refresh_token FROM google_links WHERE user_id = ?").bind(user).fetch_optional(&st.db).await.map_err(db_err)? else {
        return Ok((0, 0, 0));
    };
    let want = wanted(st, user).await.map_err(|e| e.1)?;
    let stored: HashMap<Id, (String, String)> = sqlx::query("SELECT task_id, event_id, fingerprint FROM google_events WHERE user_id = ?")
        .bind(user)
        .fetch_all(&st.db)
        .await
        .map_err(db_err)?
        .iter()
        .map(|r| (r.get("task_id"), (r.get("event_id"), r.get("fingerprint"))))
        .collect();
    let wanted_ids: std::collections::HashSet<Id> = want.iter().map(|w| w.task_id).collect();
    let todo: Vec<&Want> = want.iter().filter(|w| stored.get(&w.task_id).is_none_or(|(_, fp)| *fp != w.fingerprint)).collect();
    let stale: Vec<(&Id, &(String, String))> = stored.iter().filter(|(id, _)| !wanted_ids.contains(id)).collect();
    if todo.is_empty() && stale.is_empty() {
        return Ok((0, 0, 0));
    }

    let access = match g.access_token(&refresh).await {
        Ok(a) => a,
        Err(e) if e == "invalid_grant" => {
            // el usuario revocó el acceso desde su cuenta de Google
            let _ = sqlx::query("DELETE FROM google_links WHERE user_id = ?").bind(user).execute(&st.db).await;
            let _ = sqlx::query("DELETE FROM google_events WHERE user_id = ?").bind(user).execute(&st.db).await;
            return Err("Google revocó el acceso: volvé a conectar el calendario".into());
        }
        Err(e) => return Err(e),
    };
    let events = format!("{}/calendars/primary/events", g.api);
    let (mut created, mut updated, mut deleted) = (0, 0, 0);
    let send = |e: reqwest::Error| e.without_url().to_string();

    for (task_id, (event_id, _)) in stale {
        let r = g.http.delete(format!("{events}/{event_id}")).bearer_auth(&access).send().await.map_err(send)?;
        if r.status().is_success() || gone(r.status()) {
            sqlx::query("DELETE FROM google_events WHERE user_id = ? AND task_id = ?").bind(user).bind(task_id).execute(&st.db).await.map_err(db_err)?;
            deleted += 1;
        }
    }
    for w in todo {
        let mut event_id = stored.get(&w.task_id).map(|(e, _)| e.clone());
        let mut was_update = event_id.is_some();
        if let Some(id) = &event_id {
            let r = g.http.put(format!("{events}/{id}")).bearer_auth(&access).json(&w.body).send().await.map_err(send)?;
            if gone(r.status()) {
                event_id = None; // lo borraron a mano en Google: se vuelve a crear
                was_update = false;
            } else if !r.status().is_success() {
                return Err(format!("Google respondió {} al actualizar", r.status()));
            }
        }
        if event_id.is_none() {
            let r = g.http.post(&events).bearer_auth(&access).json(&w.body).send().await.map_err(send)?;
            if !r.status().is_success() {
                return Err(format!("Google respondió {} al crear", r.status()));
            }
            let v: Value = r.json().await.map_err(send)?;
            event_id = v["id"].as_str().map(String::from);
        }
        let Some(event_id) = event_id else { return Err("Google no devolvió el id del evento".into()) };
        sqlx::query("INSERT INTO google_events (user_id, task_id, event_id, fingerprint) VALUES (?, ?, ?, ?) ON CONFLICT(user_id, task_id) DO UPDATE SET event_id = excluded.event_id, fingerprint = excluded.fingerprint")
            .bind(user)
            .bind(w.task_id)
            .bind(&event_id)
            .bind(&w.fingerprint)
            .execute(&st.db)
            .await
            .map_err(db_err)?;
        if was_update { updated += 1 } else { created += 1 }
    }
    Ok((created, updated, deleted))
}

/// Borra del calendario todos los eventos creados para este usuario.
async fn purge(st: &AppState, user: Id) -> std::result::Result<(), String> {
    let g = st.google.clone().ok_or("Google no configurado")?;
    let _guard = g.lock.lock().await;
    let refresh: Option<String> = sqlx::query_scalar("SELECT refresh_token FROM google_links WHERE user_id = ?").bind(user).fetch_optional(&st.db).await.map_err(|e| e.to_string())?;
    let Some(refresh) = refresh else { return Ok(()) };
    let access = g.access_token(&refresh).await?;
    let ids: Vec<String> = sqlx::query_scalar("SELECT event_id FROM google_events WHERE user_id = ?").bind(user).fetch_all(&st.db).await.map_err(|e| e.to_string())?;
    for id in ids {
        let _ = g.http.delete(format!("{}/calendars/primary/events/{id}", g.api)).bearer_auth(&access).send().await;
    }
    Ok(())
}

pub fn spawn_sync(st: AppState) {
    if st.google.is_none() {
        return;
    }
    let secs = std::env::var("TM_GOOGLE_SYNC_SECS").ok().and_then(|v| v.parse::<u64>().ok()).filter(|s| *s > 0).unwrap_or(60);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(secs)).await;
            let users: Vec<Id> = sqlx::query_scalar("SELECT user_id FROM google_links").fetch_all(&st.db).await.unwrap_or_default();
            for u in users {
                if let Err(e) = sync_user(&st, u).await {
                    tracing::warn!("google: usuario {u}: {e}");
                }
            }
        }
    });
}

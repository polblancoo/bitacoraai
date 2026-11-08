//! Integraciones del equipo: enlaces a herramientas externas (videollamada con Jitsi, pizarras, editores…) que se abren en una
//! pestaña propia dentro de la app o en una ventana nueva. Las configura un administrador; todo el equipo las usa.
//! La URL admite {sala} (un código difícil de adivinar, distinto por equipo), {equipo} y {usuario}.
use crate::{auth::AuthUser, error::{AppError, Result}, routes, AppState};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use std::{net::SocketAddr, time::Duration};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tm_core::{Id, Role};

const MAX_ITEMS: usize = 12;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/teams/{id}/integrations", get(list).put(save))
        .route("/api/teams/{id}/call-invites", get(invites).post(invite_new))
        .route("/api/teams/{id}/call-invites/{code}", delete(invite_revoke))
        .route("/api/teams/{id}/call-invites/{code}/email", post(invite_email))
        .route("/j/{code}", get(invite_join))
}

fn new_code() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::fill(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[derive(Deserialize)]
struct InviteIn {
    #[serde(default)]
    label: String,
    #[serde(default)]
    hours: i64,
}

/// Crea una sala propia y un código para invitar a alguien que no es del equipo. Entrar con el código no da acceso a nada más que a esa sala.
async fn invite_new(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<InviteIn>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Member).await?;
    let has: Option<i64> = sqlx::query_scalar("SELECT 1 FROM integrations WHERE team_id = ? AND url LIKE '%{sala}%' LIMIT 1").bind(team).fetch_optional(&st.db).await?;
    has.ok_or_else(|| AppError::bad("primero hay que configurar la videollamada (Configuración → Integraciones)"))?;
    let hours = if matches!(b.hours, 1 | 4 | 24 | 168) { b.hours } else { 24 };
    let label: String = b.label.chars().filter(|c| !c.is_control()).take(60).collect();
    let (code, room) = (new_code(), new_code()[..20].to_string());
    sqlx::query("INSERT INTO call_invites (code, team_id, room, created_by, label, expires_at) VALUES (?, ?, ?, ?, ?, datetime('now', ?))")
        .bind(&code).bind(team).bind(&room).bind(user.id).bind(label.trim()).bind(format!("+{hours} hours")).execute(&st.db).await?;
    let exp: String = sqlx::query_scalar("SELECT expires_at FROM call_invites WHERE code = ?").bind(&code).fetch_one(&st.db).await?;
    Ok(Json(json!({ "code": code, "room": room, "expires_at": exp, "email_available": st.mailer.is_some() })))
}

async fn invites(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let role = routes::require(&st, team, user.id, Role::Member).await?;
    let admin = role >= Role::Admin;
    let rows = sqlx::query("SELECT i.code, i.room, i.label, i.expires_at, i.uses, u.username FROM call_invites i JOIN users u ON u.id = i.created_by WHERE i.team_id = ? AND i.revoked = 0 AND i.expires_at > datetime('now') AND (? OR i.created_by = ?) ORDER BY i.created_at DESC LIMIT 50")
        .bind(team).bind(admin).bind(user.id).fetch_all(&st.db).await?;
    Ok(Json(json!({ "email_available": st.mailer.is_some(), "items": rows.iter().map(|r| json!({
        "code": r.get::<String, _>(0), "room": r.get::<String, _>(1), "label": r.get::<String, _>(2), "expires_at": r.get::<String, _>(3), "uses": r.get::<i64, _>(4), "by": r.get::<String, _>(5),
    })).collect::<Vec<_>>() })))
}

async fn invite_revoke(State(st): State<AppState>, user: AuthUser, Path((team, code)): Path<(Id, String)>) -> Result<Json<Value>> {
    user.human_only()?;
    let role = routes::require(&st, team, user.id, Role::Member).await?;
    let r = sqlx::query("UPDATE call_invites SET revoked = 1 WHERE code = ? AND team_id = ? AND (? OR created_by = ?)").bind(&code).bind(team).bind(role >= Role::Admin).bind(user.id).execute(&st.db).await?;
    if r.rows_affected() == 0 {
        return Err(AppError::forbidden());
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct EmailIn {
    to: String,
    #[serde(default)]
    origin: String,
}

async fn invite_email(State(st): State<AppState>, user: AuthUser, headers: HeaderMap, Path((team, code)): Path<(Id, String)>, Json(b): Json<EmailIn>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Member).await?;
    let mailer = st.mailer.get().ok_or_else(|| AppError::bad("este servidor no tiene el correo configurado: copiá el enlace y mandalo vos"))?;
    let to = b.to.trim().to_string();
    if to.len() > 120 || !to.contains('@') || to.chars().any(|c| c.is_whitespace() || c.is_control() || c == ',' || c == ';') {
        return Err(AppError::bad("escribí un solo email válido"));
    }
    // el enlace tiene que apuntar a este mismo servidor: no se acepta un origen distinto del que usó quien lo pide
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
    let origin = b.origin.trim_end_matches('/');
    let ok_origin = origin.strip_prefix("https://").or_else(|| origin.strip_prefix("http://")).is_some_and(|h| h == host && !host.is_empty());
    if !ok_origin {
        return Err(AppError::bad("dirección del servidor no válida"));
    }
    let row = sqlx::query("SELECT i.label, i.expires_at, t.name FROM call_invites i JOIN teams t ON t.id = i.team_id WHERE i.code = ? AND i.team_id = ? AND i.revoked = 0 AND i.expires_at > datetime('now') AND i.created_by = ?")
        .bind(&code).bind(team).bind(user.id).fetch_optional(&st.db).await?;
    let row = row.ok_or_else(|| AppError::bad("la invitación venció o fue revocada"))?;
    if st.email_limiter.hit(&user.id, crate::limits::EMAILS_PER_HOUR, Duration::from_secs(3600)).is_err() {
        return Err(AppError::bad("demasiados emails en esta hora, probá más tarde"));
    }
    let who: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await.unwrap_or_default();
    let (label, exp, tname): (String, String, String) = (row.get(0), row.get(1), row.get(2));
    let text = format!(
        "{who} te invita a una videollamada del equipo «{tname}»{}.\n\nEntrá desde acá, no hace falta cuenta ni instalar nada:\n{origin}/j/{code}\n\nEl enlace vale hasta {exp} (hora del servidor) y se puede usar desde el navegador, con cámara y micrófono.",
        if label.is_empty() { String::new() } else { format!(" ({label})") }
    );
    mailer.send(&to, &format!("[CrewHelm] Invitación a una videollamada de {who}"), text).await.map_err(|e| AppError::bad(&format!("no se pudo enviar: {e}")))?;
    Ok(Json(json!({ "ok": true })))
}

/// Entrada pública con el código: valida y manda a la sala. Sin cuenta; todo pedido cuenta contra el límite por IP.
async fn invite_join(State(st): State<AppState>, ConnectInfo(peer): ConnectInfo<SocketAddr>, headers: HeaderMap, Path(code): Path<String>) -> Response {
    let ip = crate::limits::client_ip(&headers, peer, st.trust_proxy);
    if st.ip_limiter.hit(&ip, 30, Duration::from_secs(60)).is_err() {
        return (StatusCode::TOO_MANY_REQUESTS, "demasiados intentos, esperá un minuto").into_response();
    }
    let gone = || (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], "Esta invitación no es válida o ya venció. Pedile a quien te invitó que te mande una nueva.").into_response();
    if code.len() != 32 || !code.chars().all(|c| c.is_ascii_hexdigit()) {
        return gone();
    }
    let Ok(Some(row)) = sqlx::query("SELECT i.room, i.team_id, t.name FROM call_invites i JOIN teams t ON t.id = i.team_id WHERE i.code = ? AND i.revoked = 0 AND i.expires_at > datetime('now')").bind(&code).fetch_optional(&st.db).await else { return gone() };
    let (room, team, tname): (String, Id, String) = (row.get(0), row.get(1), row.get(2));
    let Ok(Some(url)) = sqlx::query_scalar::<_, String>("SELECT url FROM integrations WHERE team_id = ? AND url LIKE '%{sala}%' ORDER BY position, id LIMIT 1").bind(team).fetch_optional(&st.db).await else { return gone() };
    let _ = sqlx::query("UPDATE call_invites SET uses = uses + 1 WHERE code = ?").bind(&code).execute(&st.db).await;
    let enc = |s: &str| s.bytes().map(|b| if b.is_ascii_alphanumeric() || b == b'-' { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>();
    let target = url.replace("{sala}", &format!("crewhelm-{room}")).replace("{equipo}", &enc(&tname.replace(' ', "-"))).replace("{usuario}", "Invitado");
    (StatusCode::FOUND, [(header::LOCATION, target), (header::REFERRER_POLICY, "no-referrer".to_string())]).into_response()
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

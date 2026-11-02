//! Chat del equipo: canal del equipo, canal de cada proyecto y mensajes directos, con conectados en tiempo real y envío
//! de archivos. Seguridad: solo por HTTPS (o desde la propia máquina), mensajes y archivos cifrados en la base con la
//! clave del servidor, archivos con tope de tamaño y tipos permitidos (siempre se descargan, nunca se ejecutan) y un
//! registro de auditoría de todo lo que pasa (quién, qué, cuándo, desde qué IP) que ven los administradores.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    limits::{self, max_len},
    routes::{self, ensure_member},
    AppState,
};
use axum::{
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::{net::SocketAddr, time::Duration};
use tm_core::{ChatFile, ChatMessage, Event, Id, Role};

const BODY_MAX: usize = 4000;
const FILE_MAX: usize = 5 * 1024 * 1024;
const PAGE: i64 = 60;
/// Tipos permitidos: imágenes, PDF, texto y documentos de oficina. Nada ejecutable, ni HTML, ni SVG, ni comprimidos.
const ALLOWED: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "pdf", "txt", "md", "csv", "json", "log", "docx", "xlsx", "pptx", "odt", "ods"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/chat/status", get(status))
        .route("/api/teams/{id}/chat", get(history).post(send))
        .route("/api/teams/{id}/chat/files", post(upload).layer(DefaultBodyLimit::max(FILE_MAX + 4096)))
        .route("/api/teams/{id}/presence", get(presence))
        .route("/api/teams/{id}/chat/audit", get(audit_list))
        .route("/api/chat/files/{id}", get(download))
}

/// ¿La conexión es segura? HTTPS propio, proxy de confianza que avisa HTTPS, o la propia máquina del servidor.
fn secure(st: &AppState, headers: &HeaderMap, peer: SocketAddr) -> bool {
    if std::env::var("TM_CHAT_ALLOW_HTTP").is_ok_and(|v| v == "1") || st.tls || headers.contains_key(crate::tls::HDR) {
        return true;
    }
    let ip = limits::client_ip(headers, peer, st.trust_proxy);
    if st.trust_proxy && headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()) == Some("https") {
        return true;
    }
    ip.is_loopback() && !st.trust_proxy
}

fn need_https() -> AppError {
    AppError(
        StatusCode::FORBIDDEN,
        "El chat solo funciona por una conexión segura (HTTPS). Abrí la app con https:// (el navegador va a avisar una vez que el certificado es propio: aceptalo) o pedile al administrador que ponga un certificado de la empresa.".into(),
    )
}

async fn audit(st: &AppState, headers: &HeaderMap, peer: SocketAddr, user: Id, team: Id, action: &str, detail: String) {
    let ip = limits::client_ip(headers, peer, st.trust_proxy).to_string();
    let _ = sqlx::query("INSERT INTO chat_audit (at, user_id, team_id, action, detail, ip) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(Utc::now())
        .bind(user)
        .bind(team)
        .bind(action)
        .bind(detail)
        .bind(ip)
        .execute(&st.db)
        .await;
}

async fn status(State(st): State<AppState>, ConnectInfo(peer): ConnectInfo<SocketAddr>, headers: HeaderMap, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    // Enlace a la misma pantalla por HTTPS (mismo equipo, puerto seguro) para ofrecerlo cuando se entró por HTTP
    let https_url = crate::tls::port().and_then(|p| {
        let host = headers.get("host")?.to_str().ok()?;
        let name = host.rsplit_once(':').filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit())).map_or(host, |(n, _)| n);
        Some(format!("https://{name}:{p}"))
    });
    Ok(Json(json!({ "secure": secure(&st, &headers, peer), "https_url": https_url })))
}

fn clean_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let n: String = base.chars().filter(|c| !c.is_control()).take(limits::ATTACHMENT_NAME_MAX).collect();
    let n = n.trim().to_string();
    if n.is_empty() { "archivo".into() } else { n }
}

fn ext_of(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// El contenido tiene que parecerse a lo que dice la extensión (las imágenes y el PDF se reconocen por sus primeros bytes).
fn magic_ok(ext: &str, data: &[u8]) -> bool {
    match ext {
        "png" => data.starts_with(&[0x89, b'P', b'N', b'G']),
        "jpg" | "jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "gif" => data.starts_with(b"GIF8"),
        "webp" => data.len() > 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP",
        "pdf" => data.starts_with(b"%PDF"),
        "docx" | "xlsx" | "pptx" | "odt" | "ods" => data.starts_with(b"PK"),
        _ => !data.contains(&0),
    }
}

#[derive(Deserialize)]
struct UploadQ {
    name: String,
}

async fn upload(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    user: AuthUser,
    Path(team): Path<Id>,
    Query(q): Query<UploadQ>,
    body: Bytes,
) -> Result<Json<Value>> {
    user.human_only()?;
    if !secure(&st, &headers, peer) {
        return Err(need_https());
    }
    ensure_member(&st, team, user.id).await?;
    let name = clean_name(&q.name);
    let ext = ext_of(&name);
    if body.is_empty() {
        return Err(AppError::bad("archivo vacío"));
    }
    if body.len() > FILE_MAX {
        return Err(AppError(StatusCode::PAYLOAD_TOO_LARGE, "máximo 5 MB por archivo".into()));
    }
    if !ALLOWED.contains(&ext.as_str()) || !magic_ok(&ext, &body) {
        audit(&st, &headers, peer, user.id, team, "archivo_rechazado", format!("{name} ({} bytes)", body.len())).await;
        return Err(AppError::bad("tipo de archivo no permitido (imágenes, PDF, texto y documentos de oficina)"));
    }
    st.user_limiter.hit(&format!("chat-file:{}", user.id), 30, Duration::from_secs(3600))?;
    let id = sqlx::query("INSERT INTO chat_files (team_id, uploader, name, size, data, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(team)
        .bind(user.id)
        .bind(&name)
        .bind(body.len() as i64)
        .bind(st.cred.seal_bytes(&body))
        .bind(Utc::now())
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    audit(&st, &headers, peer, user.id, team, "archivo_subido", format!("#{id} {name} ({} bytes)", body.len())).await;
    Ok(Json(json!({ "id": id, "name": name, "size": body.len() })))
}

#[derive(Deserialize)]
struct SendBody {
    #[serde(default)]
    body: String,
    project_id: Option<Id>,
    to: Option<Id>,
    file_id: Option<Id>,
}

async fn name_of(st: &AppState, user: Id) -> String {
    sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(user).fetch_one(&st.db).await.unwrap_or_default()
}

async fn send(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    user: AuthUser,
    Path(team): Path<Id>,
    Json(b): Json<SendBody>,
) -> Result<Json<ChatMessage>> {
    user.human_only()?;
    if !secure(&st, &headers, peer) {
        return Err(need_https());
    }
    ensure_member(&st, team, user.id).await?;
    let text = b.body.trim();
    if text.is_empty() && b.file_id.is_none() {
        return Err(AppError::bad("mensaje vacío"));
    }
    max_len("mensaje", text, BODY_MAX)?;
    st.user_limiter.hit(&format!("chat:{}", user.id), 120, Duration::from_secs(60))?;
    if let Some(p) = b.project_id {
        if routes::team_of_project(&st, p).await? != team {
            return Err(AppError::forbidden());
        }
    }
    if let Some(to) = b.to {
        if to == user.id || ensure_member(&st, team, to).await.is_err() {
            return Err(AppError::bad("esa persona no está en el equipo"));
        }
    }
    let mut file: Option<ChatFile> = None;
    if let Some(fid) = b.file_id {
        // el archivo tiene que ser de quien escribe, de este equipo y no estar ya adjunto a otro mensaje
        let row = sqlx::query("SELECT name, size FROM chat_files WHERE id = ? AND team_id = ? AND uploader = ? AND id NOT IN (SELECT file_id FROM chat_messages WHERE file_id IS NOT NULL)")
            .bind(fid)
            .bind(team)
            .bind(user.id)
            .fetch_optional(&st.db)
            .await?
            .ok_or_else(|| AppError::bad("archivo no válido"))?;
        file = Some(ChatFile { id: fid, name: row.get("name"), size: row.get("size") });
    }
    let now = Utc::now();
    let id = sqlx::query("INSERT INTO chat_messages (team_id, project_id, from_user, to_user, body, file_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(team)
        .bind(if b.to.is_some() { None } else { b.project_id })
        .bind(user.id)
        .bind(b.to)
        .bind(st.cred.seal(text))
        .bind(b.file_id)
        .bind(now)
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    let msg = ChatMessage {
        id,
        team_id: team,
        project_id: if b.to.is_some() { None } else { b.project_id },
        from_user: user.id,
        from: name_of(&st, user.id).await,
        to_user: b.to,
        body: text.to_string(),
        file,
        created_at: now,
    };
    let canal = match (b.to, b.project_id) {
        (Some(t), _) => format!("directo a #{t}"),
        (None, Some(p)) => format!("proyecto #{p}"),
        _ => "equipo".into(),
    };
    audit(&st, &headers, peer, user.id, team, "mensaje", format!("{canal}, {} caracteres{}", text.chars().count(), if b.file_id.is_some() { ", con archivo" } else { "" })).await;
    match b.to {
        Some(t) => {
            st.emit_to(t, Event::Chat(msg.clone()));
            st.emit_to(user.id, Event::Chat(msg.clone()));
        }
        None => st.emit(team, Event::Chat(msg.clone())),
    }
    Ok(Json(msg))
}

#[derive(Deserialize)]
struct HistQ {
    project: Option<Id>,
    with: Option<Id>,
    before: Option<Id>,
}

async fn history(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    user: AuthUser,
    Path(team): Path<Id>,
    Query(q): Query<HistQ>,
) -> Result<Json<Vec<ChatMessage>>> {
    user.human_only()?;
    if !secure(&st, &headers, peer) {
        return Err(need_https());
    }
    ensure_member(&st, team, user.id).await?;
    let before = q.before.unwrap_or(i64::MAX);
    let base = "SELECT m.id, m.team_id, m.project_id, m.from_user, u.username AS from_name, m.to_user, m.body, m.file_id, f.name AS fname, f.size AS fsize, m.created_at \
                FROM chat_messages m JOIN users u ON u.id = m.from_user LEFT JOIN chat_files f ON f.id = m.file_id WHERE m.team_id = ? AND m.id < ? AND ";
    let rows = match (q.with, q.project) {
        (Some(w), _) => sqlx::query(sqlx::AssertSqlSafe(format!("{base} m.to_user IS NOT NULL AND ((m.from_user = ? AND m.to_user = ?) OR (m.from_user = ? AND m.to_user = ?)) ORDER BY m.id DESC LIMIT ?")))
            .bind(team).bind(before).bind(user.id).bind(w).bind(w).bind(user.id).bind(PAGE).fetch_all(&st.db).await?,
        (None, Some(p)) => sqlx::query(sqlx::AssertSqlSafe(format!("{base} m.to_user IS NULL AND m.project_id = ? ORDER BY m.id DESC LIMIT ?")))
            .bind(team).bind(before).bind(p).bind(PAGE).fetch_all(&st.db).await?,
        _ => sqlx::query(sqlx::AssertSqlSafe(format!("{base} m.to_user IS NULL AND m.project_id IS NULL ORDER BY m.id DESC LIMIT ?")))
            .bind(team).bind(before).bind(PAGE).fetch_all(&st.db).await?,
    };
    let mut out: Vec<ChatMessage> = rows
        .iter()
        .map(|r| {
            let sealed: String = r.get("body");
            ChatMessage {
                id: r.get("id"),
                team_id: r.get("team_id"),
                project_id: r.get("project_id"),
                from_user: r.get("from_user"),
                from: r.get("from_name"),
                to_user: r.get("to_user"),
                body: st.cred.open(&sealed).unwrap_or_else(|| "(mensaje que no se pudo descifrar)".into()),
                file: r.get::<Option<Id>, _>("file_id").map(|id| ChatFile { id, name: r.get("fname"), size: r.get("fsize") }),
                created_at: r.get::<DateTime<Utc>, _>("created_at"),
            }
        })
        .collect();
    out.reverse();
    audit(&st, &headers, peer, user.id, team, "leyo_historial", match (q.with, q.project) { (Some(w), _) => format!("directo con #{w}"), (None, Some(p)) => format!("proyecto #{p}"), _ => "equipo".into() }).await;
    Ok(Json(out))
}

async fn presence(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    ensure_member(&st, team, user.id).await?;
    let rows = sqlx::query("SELECT u.id, u.username FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND u.id NOT IN (SELECT user_id FROM api_tokens) ORDER BY u.username")
        .bind(team)
        .fetch_all(&st.db)
        .await?;
    let people: Vec<Value> = rows
        .iter()
        .map(|r| {
            let id: Id = r.get("id");
            json!({ "id": id, "username": r.get::<String, _>("username"), "online": st.ws_conns.is_online(id), "me": id == user.id })
        })
        .collect();
    Ok(Json(json!({ "people": people })))
}

async fn download(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    user: AuthUser,
    Path(id): Path<Id>,
) -> Result<Response> {
    user.human_only()?;
    if !secure(&st, &headers, peer) {
        return Err(need_https());
    }
    let f = sqlx::query("SELECT team_id, name, data FROM chat_files WHERE id = ?").bind(id).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)?;
    let team: Id = f.get("team_id");
    ensure_member(&st, team, user.id).await?;
    // un archivo de un mensaje directo solo lo ven las dos personas
    let dm: Option<(Id, Id)> = sqlx::query_as("SELECT from_user, to_user FROM chat_messages WHERE file_id = ? AND to_user IS NOT NULL").bind(id).fetch_optional(&st.db).await?;
    if let Some((a, b)) = dm {
        if user.id != a && user.id != b {
            audit(&st, &headers, peer, user.id, team, "descarga_denegada", format!("archivo #{id}")).await;
            return Err(AppError::forbidden());
        }
    }
    let name: String = f.get("name");
    let data = st.cred.open_bytes(&f.get::<Vec<u8>, _>("data")).ok_or_else(|| AppError::bad("no se pudo descifrar el archivo"))?;
    audit(&st, &headers, peer, user.id, team, "archivo_descargado", format!("#{id} {name}")).await;
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') { c } else { '_' }).collect();
    let mut resp = data.into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(resp)
}

/// Registro de auditoría del chat del equipo: solo administradores y dueños.
async fn audit_list(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    let rows = sqlx::query("SELECT a.at, a.user_id, (SELECT username FROM users WHERE id = a.user_id) AS who, a.action, a.detail, a.ip FROM chat_audit a WHERE a.team_id = ? ORDER BY a.id DESC LIMIT 300")
        .bind(team)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(json!(rows
        .iter()
        .map(|r| json!({ "at": r.get::<String, _>("at"), "who": r.get::<Option<String>, _>("who"), "action": r.get::<String, _>("action"), "detail": r.get::<String, _>("detail"), "ip": r.get::<String, _>("ip") }))
        .collect::<Vec<_>>())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipos_y_contenido_de_archivos() {
        assert!(ALLOWED.contains(&ext_of("foto.PNG").as_str()));
        assert!(!ALLOWED.contains(&ext_of("run.exe").as_str()));
        assert!(!ALLOWED.contains(&ext_of("pagina.html").as_str()));
        assert!(!ALLOWED.contains(&ext_of("dibujo.svg").as_str()));
        assert!(magic_ok("png", &[0x89, b'P', b'N', b'G', 1]));
        assert!(!magic_ok("png", b"<script>"));
        assert!(!magic_ok("txt", b"abc\0def"));
        assert!(magic_ok("pdf", b"%PDF-1.7"));
    }
}

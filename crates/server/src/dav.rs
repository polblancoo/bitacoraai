//! Carpeta de red (WebDAV): cada proyecto se puede montar como una unidad en Windows, macOS o Linux
//! (`/dav/<id del proyecto>/`) y guardar ahí desde cualquier programa (Photoshop, Premiere, Word, DaVinci…).
//! Muestra `<carpeta del proyecto>/.bitacora/` con la misma regla de siempre: todos leen, cada persona escribe solo
//! en su carpeta y `_tareas/<n>/final/` es solo lectura. Antes de pisar o borrar un archivo se guarda la versión anterior.
use crate::{auth, error::Result as ApiResult, flow, limits, routes, taskfiles, AppState};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header, HeaderValue, Method, Request, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use base64::Engine;
use dav_server::{fakels::FakeLs, localfs::LocalFs, DavHandler};
use sqlx::Row;
use std::{net::SocketAddr, path::{Path, PathBuf}, time::Duration};
use tm_core::{Id, Role};

/// Tope por archivo (el navegador sigue limitado a 25 MB; acá el archivo va en flujo, no entra entero en memoria).
const FILE_MAX: u64 = 8 * 1024 * 1024 * 1024;
/// Versiones anteriores que se guardan por archivo.
pub const KEEP_VERSIONS: usize = 20;

pub fn router() -> Router<AppState> {
    Router::new().route("/dav", any(handle)).route("/dav/", any(handle)).route("/dav/{*rest}", any(handle))
}

fn plain(code: StatusCode, msg: &str) -> Response {
    let mut r = Response::new(Body::from(msg.to_string()));
    *r.status_mut() = code;
    r
}

fn unauthorized() -> Response {
    let mut r = plain(StatusCode::UNAUTHORIZED, "Hace falta usuario y contraseña de BitacoraAI");
    r.headers_mut().insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic realm=\"BitacoraAI\", charset=\"UTF-8\""));
    r
}

fn pct_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Ruta dentro del proyecto (ya decodificada, en segmentos) de `/dav/<id>/a/b`; None si es otro proyecto o es inválida.
fn segments_of(path: &str, project: Id) -> Option<Vec<String>> {
    let rest = path.strip_prefix("/dav/")?;
    let mut it = rest.split('/');
    if it.next()?.parse::<Id>().ok()? != project {
        return None;
    }
    let mut out = Vec::new();
    for s in it.filter(|s| !s.is_empty()) {
        let d = pct_decode(s)?;
        if d == ".." || d == "." || d.contains(['\0', '\\', '/']) {
            return None;
        }
        out.push(d);
    }
    Some(out)
}

fn url_path(u: &str) -> &str {
    let u = u.trim();
    match u.find("://") {
        Some(i) => u[i + 3..].find('/').map(|j| &u[i + 3 + j..]).unwrap_or("/"),
        None => u,
    }
}

struct Who {
    id: Id,
    name: String,
}

async fn authenticate(st: &AppState, peer: SocketAddr, h: &axum::http::HeaderMap) -> Result<Who, Response> {
    let Some(b64) = h.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Basic ")) else { return Err(unauthorized()) };
    let Some((user, pass)) = base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok().and_then(|b| String::from_utf8(b).ok()).and_then(|s| s.split_once(':').map(|(u, p)| (u.to_string(), p.to_string()))) else { return Err(unauthorized()) };
    if user.len() > 64 || pass.len() > 128 {
        return Err(unauthorized());
    }
    let key = user.trim().to_lowercase();
    let window = Duration::from_secs(15 * 60);
    if st.user_limiter.blocked(&key, 5, window) {
        return Err(plain(StatusCode::TOO_MANY_REQUESTS, "Demasiados intentos: esperá unos minutos"));
    }
    let ip = limits::client_ip(h, peer, st.trust_proxy);
    let row = sqlx::query("SELECT id, username, password_hash, is_bot FROM users WHERE username = ?").bind(user.trim()).fetch_optional(&st.db).await.ok().flatten();
    let ok = match &row {
        Some(r) => {
            let uid: Id = r.get("id");
            let by_app = crate::twofa::dav_token_ok(st, uid, &pass).await;
            // con verificación en dos pasos la contraseña sola no entra: hace falta la «contraseña de aplicación»
            by_app || (!crate::twofa::enabled(st, uid).await && auth::verify_password(&pass, &r.get::<String, _>("password_hash")))
        }
        None => { auth::dummy_verify(&pass); false }
    };
    let Some(r) = row.filter(|_| ok) else {
        let _ = st.user_limiter.hit(&key, 5, window);
        let _ = st.ip_limiter.hit(&ip, 20, Duration::from_secs(60));
        return Err(unauthorized());
    };
    st.user_limiter.reset(&key);
    if r.get::<i64, _>("is_bot") != 0 {
        return Err(plain(StatusCode::FORBIDDEN, "Las cuentas de IA no usan la carpeta de red"));
    }
    Ok(Who { id: r.get("id"), name: r.get("username") })
}

/// Guarda la versión anterior de un archivo (si existe) en `.versions/<ruta>/<fecha>` y deja solo las últimas.
pub fn archive(root: &Path, segs: &[String]) {
    let file: PathBuf = segs.iter().fold(root.to_path_buf(), |p, s| p.join(s));
    let (Ok(real), Ok(base)) = (std::fs::canonicalize(&file), std::fs::canonicalize(root)) else { return };
    if !real.starts_with(&base) || !real.is_file() {
        return;
    }
    let dir = segs.iter().fold(base.join(".versions"), |p, s| p.join(s));
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let _ = std::fs::copy(&real, dir.join(format!("{ts}")));
    let mut vs: Vec<_> = std::fs::read_dir(&dir).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    vs.sort_by_key(|n: &String| n.parse::<u128>().unwrap_or(0));
    while vs.len() > KEEP_VERSIONS {
        let _ = std::fs::remove_file(dir.join(vs.remove(0)));
    }
}

fn is_write(m: &Method) -> bool {
    matches!(m.as_str(), "PUT" | "DELETE" | "MKCOL" | "MOVE" | "COPY" | "PROPPATCH" | "LOCK" | "UNLOCK" | "PATCH")
}

async fn handle(State(st): State<AppState>, ConnectInfo(peer): ConnectInfo<SocketAddr>, req: Request<Body>) -> Response {
    run(&st, peer, req).await.unwrap_or_else(|r| r)
}

async fn run(st: &AppState, peer: SocketAddr, req: Request<Body>) -> Result<Response, Response> {
    let who = authenticate(st, peer, &req.headers().clone()).await?;
    let path = req.uri().path().to_string();
    let project: Id = path.strip_prefix("/dav/").and_then(|r| r.split('/').next()).and_then(|p| p.parse().ok()).ok_or_else(|| plain(StatusCode::NOT_FOUND, "La dirección es /dav/<número de proyecto>/ (la ves en Configuración → Equipo y proyecto → Carpeta de red)"))?;
    let team = routes::team_of_project(st, project).await.map_err(|_| plain(StatusCode::NOT_FOUND, "No existe ese proyecto"))?;
    let role = routes::role_of(st, team, who.id).await.map_err(|_| plain(StatusCode::FORBIDDEN, "No sos parte de ese equipo"))?;
    let write = is_write(req.method());
    if role < Role::Viewer || (write && role < Role::Member) {
        return Err(plain(StatusCode::FORBIDDEN, "Tu rol no permite escribir"));
    }
    let wd: Option<String> = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await.ok().flatten();
    let wd = wd.filter(|w| !w.trim().is_empty()).and_then(|w| std::fs::canonicalize(w).ok()).ok_or_else(|| plain(StatusCode::NOT_FOUND, "La carpeta del proyecto no está en la máquina del servidor"))?;
    let root = crate::taskfiles::data_dir(&wd);
    let mine = flow::safe_name(&who.name);
    if role >= Role::Member {
        let _ = std::fs::create_dir_all(root.join(&mine));
        let gi = root.join(".gitignore");
        if !gi.exists() { let _ = std::fs::write(gi, "*\n"); }
    }
    if !root.is_dir() {
        return Err(plain(StatusCode::NOT_FOUND, "Todavía no hay archivos en este proyecto"));
    }

    let segs = segments_of(&path, project).ok_or_else(|| plain(StatusCode::BAD_REQUEST, "ruta no válida"))?;
    // lo oculto y lo interno (versiones, .gitignore) no se toca por acá
    if segs.first().is_some_and(|f| f.starts_with('.')) {
        return Err(plain(StatusCode::FORBIDDEN, "ruta reservada"));
    }
    let method = req.method().clone();
    // escribir: solo dentro de la carpeta propia; copiar solo exige que el destino sea propio
    let dest = if matches!(method.as_str(), "MOVE" | "COPY") {
        let d = req.headers().get("destination").and_then(|v| v.to_str().ok()).ok_or_else(|| plain(StatusCode::BAD_REQUEST, "falta el destino"))?;
        Some(segments_of(&pct_path(url_path(d)), project).ok_or_else(|| plain(StatusCode::FORBIDDEN, "el destino debe estar en tu carpeta del mismo proyecto"))?)
    } else { None };
    if write {
        let own = |s: &[String]| s.first().is_some_and(|f| *f == mine) && s.len() >= 1;
        let src_ok = method.as_str() == "COPY" || own(&segs);
        let dst_ok = dest.as_ref().is_none_or(|d| own(d) && !d.first().is_some_and(|f| f.starts_with('.')));
        // la carpeta propia en sí no se borra ni se mueve
        let whole = segs.len() == 1 && matches!(method.as_str(), "DELETE" | "MOVE");
        if !src_ok || !dst_ok || whole {
            return Err(plain(StatusCode::FORBIDDEN, "Solo podés escribir en tu carpeta"));
        }
        for s in std::iter::once(&segs).chain(dest.iter()) {
            if let Some(name) = s.last() {
                let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
                if matches!(method.as_str(), "PUT" | "MOVE" | "COPY") && name.contains('.') && taskfiles::BLOCKED.contains(&ext.as_str()) {
                    return Err(plain(StatusCode::FORBIDDEN, "No se suben programas ejecutables a la carpeta del equipo"));
                }
            }
        }
        if method == Method::PUT {
            let len = req.headers().get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            if len > FILE_MAX {
                return Err(plain(StatusCode::PAYLOAD_TOO_LARGE, "El archivo supera los 8 GB"));
            }
        }
        if matches!(method.as_str(), "PUT" | "DELETE" | "MOVE") {
            archive(&root, &segs);
        }
        if method.as_str() == "MOVE" {
            if let Some(d) = &dest { archive(&root, d); }
        }
    }

    let handler = DavHandler::builder().filesystem(LocalFs::new(&root, true, false, true)).locksystem(FakeLs::new()).strip_prefix(format!("/dav/{project}")).hide_symlinks(true).build_handler();
    let resp = handler.handle(req).await;
    let (parts, body) = resp.into_parts();
    if write && matches!(parts.status, StatusCode::CREATED | StatusCode::NO_CONTENT | StatusCode::OK) && matches!(method.as_str(), "PUT" | "DELETE" | "MOVE" | "COPY" | "MKCOL") {
        let what = match method.as_str() { "PUT" => "subió", "DELETE" => "borró", "MOVE" => "movió", "COPY" => "copió", _ => "creó la carpeta" };
        crate::audit::record(st, team, Some(project), None, who.id, "carpeta_de_red", &format!("{what} «{}» por la carpeta de red", segs.join("/"))).await;
    }
    Ok(Response::from_parts(parts, Body::new(body)))
}

/// Deja el camino como lo espera `segments_of`: `/dav/...` aunque el destino venga con otro prefijo de servidor.
fn pct_path(p: &str) -> String {
    match p.find("/dav/") { Some(i) => p[i..].to_string(), None => p.to_string() }
}

#[allow(dead_code)]
fn _unused(_: ApiResult<()>) {}

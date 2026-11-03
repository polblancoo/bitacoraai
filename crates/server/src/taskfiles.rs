//! Carpeta de cada tarea: `<carpeta del proyecto>/.crewhelm/tareas/<id>/<usuario>/` y `final/`.
//! Al estilo Linux: todos los del equipo leen; cada uno escribe solo en su carpeta; `final/` solo se llena al aprobar.
use crate::{auth::AuthUser, error::{AppError, Result}, flow, routes, AppState};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tm_core::{Id, Role};

const FILE_MAX: usize = 25 * 1024 * 1024;
const ENTRIES_MAX: usize = 500;
/// Lo que nunca se sube a mano: programas que alguien podría ejecutar después en el servidor.
const BLOCKED: &[&str] = &["exe", "dll", "bat", "cmd", "msi", "com", "scr", "ps1", "vbs", "jar", "so", "dylib", "apk", "app"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/tasks/{id}/folder", get(list))
        .route("/api/tasks/{id}/folder/file", get(download))
        .route("/api/tasks/{id}/folder/upload", post(upload).layer(DefaultBodyLimit::max(FILE_MAX + 4096)))
        .route("/api/tasks/{id}/publish", post(publish))
}

#[derive(Deserialize)]
struct Q {
    #[serde(default)]
    path: String,
    #[serde(default)]
    name: String,
}

/// Carpeta de la tarea en la máquina del servidor (None si el proyecto no tiene carpeta o no está acá).
async fn task_dir(st: &AppState, task: Id) -> Option<PathBuf> {
    let wd: Option<String> = sqlx::query_scalar("SELECT p.work_dir FROM projects p JOIN tasks t ON t.project_id = p.id WHERE t.id = ?").bind(task).fetch_optional(&st.db).await.ok().flatten();
    let wd = wd.filter(|w| !w.trim().is_empty())?;
    let base = std::fs::canonicalize(&wd).ok()?;
    Some(base.join(".crewhelm").join("tareas").join(task.to_string()))
}

/// Une partes de ruta ya validadas: sin «..», sin vacíos ni NUL, y el resultado (con enlaces resueltos) no sale de `root`.
fn inside(root: &std::path::Path, rel: &str) -> Result<PathBuf> {
    let mut p = root.to_path_buf();
    for part in rel.split('/').filter(|s| !s.is_empty()) {
        if part == ".." || part == "." || part.contains('\0') || part.contains('\\') {
            return Err(AppError::bad("ruta no permitida"));
        }
        p.push(part);
    }
    let real = std::fs::canonicalize(&p).map_err(|_| AppError::bad("no existe"))?;
    let root = std::fs::canonicalize(root).map_err(|_| AppError::bad("no existe"))?;
    if !real.starts_with(&root) {
        return Err(AppError::bad("ruta no permitida"));
    }
    Ok(real)
}

async fn list(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Query(q): Query<Q>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_task(&st, id).await?;
    routes::require(&st, team, user.id, Role::Viewer).await?;
    let Some(root) = task_dir(&st, id).await.filter(|d| d.is_dir()) else {
        let has = sqlx::query_scalar::<_, String>("SELECT p.work_dir FROM projects p JOIN tasks t ON t.project_id = p.id WHERE t.id = ?").bind(id).fetch_optional(&st.db).await.ok().flatten().is_some_and(|w| !w.trim().is_empty());
        return Ok(Json(json!({ "exists": false, "entries": [], "path": "", "reason": if has { "Todavía no hay archivos de esta tarea (o la carpeta del proyecto no está en la máquina del servidor)." } else { "El proyecto no tiene carpeta de trabajo." } })));
    };
    let dir = inside(&root, &q.path)?;
    let mut items: Vec<(bool, String, u64)> = std::fs::read_dir(&dir).map_err(|_| AppError::bad("no es una carpeta"))?
        .flatten()
        .filter_map(|e| {
            let real = std::fs::canonicalize(e.path()).ok().filter(|p| p.starts_with(std::fs::canonicalize(&root).unwrap_or_default()))?;
            let md = std::fs::metadata(&real).ok()?;
            Some((md.is_dir(), e.file_name().to_string_lossy().into_owned(), md.len()))
        })
        .collect();
    items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    let truncated = items.len() > ENTRIES_MAX;
    items.truncate(ENTRIES_MAX);
    Ok(Json(json!({
        "exists": true, "path": q.path.trim_matches('/'), "truncated": truncated,
        "entries": items.iter().map(|(d, n, s)| json!({ "name": n, "dir": d, "size": s })).collect::<Vec<_>>(),
    })))
}

async fn download(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Query(q): Query<Q>) -> Result<Response> {
    user.human_only()?;
    let team = routes::team_of_task(&st, id).await?;
    routes::require(&st, team, user.id, Role::Viewer).await?;
    let root = task_dir(&st, id).await.ok_or_else(|| AppError::bad("la tarea no tiene carpeta"))?;
    let file = inside(&root, &q.path)?;
    let md = std::fs::metadata(&file).map_err(|_| AppError::bad("no existe"))?;
    if !md.is_file() {
        return Err(AppError::bad("no es un archivo"));
    }
    if md.len() as usize > FILE_MAX {
        return Err(AppError::bad("el archivo supera los 25 MB"));
    }
    let data = std::fs::read(&file).map_err(|_| AppError::bad("no se pudo leer"))?;
    let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') { c } else { '_' }).collect();
    let mut resp = Bytes::from(data).into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(resp)
}

/// Sube un archivo a la carpeta propia de quien lo manda (para quien trabaja desde otra máquina o con su IA local).
async fn upload(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Query(q): Query<Q>, body: Bytes) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_task(&st, id).await?;
    routes::require(&st, team, user.id, Role::Member).await?;
    let name: String = q.name.rsplit(['/', '\\']).next().unwrap_or("").chars().map(|c| if c.is_control() { '_' } else { c }).collect();
    let name = name.trim().to_string();
    if name.is_empty() || name.starts_with('.') || name.len() > 120 {
        return Err(AppError::bad("nombre de archivo no válido"));
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    if name.contains('.') && BLOCKED.contains(&ext.as_str()) {
        return Err(AppError::bad("no se suben programas ejecutables a la carpeta de la tarea"));
    }
    if body.is_empty() {
        return Err(AppError::bad("archivo vacío"));
    }
    if body.len() > FILE_MAX {
        return Err(AppError(axum::http::StatusCode::PAYLOAD_TOO_LARGE, "máximo 25 MB por archivo".into()));
    }
    let root = task_dir(&st, id).await.ok_or_else(|| AppError::bad("el proyecto no tiene carpeta de trabajo en la máquina del servidor"))?;
    let me = crate::notify::username_of(&st, user.id).await;
    let dir = root.join(flow::safe_name(&me));
    std::fs::create_dir_all(&dir).map_err(|_| AppError::bad("no se pudo crear la carpeta"))?;
    let _ = std::fs::create_dir_all(root.join("final"));
    let gi = root.parent().and_then(|p| p.parent()).map(|p| p.join(".gitignore"));
    if let Some(gi) = gi.filter(|g| !g.exists()) { let _ = std::fs::write(gi, "*\n"); }
    std::fs::write(dir.join(&name), &body).map_err(|_| AppError::bad("no se pudo guardar"))?;
    crate::extras::log_task(&st, id, user.id, format!("subió «{name}» a su carpeta de la tarea")).await;
    Ok(Json(json!({ "ok": true, "path": format!("{}/{}", flow::safe_name(&me), name) })))
}

/// Tarea suelta (sin flujo): una persona (quien la creó o un admin) aprueba, el trabajo del responsable pasa a `final/` y queda Hecha.
async fn publish(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<Json<tm_core::Task>> {
    user.human_only()?;
    let team = routes::team_of_task(&st, id).await?;
    let role = routes::require(&st, team, user.id, Role::Member).await?;
    let t = routes::fetch_task(&st, id).await?;
    if t.created_by != Some(user.id) && role < Role::Admin {
        return Err(AppError::forbidden());
    }
    let flow_author = flow::last_work_author(&st, id).await;
    if flow::has_active(&st, id).await && !(t.waiting && flow_author.is_some() && flow::on_last_stage(&st, id).await) {
        return Err(AppError::bad("la tarea tiene un flujo en marcha: se aprueba desde el flujo, cuando llegue a la última etapa"));
    }
    let author = if let Some(a) = flow_author { a } else { match t.assignee_id { Some(a) => crate::notify::username_of(&st, a).await, None => crate::notify::username_of(&st, user.id).await } };
    let who = crate::notify::username_of(&st, user.id).await;
    flow::publish_final(&st, team, id, user.id, &author).await;
    Ok(Json(flow::finish(&st, team, id, user.id, format!("✅ {who} aprobó y publicó el trabajo de {author} en `.crewhelm/tareas/{id}/final/`.")).await?))
}

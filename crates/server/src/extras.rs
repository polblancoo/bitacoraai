//! Menciones, recordatorios de vencimiento, tareas recurrentes, adjuntos, duplicar proyecto,
//! exportación y registro de actividad.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    limits::{self, max_len, too_many},
    routes::{count, ensure_member, ensure_writer, fetch_task, require, role_of, team_of_project, team_of_task},
    AppState,
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Months, Utc};
use serde::Deserialize;
use sqlx::Row;
use std::time::Duration;
use tm_core::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/tasks/{id}/attachments",
            get(list_attachments).post(upload_attachment).layer(DefaultBodyLimit::max(limits::MAX_ATTACHMENT_BYTES + 4096)),
        )
        .route("/api/attachments/{id}", get(download_attachment).delete(delete_attachment))
        .route("/api/projects/{id}/duplicate", post(duplicate_project))
        .route("/api/projects/{id}/export", get(export_project))
        .route("/api/projects/{id}/activity", get(list_activity))
}

// ---------- recurrencia ----------

pub fn valid_recurrence(v: Option<&str>) -> Result<Option<String>> {
    match v.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(r @ ("daily" | "weekly" | "monthly")) => Ok(Some(r.into())),
        Some(_) => Err(AppError::bad("repetición inválida: daily, weekly o monthly")),
    }
}

fn shift(d: DateTime<Utc>, rule: &str) -> DateTime<Utc> {
    match rule {
        "daily" => d + chrono::Duration::days(1),
        "weekly" => d + chrono::Duration::days(7),
        _ => d.checked_add_months(Months::new(1)).unwrap_or(d + chrono::Duration::days(30)),
    }
}

/// Crea la próxima ocurrencia de una tarea recurrente que acaba de completarse.
async fn spawn_next(st: &AppState, team: Id, t: &Task) {
    let Some(rule) = t.recurrence.as_deref() else { return };
    let Ok(n) = count(st, "SELECT COUNT(*) FROM tasks WHERE project_id = ?", t.project_id).await else { return };
    if n >= limits::MAX_TASKS_PER_PROJECT {
        return;
    }
    let now = Utc::now();
    let due = shift(t.due.unwrap_or(now), rule);
    let start = t.start.map(|s| shift(s, rule));
    let Ok(r) = sqlx::query(
        "INSERT INTO tasks (project_id, title, description, priority, assignee_id, due, start, recurrence, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(t.project_id)
    .bind(&t.title)
    .bind(&t.description)
    .bind(t.priority)
    .bind(t.assignee_id)
    .bind(due)
    .bind(start)
    .bind(rule)
    .bind(now)
    .bind(now)
    .execute(&st.db)
    .await
    else {
        return;
    };
    let id = r.last_insert_rowid();
    let _ = sqlx::query("INSERT INTO task_labels (task_id, label_id) SELECT ?, label_id FROM task_labels WHERE task_id = ?")
        .bind(id)
        .bind(t.id)
        .execute(&st.db)
        .await;
    // las subtareas se copian pendientes
    let _ = sqlx::query(
        "INSERT INTO tasks (project_id, title, description, priority, parent_id, created_at, updated_at) \
         SELECT project_id, title, description, priority, ?, ?, ? FROM tasks WHERE parent_id = ?",
    )
    .bind(id)
    .bind(now)
    .bind(now)
    .bind(t.id)
    .execute(&st.db)
    .await;
    if let Ok(new) = fetch_task(st, id).await {
        st.emit(team, Event::TaskCreated(new));
    }
    log_project(st, t.project_id, t.assignee_id.unwrap_or(0), format!("se repitió «{}» ({})", t.title, rule_label(rule))).await;
}

fn rule_label(r: &str) -> &'static str {
    match r {
        "daily" => "cada día",
        "weekly" => "cada semana",
        _ => "cada mes",
    }
}

/// Efectos posteriores a editar una tarea: actividad y recurrencia.
pub async fn after_update(st: &AppState, team: Id, actor: Id, cur: &Task, t: &Task) {
    if t.parent_id.is_some() {
        return;
    }
    if cur.status != t.status {
        log(st, t.project_id, actor, format!("movió «{}» a {}", t.title, t.status.label())).await;
        if t.status == Status::Done && t.recurrence.is_some() {
            spawn_next(st, team, t).await;
        }
    }
    if cur.assignee_id != t.assignee_id {
        let text = match &t.assignee {
            Some(a) => format!("asignó «{}» a {a}", t.title),
            None => format!("dejó «{}» sin responsable", t.title),
        };
        log(st, t.project_id, actor, text).await;
    }
}

/// Marca una tarea como hecha en nombre de `actor` (lo usa el bot de Telegram): mismas reglas que el PATCH.
pub async fn set_done(st: &AppState, actor: Id, id: Id) -> std::result::Result<Task, String> {
    let run = async {
        let team = team_of_task(st, id).await?;
        let role = require(st, team, actor, Role::Member).await?;
        let cur = fetch_task(st, id).await?;
        if role < Role::Admin && cur.assignee_id != Some(actor) {
            return Err(AppError(StatusCode::FORBIDDEN, "solo podés cerrar tus tareas".into()));
        }
        if cur.status == Status::Done {
            return Ok(cur);
        }
        sqlx::query("UPDATE tasks SET status = 'done', updated_at = ? WHERE id = ?").bind(Utc::now()).bind(id).execute(&st.db).await?;
        let t = fetch_task(st, id).await?;
        st.emit(team, Event::TaskUpdated(t.clone()));
        after_update(st, team, actor, &cur, &t).await;
        Ok(t)
    };
    run.await.map_err(|e: AppError| e.1)
}

// ---------- actividad ----------

pub async fn log_project(st: &AppState, project: Id, user: Id, text: String) {
    log_project_task(st, project, user, text, None).await;
}

pub async fn log_project_task(st: &AppState, project: Id, user: Id, text: String, task: Option<Id>) {
    let _ = sqlx::query("INSERT INTO activity (project_id, user_id, text, created_at, task_id) VALUES (?, ?, ?, ?, ?)")
        .bind(project)
        .bind(user)
        .bind(text.chars().take(300).collect::<String>())
        .bind(Utc::now())
        .bind(task)
        .execute(&st.db)
        .await;
    let _ = sqlx::query("DELETE FROM activity WHERE project_id = ? AND id NOT IN (SELECT id FROM activity WHERE project_id = ? ORDER BY id DESC LIMIT 500)")
        .bind(project)
        .bind(project)
        .execute(&st.db)
        .await;
}

pub async fn log(st: &AppState, project: Id, user: Id, text: String) {
    log_project(st, project, user, text).await;
}

pub async fn log_task(st: &AppState, task: Id, user: Id, text: String) {
    if let Ok(project) = sqlx::query_scalar::<_, Id>("SELECT project_id FROM tasks WHERE id = ?").bind(task).fetch_one(&st.db).await {
        log_project_task(st, project, user, text, Some(task)).await;
    }
}

async fn list_activity(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Vec<Activity>>> {
    ensure_member(&st, team_of_project(&st, project).await?, user.id).await?;
    let rows = sqlx::query(
        "SELECT a.id, a.project_id, a.user_id, COALESCE(u.username, '') AS username, a.text, a.created_at \
         FROM activity a LEFT JOIN users u ON u.id = a.user_id WHERE a.project_id = ? ORDER BY a.id DESC LIMIT 100",
    )
    .bind(project)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| Activity {
                id: r.get("id"),
                project_id: r.get("project_id"),
                user_id: r.get("user_id"),
                username: r.get("username"),
                text: r.get("text"),
                created_at: r.get("created_at"),
            })
            .collect(),
    ))
}

// ---------- menciones ----------

/// Nombres escritos como `@usuario` (al inicio o tras un espacio/puntuación), sin repetir.
pub fn mentions(body: &str) -> Vec<String> {
    let chars: Vec<char> = body.chars().collect();
    let mut out: Vec<String> = vec![];
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '@' && (i == 0 || !chars[i - 1].is_alphanumeric()) {
            let name: String = chars[i + 1..].iter().take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-')).take(limits::USERNAME_MAX).collect();
            let name = name.trim_end_matches(['.', '-']).to_string();
            if !name.is_empty() && !out.iter().any(|n| n.eq_ignore_ascii_case(&name)) && out.len() < 10 {
                out.push(name);
            }
        }
        i += 1;
    }
    out
}

/// Avisa a los miembros del equipo mencionados; devuelve a quiénes avisó.
pub async fn notify_mentions(st: &AppState, team: Id, author: Id, author_name: &str, task: Id, title: &str, body: &str) -> Vec<Id> {
    let mut notified = vec![];
    let preview: String = body.chars().take(200).collect();
    for name in mentions(body) {
        let id: Option<Id> = sqlx::query_scalar(
            "SELECT u.id FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND u.username = ? COLLATE NOCASE",
        )
        .bind(team)
        .bind(&name)
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten();
        if let Some(id) = id.filter(|i| *i != author) {
            crate::notify::notify(st, id, "mention", Some(team), Some(task), format!("{author_name} te mencionó en «{title}»: {preview}")).await;
            notified.push(id);
        }
    }
    notified
}

// ---------- recordatorios de vencimiento ----------

/// Cada `TM_REMINDER_SECS` (300 por defecto) avisa al responsable de las tareas que vencen en
/// menos de 24 h y, una sola vez, cuando ya vencieron.
pub fn spawn_reminders(st: AppState) {
    let secs = std::env::var("TM_REMINDER_SECS").ok().and_then(|v| v.parse::<u64>().ok()).filter(|s| *s > 0).unwrap_or(300);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(secs.min(30))).await;
            run_reminders(&st).await;
            if secs > 30 {
                tokio::time::sleep(Duration::from_secs(secs - 30)).await;
            }
        }
    });
}

pub async fn run_reminders(st: &AppState) {
    let now = Utc::now();
    let rows = sqlx::query(
        "SELECT t.id, t.title, t.assignee_id, t.due, t.remind_state, p.team_id FROM tasks t JOIN projects p ON p.id = t.project_id \
         WHERE t.status != 'done' AND t.assignee_id IS NOT NULL AND t.due IS NOT NULL AND t.due <= ? AND t.remind_state < 2 LIMIT 200",
    )
    .bind(now + chrono::Duration::hours(24))
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();
    for r in rows {
        let (id, title, who, team): (Id, String, Id, Id) = (r.get("id"), r.get("title"), r.get("assignee_id"), r.get("team_id"));
        let due: DateTime<Utc> = r.get("due");
        let state: i64 = r.get("remind_state");
        let (new_state, text) = if due <= now {
            (2, format!("⏰ «{title}» está vencida (era para el {})", due.format("%d/%m/%Y")))
        } else if state < 1 {
            (1, format!("⏰ «{title}» vence en menos de 24 horas ({})", due.format("%d/%m %H:%M")))
        } else {
            continue;
        };
        if sqlx::query("UPDATE tasks SET remind_state = ? WHERE id = ?").bind(new_state).bind(id).execute(&st.db).await.is_ok() {
            crate::notify::notify(st, who, "reminder", Some(team), Some(id), text).await;
        }
    }
}

// ---------- adjuntos ----------

#[derive(Deserialize)]
struct UploadParams {
    name: String,
}

fn clean_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let n: String = base.chars().filter(|c| !c.is_control()).take(limits::ATTACHMENT_NAME_MAX).collect();
    let n = n.trim().to_string();
    if n.is_empty() { "archivo".into() } else { n }
}

fn attachment_from_row(r: &sqlx::sqlite::SqliteRow) -> Attachment {
    Attachment {
        id: r.get("id"),
        task_id: r.get("task_id"),
        user_id: r.get("user_id"),
        username: r.get("username"),
        name: r.get("name"),
        size: r.get("size"),
        created_at: r.get("created_at"),
    }
}

const ATT_SELECT: &str = "SELECT a.id, a.task_id, a.user_id, COALESCE(u.username, '') AS username, a.name, a.size, a.created_at \
                          FROM attachments a LEFT JOIN users u ON u.id = a.user_id";

async fn list_attachments(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Vec<Attachment>>> {
    ensure_member(&st, team_of_task(&st, task).await?, user.id).await?;
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{ATT_SELECT} WHERE a.task_id = ? ORDER BY a.id"))).bind(task).fetch_all(&st.db).await?;
    Ok(Json(rows.iter().map(attachment_from_row).collect()))
}

async fn upload_attachment(
    State(st): State<AppState>,
    user: AuthUser,
    Path(task): Path<Id>,
    Query(p): Query<UploadParams>,
    body: Bytes,
) -> Result<Json<Attachment>> {
    let team = team_of_task(&st, task).await?;
    ensure_writer(&st, team, user.id).await?;
    if body.is_empty() {
        return Err(AppError::bad("archivo vacío"));
    }
    if body.len() > limits::MAX_ATTACHMENT_BYTES {
        return Err(AppError(StatusCode::PAYLOAD_TOO_LARGE, format!("máximo {} MB por archivo", limits::MAX_ATTACHMENT_BYTES / 1024 / 1024)));
    }
    st.user_limiter.hit(&format!("upload:{}", user.id), 60, Duration::from_secs(3600))?;
    if count(&st, "SELECT COUNT(*) FROM attachments WHERE task_id = ?", task).await? >= limits::MAX_ATTACHMENTS_PER_TASK {
        return Err(too_many("adjuntos", limits::MAX_ATTACHMENTS_PER_TASK));
    }
    let name = clean_name(&p.name);
    let id = sqlx::query("INSERT INTO attachments (task_id, user_id, name, size, data, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(task)
        .bind(user.id)
        .bind(&name)
        .bind(body.len() as i64)
        .bind(body.to_vec())
        .bind(Utc::now())
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    log_task(&st, task, user.id, format!("adjuntó «{name}»")).await;
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{ATT_SELECT} WHERE a.id = ?"))).bind(id).fetch_one(&st.db).await?;
    Ok(Json(attachment_from_row(&row)))
}

async fn download_attachment(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<Response> {
    let row = sqlx::query("SELECT task_id, name, data FROM attachments WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    ensure_member(&st, team_of_task(&st, row.get("task_id")).await?, user.id).await?;
    let name: String = row.get("name");
    let data: Vec<u8> = row.get("data");
    // siempre como descarga y sin tipo propio: un adjunto nunca se ejecuta ni se muestra en la página
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') { c } else { '_' }).collect();
    let mut resp = data.into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(resp)
}

async fn delete_attachment(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    let row = sqlx::query("SELECT task_id, user_id FROM attachments WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    let team = team_of_task(&st, row.get("task_id")).await?;
    let role = role_of(&st, team, user.id).await?;
    if role < Role::Member || (row.get::<Id, _>("user_id") != user.id && role < Role::Admin) {
        return Err(AppError::forbidden());
    }
    sqlx::query("DELETE FROM attachments WHERE id = ?").bind(id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- duplicar proyecto (plantilla) ----------

async fn duplicate_project(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<DuplicateProject>) -> Result<Json<Project>> {
    let team = team_of_project(&st, project).await?;
    ensure_writer(&st, team, user.id).await?;
    let name = b.name.trim();
    if name.is_empty() {
        return Err(AppError::bad("nombre requerido"));
    }
    max_len("nombre", name, limits::NAME_MAX)?;
    if count(&st, "SELECT COUNT(*) FROM projects WHERE team_id = ?", team).await? >= limits::MAX_PROJECTS_PER_TEAM {
        return Err(too_many("proyectos", limits::MAX_PROJECTS_PER_TEAM));
    }
    let src = sqlx::query("SELECT description, color FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await?;
    let (description, color): (String, String) = (src.get("description"), src.get("color"));
    let new_id = sqlx::query("INSERT INTO projects (team_id, name, description, color) VALUES (?, ?, ?, ?)")
        .bind(team)
        .bind(name)
        .bind(&description)
        .bind(&color)
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    // tareas: pendientes, sin responsable ni fechas; primero las principales y luego las subtareas
    let now = Utc::now();
    let rows = sqlx::query("SELECT id, parent_id, title, description, priority, recurrence FROM tasks WHERE project_id = ? ORDER BY (parent_id IS NOT NULL), id LIMIT ?")
        .bind(project)
        .bind(limits::MAX_TASKS_PER_PROJECT)
        .fetch_all(&st.db)
        .await?;
    let mut map = std::collections::HashMap::<Id, Id>::new();
    for r in &rows {
        let old: Id = r.get("id");
        let parent: Option<Id> = r.get::<Option<Id>, _>("parent_id").and_then(|p| map.get(&p).copied());
        let nid = sqlx::query(
            "INSERT INTO tasks (project_id, title, description, priority, parent_id, recurrence, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id)
        .bind(r.get::<String, _>("title"))
        .bind(r.get::<String, _>("description"))
        .bind(r.get::<i64, _>("priority"))
        .bind(parent)
        .bind(r.get::<Option<String>, _>("recurrence"))
        .bind(now)
        .bind(now)
        .execute(&st.db)
        .await?
        .last_insert_rowid();
        map.insert(old, nid);
        sqlx::query("INSERT INTO task_labels (task_id, label_id) SELECT ?, label_id FROM task_labels WHERE task_id = ?").bind(nid).bind(old).execute(&st.db).await?;
    }
    for (old, nid) in &map {
        let deps: Vec<Id> = sqlx::query_scalar("SELECT depends_on FROM task_deps WHERE task_id = ?").bind(old).fetch_all(&st.db).await?;
        for d in deps.iter().filter_map(|d| map.get(d)) {
            sqlx::query("INSERT OR IGNORE INTO task_deps (task_id, depends_on) VALUES (?, ?)").bind(nid).bind(d).execute(&st.db).await?;
        }
    }
    log_project(&st, new_id, user.id, format!("creó el proyecto copiando otro ({} tareas)", rows.len())).await;
    let p = Project { id: new_id, team_id: team, name: name.into(), description, color, work_dir: String::new(), repo: String::new() };
    st.emit(team, Event::ProjectCreated(p.clone()));
    Ok(Json(p))
}

// ---------- exportar ----------

#[derive(Deserialize)]
struct ExportParams {
    format: Option<String>,
}

/// Escapa una celda CSV y neutraliza fórmulas (`=`, `+`, `-`, `@`) para que no se ejecuten en una planilla.
fn csv_cell(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{s}") } else { s.to_string() };
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
}

async fn export_project(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Query(q): Query<ExportParams>) -> Result<Response> {
    let Json(tasks) = crate::routes::list_tasks(State(st.clone()), user, Path(project)).await?;
    let (body, mime, ext) = match q.format.as_deref().unwrap_or("csv") {
        "json" => (serde_json::to_string_pretty(&tasks).map_err(|_| AppError::bad("no se pudo exportar"))?, "application/json", "json"),
        "csv" => {
            let day = |d: Option<DateTime<Utc>>| d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
            let mut out = String::from("\u{feff}id,titulo,estado,prioridad,responsable,empieza,vence,repite,etiquetas,subtareas,padre\n");
            for t in &tasks {
                let cells = [
                    t.id.to_string(),
                    csv_cell(&t.title),
                    t.status.label().into(),
                    ["baja", "media", "alta", "urgente"][t.priority.clamp(0, 3) as usize].into(),
                    csv_cell(t.assignee.as_deref().unwrap_or("")),
                    day(t.start),
                    day(t.due),
                    t.recurrence.clone().unwrap_or_default(),
                    csv_cell(&t.labels.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(" ")),
                    format!("{}/{}", t.subtasks_done, t.subtasks_total),
                    t.parent_id.map(|p| p.to_string()).unwrap_or_default(),
                ];
                out.push_str(&cells.join(","));
                out.push('\n');
            }
            (out, "text/csv; charset=utf-8", "csv")
        }
        _ => return Err(AppError::bad("formato inválido: csv o json")),
    };
    let mut resp = body.into_response();
    resp.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"proyecto-{project}.{ext}\"")) {
        resp.headers_mut().insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menciones() {
        assert_eq!(mentions("hola @ana y @Beto-2, mail a@b.com @ana"), vec!["ana", "Beto-2"]);
        assert_eq!(mentions("@ana."), vec!["ana"]);
        assert!(mentions("sin menciones ni@arroba").is_empty());
    }

    #[test]
    fn csv_seguro() {
        assert_eq!(csv_cell("=1+1"), "'=1+1");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("di \"hola\""), "\"di \"\"hola\"\"\"");
    }

    #[test]
    fn recurrencia() {
        assert!(valid_recurrence(Some("weekly")).unwrap().is_some());
        assert!(valid_recurrence(Some("")).unwrap().is_none());
        assert!(valid_recurrence(Some("hourly")).is_err());
    }
}

//! Revisión humana del trabajo de las IAs (función «revision» de la licencia firmada).
//!
//! Con el interruptor del proyecto encendido, cuando una IA marca una tarea como Hecha la tarea NO pasa a Hecha:
//! queda «en revisión» y se avisa a una persona. Esa persona aprueba (pasa a Hecha) o pide cambios (vuelve a
//! Por hacer con su comentario, que la IA lee al retomar). Cada decisión queda en el historial.
use crate::{auth::AuthUser, error::{AppError, Result}, routes, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::{Id, Role, Status, Task};

const NOTE_MAX: usize = 4000;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects/{id}/reviews", get(project_reviews).put(set_enabled))
        .route("/api/projects/{id}/continuation", get(get_hold).put(set_hold))
        .route("/api/tasks/{id}/review", get(task_reviews).post(decide))
}

/// ¿La licencia firmada de esta instalación incluye la función?
async fn licensed(st: &AppState) -> bool {
    crate::license::pro_features(&st.db).await.iter().any(|f| f == "revision")
}

fn need_license() -> AppError {
    AppError(StatusCode::PAYMENT_REQUIRED, "La revisión del trabajo de las IAs está incluida en los planes Equipo y Empresa (Configuración → Licencia).".into())
}

async fn enabled(st: &AppState, project: Id) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT require_review FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await.ok().flatten().unwrap_or(0) == 1
}

/// Lo llama `update_task`: si una IA intenta pasar a Hecha una tarea de un proyecto con revisión, queda en revisión.
pub(crate) async fn holds(st: &AppState, user: &AuthUser, cur: &Task, new: Status) -> bool {
    user.bot && new == Status::Done && cur.status != Status::Done && enabled(st, cur.project_id).await && licensed(st).await
}

/// ¿Lo que una IA da por terminado queda esperando el próximo paso en vez de pasar a Hecho? (se enciende solo en cada proyecto)
pub(crate) async fn hold_ai_done(st: &AppState, project: Id) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT hold_ai_done FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await.ok().flatten().unwrap_or(0) == 1
}

async fn get_hold(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    let team = routes::team_of_project(&st, project).await?;
    routes::ensure_member(&st, team, user.id).await?;
    Ok(Json(json!({ "enabled": hold_ai_done(&st, project).await })))
}

async fn set_hold(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<EnabledBody>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(&st, project).await?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    sqlx::query("UPDATE projects SET hold_ai_done = ? WHERE id = ?").bind(b.enabled as i64).bind(project).execute(&st.db).await?;
    crate::extras::log(&st, project, user.id, if b.enabled { "activó «la IA entrega tramos y esperan tu próximo paso»".into() } else { "apagó la espera de continuación: lo que termina una IA pasa a Hecho".into() }).await;
    Ok(Json(json!({ "enabled": b.enabled })))
}

/// Persona que revisa: quien creó la tarea si es una persona con rol de miembro o más; si no, un admin o dueño del equipo.
async fn pick_reviewer(st: &AppState, team: Id, t: &Task) -> Option<Id> {
    if let Some(c) = t.created_by {
        let human: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_tokens WHERE user_id = ?").bind(c).fetch_one(&st.db).await.unwrap_or(1);
        if human == 0 && routes::require(st, team, c, Role::Member).await.is_ok() {
            return Some(c);
        }
    }
    sqlx::query_scalar(
        "SELECT m.user_id FROM team_members m WHERE m.team_id = ? AND m.role IN ('owner', 'admin') \
         AND m.user_id NOT IN (SELECT user_id FROM api_tokens) ORDER BY (m.role = 'owner') DESC, m.user_id LIMIT 1",
    )
    .bind(team)
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten()
}

/// Abre la revisión de una tarea que la IA dio por terminada y avisa a quien revisa.
pub(crate) async fn request(st: &AppState, team: Id, t: &Task, bot: Id) {
    let reviewer = pick_reviewer(st, team, t).await;
    let _ = sqlx::query("INSERT INTO task_reviews (task_id, requested_by, reviewer_id, state, requested_at) VALUES (?, ?, ?, 'pending', ?)")
        .bind(t.id)
        .bind(bot)
        .bind(reviewer)
        .bind(Utc::now())
        .execute(&st.db)
        .await;
    let who = crate::notify::username_of(st, bot).await;
    crate::extras::log_task(st, t.id, bot, format!("terminó «{}» y quedó en revisión", t.title)).await;
    if let Some(r) = reviewer {
        crate::notify::notify(st, r, "review", Some(team), Some(t.id), format!("🔎 {who} terminó «{}»: falta tu revisión", t.title)).await;
    }
}

async fn current(st: &AppState, task: Id) -> Option<Value> {
    sqlx::query("SELECT id, reviewer_id, state, requested_at FROM task_reviews WHERE task_id = ? ORDER BY id DESC LIMIT 1")
        .bind(task)
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten()
        .map(|r| json!({ "id": r.get::<i64, _>("id"), "reviewer_id": r.get::<Option<Id>, _>("reviewer_id"), "state": r.get::<String, _>("state"), "requested_at": r.get::<String, _>("requested_at") }))
}

/// Estado de la revisión del proyecto: interruptor, si la licencia lo permite y qué tareas esperan revisión.
async fn project_reviews(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    let team = routes::team_of_project(&st, project).await?;
    routes::ensure_member(&st, team, user.id).await?;
    let pending = sqlx::query(
        "SELECT r.task_id, r.reviewer_id, r.requested_at FROM task_reviews r JOIN tasks t ON t.id = r.task_id \
         WHERE t.project_id = ? AND r.state = 'pending' AND r.id = (SELECT MAX(id) FROM task_reviews WHERE task_id = r.task_id)",
    )
    .bind(project)
    .fetch_all(&st.db)
    .await?;
    let items: Vec<Value> = pending
        .iter()
        .map(|r| json!({ "task_id": r.get::<Id, _>("task_id"), "reviewer_id": r.get::<Option<Id>, _>("reviewer_id"), "requested_at": r.get::<String, _>("requested_at") }))
        .collect();
    Ok(Json(json!({ "enabled": enabled(&st, project).await, "licensed": licensed(&st).await, "pending": items })))
}

#[derive(Deserialize)]
struct EnabledBody {
    enabled: bool,
}

async fn set_enabled(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<EnabledBody>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_project(&st, project).await?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    if b.enabled && !licensed(&st).await {
        return Err(need_license());
    }
    sqlx::query("UPDATE projects SET require_review = ? WHERE id = ?").bind(b.enabled as i64).bind(project).execute(&st.db).await?;
    crate::extras::log(&st, project, user.id, if b.enabled { "activó la revisión humana del trabajo de las IAs".into() } else { "apagó la revisión humana".into() }).await;
    Ok(Json(json!({ "enabled": b.enabled })))
}

/// Historial de revisiones de una tarea.
async fn task_reviews(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    let team = routes::team_of_task(&st, task).await?;
    routes::ensure_member(&st, team, user.id).await?;
    let rows = sqlx::query(
        "SELECT r.id, r.state, r.note, r.requested_at, r.decided_at, \
         (SELECT username FROM users WHERE id = r.reviewer_id) AS reviewer, (SELECT username FROM users WHERE id = r.decided_by) AS decided_by, \
         (SELECT username FROM users WHERE id = r.requested_by) AS requested_by, r.reviewer_id \
         FROM task_reviews r WHERE r.task_id = ? ORDER BY r.id DESC LIMIT 50",
    )
    .bind(task)
    .fetch_all(&st.db)
    .await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64, _>("id"), "state": r.get::<String, _>("state"), "note": r.get::<String, _>("note"),
                "requested_at": r.get::<String, _>("requested_at"), "decided_at": r.get::<Option<String>, _>("decided_at"),
                "reviewer": r.get::<Option<String>, _>("reviewer"), "reviewer_id": r.get::<Option<Id>, _>("reviewer_id"),
                "decided_by": r.get::<Option<String>, _>("decided_by"), "requested_by": r.get::<Option<String>, _>("requested_by"),
            })
        })
        .collect();
    Ok(Json(json!({ "items": items })))
}

#[derive(Deserialize)]
struct DecideBody {
    /// «approve» o «changes».
    action: String,
    #[serde(default)]
    note: String,
}

/// Aprobar o pedir cambios: lo hace quien tiene asignada la revisión o un admin/dueño; nunca una IA.
async fn decide(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>, Json(b): Json<DecideBody>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = routes::team_of_task(&st, task).await?;
    let role = routes::require(&st, team, user.id, Role::Member).await?;
    if b.note.chars().count() > NOTE_MAX {
        return Err(AppError::bad("la nota es demasiado larga"));
    }
    if !licensed(&st).await {
        return Err(need_license());
    }
    let row = sqlx::query("SELECT id, reviewer_id, requested_by FROM task_reviews WHERE task_id = ? AND state = 'pending' ORDER BY id DESC LIMIT 1")
        .bind(task)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| AppError::bad("esta tarea no está esperando revisión"))?;
    let (rid, reviewer, bot): (i64, Option<Id>, Id) = (row.get("id"), row.get("reviewer_id"), row.get("requested_by"));
    if reviewer != Some(user.id) && role < Role::Admin {
        return Err(AppError::forbidden());
    }
    let (state, status, text) = match b.action.as_str() {
        "approve" => ("approved", Status::Done, "✅ Revisión aprobada".to_string()),
        "changes" => {
            if b.note.trim().is_empty() {
                return Err(AppError::bad("escribí qué hay que cambiar"));
            }
            ("changes", Status::Todo, "🔁 Cambios pedidos".to_string())
        }
        _ => return Err(AppError::bad("acción inválida")),
    };
    sqlx::query("UPDATE task_reviews SET state = ?, note = ?, decided_by = ?, decided_at = ? WHERE id = ?")
        .bind(state)
        .bind(b.note.trim())
        .bind(user.id)
        .bind(Utc::now())
        .bind(rid)
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?").bind(status.as_str()).bind(Utc::now()).bind(task).execute(&st.db).await?;
    // el comentario lo lee la IA al retomar la tarea
    let body = if b.note.trim().is_empty() { text.clone() } else { format!("{text}: {}", b.note.trim()) };
    sqlx::query("INSERT INTO comments (task_id, user_id, body, created_at) VALUES (?, ?, ?, ?)").bind(task).bind(user.id).bind(&body).bind(Utc::now()).execute(&st.db).await?;
    let t = routes::fetch_task(&st, task).await?;
    st.emit(team, tm_core::Event::TaskUpdated(t.clone()));
    crate::extras::log_task(&st, task, user.id, format!("{} «{}»", if state == "approved" { "aprobó" } else { "pidió cambios en" }, t.title)).await;
    if state == "changes" {
        crate::notify::notify(&st, bot, "review", Some(team), Some(task), format!("🔁 Pidieron cambios en «{}»: {}", t.title, b.note.trim())).await;
    }
    let _ = current(&st, task).await;
    Ok(Json(json!({ "state": state, "task": t })))
}

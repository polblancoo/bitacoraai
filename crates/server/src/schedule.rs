//! Tareas programadas: a una hora fija (cada día, cada semana o cada mes) el servidor crea una tarea ya asignada,
//! por ejemplo para que una IA «chequee resultados» todos los lunes. Es distinto de la repetición de una tarea
//! (`recurrence`), que crea la siguiente cuando se completa la anterior.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    extras::log_project,
    limits::{max_len, too_many, DESCRIPTION_MAX, MAX_TASKS_PER_PROJECT, TITLE_MAX},
    routes::{check_assign, count, fetch_task, notify_assigned, require, role_of, team_of_project},
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::{Event, Id, Role};

const MAX_PER_PROJECT: i64 = 50;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects/{id}/schedules", get(list).post(create))
        .route("/api/schedules/{id}", put(update).delete(remove))
        .route("/api/schedules/{id}/run", post(run_now))
}

pub fn spawn_schedules(st: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            run_due(&st).await;
        }
    });
}

fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1).and_then(|d| d.pred_opt()).map_or(28, |d| d.day())
}

/// Próxima vez (en UTC) que toca, estrictamente después de `after`. `at_minute` son los minutos desde la medianoche
/// local y `tz_offset` los minutos al este de UTC (Argentina: -180). Semanal: `weekday` 0 = lunes … 6 = domingo.
/// Mensual: si el mes es más corto que `monthday`, toca el último día.
pub fn next_after(freq: &str, weekday: i64, monthday: i64, at_minute: i64, tz_offset: i64, after: DateTime<Utc>) -> DateTime<Utc> {
    let off = Duration::minutes(tz_offset);
    let local_after = (after + off).naive_utc();
    let at = NaiveTime::from_hms_opt((at_minute / 60) as u32 % 24, (at_minute % 60) as u32, 0).unwrap_or_default();
    let mut day = local_after.date();
    for _ in 0..70 {
        let toca = match freq {
            "weekly" => i64::from(day.weekday().num_days_from_monday()) == weekday,
            "monthly" => i64::from(day.day()) == monthday.min(i64::from(days_in_month(day.year(), day.month()))),
            _ => true,
        };
        if toca {
            let cand = day.and_time(at);
            if cand > local_after {
                return DateTime::from_naive_utc_and_offset(cand - off, Utc);
            }
        }
        match day.succ_opt() {
            Some(d) => day = d,
            None => break,
        }
    }
    after + Duration::days(1)
}

fn label(freq: &str) -> &'static str {
    match freq {
        "daily" => "cada día",
        "weekly" => "cada semana",
        _ => "cada mes",
    }
}

fn one() -> i64 { 1 }
fn nine() -> i64 { 540 }
fn yes() -> bool { true }

#[derive(Deserialize)]
struct Body {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default = "one")]
    priority: i64,
    assignee_id: Option<Id>,
    freq: String,
    #[serde(default)]
    weekday: i64,
    #[serde(default = "one")]
    monthday: i64,
    #[serde(default = "nine")]
    at_minute: i64,
    #[serde(default)]
    tz_offset: i64,
    #[serde(default = "yes")]
    skip_if_open: bool,
    #[serde(default = "yes")]
    active: bool,
}

fn validate(b: &Body) -> Result<()> {
    if b.title.trim().is_empty() {
        return Err(AppError::bad("título requerido"));
    }
    max_len("título", &b.title, TITLE_MAX)?;
    max_len("descripción", &b.description, DESCRIPTION_MAX)?;
    if !matches!(b.freq.as_str(), "daily" | "weekly" | "monthly") {
        return Err(AppError::bad("frecuencia inválida: daily, weekly o monthly"));
    }
    if !(0..=6).contains(&b.weekday) || !(1..=31).contains(&b.monthday) || !(0..1440).contains(&b.at_minute) || !(-840..=840).contains(&b.tz_offset) {
        return Err(AppError::bad("día, hora o zona horaria fuera de rango"));
    }
    Ok(())
}

fn row_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    json!({
        "id": r.get::<Id, _>("id"), "title": r.get::<String, _>("title"), "description": r.get::<String, _>("description"),
        "priority": r.get::<i64, _>("priority"), "assignee_id": r.get::<Option<Id>, _>("assignee_id"),
        "assignee": r.get::<Option<String>, _>("assignee"), "freq": r.get::<String, _>("freq"),
        "weekday": r.get::<i64, _>("weekday"), "monthday": r.get::<i64, _>("monthday"), "at_minute": r.get::<i64, _>("at_minute"),
        "tz_offset": r.get::<i64, _>("tz_offset"), "skip_if_open": r.get::<i64, _>("skip_if_open") != 0,
        "active": r.get::<i64, _>("active") != 0, "next_run": r.get::<DateTime<Utc>, _>("next_run"),
        "last_run": r.get::<Option<DateTime<Utc>>, _>("last_run"), "last_task_id": r.get::<Option<Id>, _>("last_task_id"),
    })
}

async fn list(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Vec<Value>>> {
    let team = team_of_project(&st, project).await?;
    require(&st, team, user.id, Role::Viewer).await?;
    let rows = sqlx::query(
        "SELECT s.*, u.username AS assignee FROM schedules s LEFT JOIN users u ON u.id = s.assignee_id WHERE s.project_id = ? ORDER BY s.id",
    )
    .bind(project)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(rows.iter().map(row_json).collect()))
}

async fn create(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<Body>) -> Result<Json<Value>> {
    if user.bot {
        return Err(AppError::forbidden());
    }
    let team = team_of_project(&st, project).await?;
    let role = require(&st, team, user.id, Role::Member).await?;
    validate(&b)?;
    if count(&st, "SELECT COUNT(*) FROM schedules WHERE project_id = ?", project).await? >= MAX_PER_PROJECT {
        return Err(too_many("tareas programadas", MAX_PER_PROJECT));
    }
    check_assign(&st, team, user.id, role, None, b.assignee_id, false).await?;
    let now = Utc::now();
    let next = next_after(&b.freq, b.weekday, b.monthday, b.at_minute, b.tz_offset, now);
    let id = sqlx::query(
        "INSERT INTO schedules (project_id, title, description, priority, assignee_id, freq, weekday, monthday, at_minute, tz_offset, \
         skip_if_open, active, next_run, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(project)
    .bind(b.title.trim())
    .bind(&b.description)
    .bind(b.priority.clamp(0, 3))
    .bind(b.assignee_id)
    .bind(&b.freq)
    .bind(b.weekday)
    .bind(b.monthday)
    .bind(b.at_minute)
    .bind(b.tz_offset)
    .bind(i64::from(b.skip_if_open))
    .bind(i64::from(b.active))
    .bind(next)
    .bind(user.id)
    .bind(now)
    .execute(&st.db)
    .await?
    .last_insert_rowid();
    log_project(&st, project, user.id, format!("programó «{}» ({})", b.title.trim(), label(&b.freq))).await;
    Ok(Json(json!({ "id": id, "next_run": next })))
}

/// Carga una programación y comprueba que quien llama puede cambiarla (quien la creó o un líder).
async fn owned(st: &AppState, user: &AuthUser, id: Id) -> Result<(Id, Id, Role, Option<Id>)> {
    if user.bot {
        return Err(AppError::forbidden());
    }
    let r = sqlx::query("SELECT project_id, created_by, assignee_id FROM schedules WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    let project: Id = r.get("project_id");
    let team = team_of_project(st, project).await?;
    let role = require(st, team, user.id, Role::Member).await?;
    if role < Role::Admin && r.get::<Option<Id>, _>("created_by") != Some(user.id) {
        return Err(AppError::forbidden());
    }
    Ok((project, team, role, r.get("assignee_id")))
}

async fn update(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Json(b): Json<Body>) -> Result<Json<Value>> {
    let (_, team, role, current) = owned(&st, &user, id).await?;
    validate(&b)?;
    check_assign(&st, team, user.id, role, current, b.assignee_id, false).await?;
    let next = next_after(&b.freq, b.weekday, b.monthday, b.at_minute, b.tz_offset, Utc::now());
    sqlx::query(
        "UPDATE schedules SET title = ?, description = ?, priority = ?, assignee_id = ?, freq = ?, weekday = ?, monthday = ?, at_minute = ?, \
         tz_offset = ?, skip_if_open = ?, active = ?, next_run = ? WHERE id = ?",
    )
    .bind(b.title.trim())
    .bind(&b.description)
    .bind(b.priority.clamp(0, 3))
    .bind(b.assignee_id)
    .bind(&b.freq)
    .bind(b.weekday)
    .bind(b.monthday)
    .bind(b.at_minute)
    .bind(b.tz_offset)
    .bind(i64::from(b.skip_if_open))
    .bind(i64::from(b.active))
    .bind(next)
    .bind(id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({ "next_run": next })))
}

async fn remove(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<Json<Value>> {
    owned(&st, &user, id).await?;
    sqlx::query("DELETE FROM schedules WHERE id = ?").bind(id).execute(&st.db).await?;
    Ok(Json(json!({ "ok": true })))
}

/// «Ejecutar ahora»: crea la tarea sin esperar y sin mover el próximo horario.
async fn run_now(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<Json<Value>> {
    owned(&st, &user, id).await?;
    let task = spawn(&st, id, true).await?;
    Ok(Json(json!({ "task_id": task })))
}

/// Crea la tarea de una programación. Devuelve `None` si no corresponde crearla (la anterior sigue abierta o el
/// proyecto llegó al máximo de tareas).
async fn spawn(st: &AppState, sid: Id, force: bool) -> Result<Option<Id>> {
    let r = sqlx::query("SELECT * FROM schedules WHERE id = ?").bind(sid).fetch_one(&st.db).await?;
    let project: Id = r.get("project_id");
    let team = team_of_project(st, project).await?;
    if !force && r.get::<i64, _>("skip_if_open") != 0 {
        if let Some(last) = r.get::<Option<Id>, _>("last_task_id") {
            let abierta = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks WHERE id = ? AND status != 'done'").bind(last).fetch_one(&st.db).await?;
            if abierta > 0 {
                return Ok(None);
            }
        }
    }
    if count(st, "SELECT COUNT(*) FROM tasks WHERE project_id = ?", project).await? >= MAX_TASKS_PER_PROJECT {
        return Ok(None);
    }
    let mut assignee: Option<Id> = r.get("assignee_id");
    if let Some(a) = assignee {
        // si ya no es miembro (o quedó de lector), la tarea nace sin responsable
        if !role_of(st, team, a).await.is_ok_and(|x| x >= Role::Member) {
            assignee = None;
        }
    }
    let creator: Option<Id> = r.get("created_by");
    let now = Utc::now();
    // la tarea nace con el día (y la hora, en la descripción) en que se programó, en la zona de quien la creó
    let local = (now + Duration::minutes(r.get::<i64, _>("tz_offset"))).naive_utc();
    let day: DateTime<Utc> = DateTime::from_naive_utc_and_offset(local.date().and_hms_opt(12, 0, 0).unwrap_or(local), Utc);
    let at = r.get::<i64, _>("at_minute");
    let mut description = r.get::<String, _>("description");
    if !description.is_empty() {
        description.push_str("\n\n");
    }
    description.push_str(&format!(
        "⏰ Tarea programada ({}, {:02}:{:02}). Creada el {} a las {}.",
        label(&r.get::<String, _>("freq")),
        at / 60,
        at % 60,
        local.format("%d/%m/%Y"),
        local.format("%H:%M")
    ));
    let tid = sqlx::query(
        "INSERT INTO tasks (project_id, title, description, priority, assignee_id, due, start, created_at, updated_at, created_by, position) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, (SELECT COALESCE(MAX(position), 0) + 1 FROM tasks WHERE project_id = ?))",
    )
    .bind(project)
    .bind(r.get::<String, _>("title"))
    .bind(description)
    .bind(r.get::<i64, _>("priority"))
    .bind(assignee)
    .bind(day)
    .bind(day)
    .bind(now)
    .bind(now)
    .bind(creator)
    .bind(project)
    .execute(&st.db)
    .await?
    .last_insert_rowid();
    sqlx::query("UPDATE schedules SET last_run = ?, last_task_id = ? WHERE id = ?").bind(now).bind(tid).bind(sid).execute(&st.db).await?;
    if let Ok(t) = fetch_task(st, tid).await {
        st.emit(team, Event::TaskCreated(t.clone()));
        notify_assigned(st, team, creator.unwrap_or(0), &t).await;
        log_project(st, project, creator.unwrap_or(0), format!("tarea programada «{}» ({})", t.title, label(&r.get::<String, _>("freq")))).await;
    }
    Ok(Some(tid))
}

async fn run_due(st: &AppState) {
    let now = Utc::now();
    let Ok(rows) = sqlx::query("SELECT id, freq, weekday, monthday, at_minute, tz_offset FROM schedules WHERE active = 1 AND next_run <= ?")
        .bind(now)
        .fetch_all(&st.db)
        .await
    else {
        return;
    };
    for r in rows {
        let id: Id = r.get("id");
        // se avanza el horario antes de crear la tarea: así una falla no la repite en cada vuelta y, si el servidor
        // estuvo apagado, se crea una sola tarea atrasada y no una por cada hora perdida
        let next = next_after(&r.get::<String, _>("freq"), r.get("weekday"), r.get("monthday"), r.get("at_minute"), r.get("tz_offset"), now);
        let _ = sqlx::query("UPDATE schedules SET next_run = ? WHERE id = ?").bind(next).bind(id).execute(&st.db).await;
        if let Err(e) = spawn(st, id, false).await {
            tracing::warn!("tarea programada {id}: {}", e.1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn diaria_hoy_o_manana() {
        // Argentina (UTC-3), todos los días 09:00 locales = 12:00 UTC
        assert_eq!(next_after("daily", 0, 1, 540, -180, utc("2026-10-30T10:00:00Z")), utc("2026-10-30T12:00:00Z"));
        assert_eq!(next_after("daily", 0, 1, 540, -180, utc("2026-10-30T13:00:00Z")), utc("2026-10-31T12:00:00Z"));
        // justo a la hora: pasa al día siguiente (estrictamente después)
        assert_eq!(next_after("daily", 0, 1, 540, -180, utc("2026-10-30T12:00:00Z")), utc("2026-10-31T12:00:00Z"));
    }

    #[test]
    fn semanal_lunes() {
        // 2026-10-30 es viernes; el lunes siguiente es 2026-11-02
        assert_eq!(next_after("weekly", 0, 1, 540, 0, utc("2026-10-30T10:00:00Z")), utc("2026-11-02T09:00:00Z"));
        // el mismo lunes antes de la hora
        assert_eq!(next_after("weekly", 0, 1, 540, 0, utc("2026-11-02T08:00:00Z")), utc("2026-11-02T09:00:00Z"));
    }

    #[test]
    fn mensual_y_fin_de_mes() {
        assert_eq!(next_after("monthly", 0, 15, 600, 0, utc("2026-10-30T10:00:00Z")), utc("2026-11-15T10:00:00Z"));
        // día 31 en un mes de 30 días: toca el último día
        assert_eq!(next_after("monthly", 0, 31, 600, 0, utc("2026-11-01T00:00:00Z")), utc("2026-11-30T10:00:00Z"));
        // febrero
        assert_eq!(next_after("monthly", 0, 31, 600, 0, utc("2027-02-01T00:00:00Z")), utc("2027-02-28T10:00:00Z"));
    }

    #[test]
    fn la_zona_horaria_cambia_el_dia() {
        // 22:00 locales en UTC-3 son las 01:00 UTC del día siguiente
        assert_eq!(next_after("daily", 0, 1, 1320, -180, utc("2026-10-30T20:00:00Z")), utc("2026-10-31T01:00:00Z"));
    }
}

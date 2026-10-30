//! Consumo de las IAs: tokens, costo y horas por tarea, proyecto y equipo, con cuotas mensuales
//! (equipo y proyecto) y tope por tarea. Los tokens son los «frescos»: entrada + salida + creación de caché
//! (la lectura de caché se guarda aparte porque es mucho más barata).
use crate::{auth::AuthUser, error::{AppError, Result}, routes, AppState};
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use tm_core::{Id, Role};

const FRESH: &str = "(input_tokens + output_tokens + cache_create)";
const MAX_TOKENS: i64 = 100_000_000_000;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/tasks/{id}/run", post(record_run))
        .route("/api/tasks/{id}/budget-check", post(budget_check))
        .route("/api/tasks/{id}/usage", get(task_usage))
        .route("/api/tasks/{id}/budget", put(set_budget))
        .route("/api/projects/{id}/estimate", post(estimate))
        .route("/api/projects/{id}/usage", get(project_usage))
        .route("/api/projects/{id}/quota", put(set_project_quota))
        .route("/api/teams/{id}/usage", get(team_usage))
        .route("/api/teams/{id}/quota", put(set_team_quota))
}

fn month_start() -> String {
    format!("{}-01", Utc::now().format("%Y-%m"))
}

async fn project_team(st: &AppState, project: Id) -> Result<Id> {
    sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)
}
async fn task_project(st: &AppState, task: Id) -> Result<Id> {
    sqlx::query_scalar("SELECT project_id FROM tasks WHERE id = ?").bind(task).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)
}

/// (tokens frescos, costo USD, segundos, ejecuciones) con un filtro SQL ya armado.
async fn sums(st: &AppState, filter: &str, bind: &[String]) -> Result<(i64, f64, i64, i64)> {
    let sql = format!("SELECT COALESCE(SUM({FRESH}), 0), COALESCE(SUM(cost_usd), 0.0), COALESCE(SUM(seconds), 0), COUNT(*) FROM task_runs WHERE {filter}");
    let mut q = sqlx::query_as::<_, (i64, f64, i64, i64)>(sqlx::AssertSqlSafe(sql));
    for b in bind {
        q = q.bind(b.clone());
    }
    Ok(q.fetch_one(&st.db).await?)
}

#[derive(Deserialize)]
struct Run {
    #[serde(default)]
    seconds: i64,
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    cache_create: i64,
    #[serde(default)]
    cache_read: i64,
    #[serde(default)]
    cost_usd: f64,
    #[serde(default)]
    model: String,
    #[serde(default = "yes")]
    ok: bool,
}
fn yes() -> bool {
    true
}

/// La IA informa lo que gastó en una ejecución (solo la responsable de la tarea).
async fn record_run(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>, Json(r): Json<Run>) -> Result<Json<Value>> {
    let mine: Option<i64> = sqlx::query_scalar("SELECT 1 FROM tasks WHERE id = ? AND assignee_id = ?").bind(task).bind(user.id).fetch_optional(&st.db).await?;
    mine.filter(|_| user.bot).ok_or_else(AppError::forbidden)?;
    let project = task_project(&st, task).await?;
    let c = |n: i64| n.clamp(0, MAX_TOKENS);
    let started = Utc::now() - Duration::seconds(r.seconds.clamp(0, 86_400));
    sqlx::query("INSERT INTO task_runs (task_id, project_id, bot_id, model, started_at, seconds, input_tokens, output_tokens, cache_create, cache_read, cost_usd, ok) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(task)
        .bind(project)
        .bind(user.id)
        .bind(r.model.chars().take(80).collect::<String>())
        .bind(started)
        .bind(r.seconds.clamp(0, 86_400))
        .bind(c(r.input_tokens))
        .bind(c(r.output_tokens))
        .bind(c(r.cache_create))
        .bind(c(r.cache_read))
        .bind(if r.cost_usd.is_finite() { r.cost_usd.clamp(0.0, 100_000.0) } else { 0.0 })
        .bind(i64::from(r.ok))
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

/// Antes de empezar, la IA pregunta si quedan tokens: tope de la tarea, cuota del proyecto y cuota del equipo.
async fn budget_check(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    let mine: Option<i64> = sqlx::query_scalar("SELECT 1 FROM tasks WHERE id = ? AND assignee_id = ?").bind(task).bind(user.id).fetch_optional(&st.db).await?;
    mine.filter(|_| user.bot).ok_or_else(AppError::forbidden)?;
    let project = task_project(&st, task).await?;
    let team = project_team(&st, project).await?;
    let ms = month_start();
    let (budget, pq): (i64, i64) = sqlx::query_as("SELECT t.token_budget, p.token_quota FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?").bind(task).fetch_one(&st.db).await?;
    let tq: i64 = sqlx::query_scalar("SELECT token_quota FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    if budget > 0 {
        let used = sums(&st, "task_id = ?", &[task.to_string()]).await?.0;
        if used >= budget {
            return Ok(Json(json!({ "ok": false, "reason": format!("la tarea ya gastó {used} de sus {budget} tokens: subile el tope para que siga") })));
        }
    }
    if pq > 0 {
        let used = sums(&st, "project_id = ? AND started_at >= ?", &[project.to_string(), ms.clone()]).await?.0;
        if used >= pq {
            return Ok(Json(json!({ "ok": false, "reason": format!("el proyecto agotó su cuota del mes ({used} de {pq} tokens)") })));
        }
    }
    if tq > 0 {
        let used = sums(&st, "project_id IN (SELECT id FROM projects WHERE team_id = ?) AND started_at >= ?", &[team.to_string(), ms]).await?.0;
        if used >= tq {
            return Ok(Json(json!({ "ok": false, "reason": format!("el equipo agotó su cuota del mes ({used} de {tq} tokens)") })));
        }
    }
    Ok(Json(json!({ "ok": true })))
}

async fn task_usage(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    let project = task_project(&st, task).await?;
    routes::ensure_member(&st, project_team(&st, project).await?, user.id).await?;
    let budget: i64 = sqlx::query_scalar("SELECT token_budget FROM tasks WHERE id = ?").bind(task).fetch_one(&st.db).await?;
    let (tokens, cost, seconds, runs) = sums(&st, "task_id = ?", &[task.to_string()]).await?;
    Ok(Json(json!({ "budget": budget, "tokens": tokens, "cost_usd": cost, "seconds": seconds, "runs": runs })))
}

#[derive(Deserialize)]
struct Tokens {
    tokens: i64,
}

/// El creador (o cualquier miembro con permiso de escritura) fija cuántos tokens puede gastar la tarea; 0 = sin tope.
async fn set_budget(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>, Json(b): Json<Tokens>) -> Result<Json<Value>> {
    user.human_only()?;
    let project = task_project(&st, task).await?;
    routes::require(&st, project_team(&st, project).await?, user.id, Role::Member).await?;
    sqlx::query("UPDATE tasks SET token_budget = ? WHERE id = ?").bind(b.tokens.clamp(0, MAX_TOKENS)).bind(task).execute(&st.db).await?;
    Ok(Json(json!({ "budget": b.tokens.clamp(0, MAX_TOKENS) })))
}

#[derive(Deserialize)]
struct Est {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
}

/// Estimación de tokens para una tarea nueva: el promedio real de las últimas ejecuciones del proyecto
/// o, si todavía no hay historial, una regla simple según el largo del pedido. Es solo una guía.
async fn estimate(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(e): Json<Est>) -> Result<Json<Value>> {
    routes::ensure_member(&st, project_team(&st, project).await?, user.id).await?;
    let hist: Vec<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT {FRESH} FROM task_runs WHERE project_id = ? AND ok = 1 AND {FRESH} > 0 ORDER BY id DESC LIMIT 20")))
        .bind(project)
        .fetch_all(&st.db)
        .await?;
    let chars = (e.title.chars().count() + e.description.chars().count()) as i64;
    Ok(Json(if hist.is_empty() {
        json!({ "tokens": (30_000 + chars * 8).clamp(20_000, 2_000_000), "basis": "regla", "runs": 0 })
    } else {
        let avg = hist.iter().sum::<i64>() / hist.len() as i64;
        json!({ "tokens": avg, "basis": "historial", "runs": hist.len() })
    }))
}

async fn project_usage(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    let team = project_team(&st, project).await?;
    routes::ensure_member(&st, team, user.id).await?;
    let ms = month_start();
    let (mt, mc, msec, mr) = sums(&st, "project_id = ? AND started_at >= ?", &[project.to_string(), ms.clone()]).await?;
    let (at, ac, asec, ar) = sums(&st, "project_id = ?", &[project.to_string()]).await?;
    let quota: i64 = sqlx::query_scalar("SELECT token_quota FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await?;
    let by_ai: Vec<(String, String, i64, f64, i64, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT COALESCE(u.username, '(borrada)'), COALESCE(NULLIF(r.model, ''), COALESCE(u.model, '')), COALESCE(SUM({}), 0), COALESCE(SUM(r.cost_usd), 0.0), COALESCE(SUM(r.seconds), 0), COUNT(*) \
         FROM task_runs r LEFT JOIN users u ON u.id = r.bot_id WHERE r.project_id = ? AND r.started_at >= ? GROUP BY r.bot_id ORDER BY 3 DESC",
        FRESH.replace("input_tokens", "r.input_tokens").replace("output_tokens", "r.output_tokens").replace("cache_create", "r.cache_create")
    )))
    .bind(project)
    .bind(&ms)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!({
        "month": { "tokens": mt, "cost_usd": mc, "seconds": msec, "runs": mr },
        "total": { "tokens": at, "cost_usd": ac, "seconds": asec, "runs": ar },
        "quota": quota,
        "by_ai": by_ai.into_iter().map(|(name, model, tokens, cost, seconds, runs)| json!({ "name": name, "model": model, "tokens": tokens, "cost_usd": cost, "seconds": seconds, "runs": runs })).collect::<Vec<_>>(),
    })))
}

async fn team_usage(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    routes::ensure_member(&st, team, user.id).await?;
    let ms = month_start();
    let projects: Vec<(Id, String, i64)> = sqlx::query_as("SELECT id, name, token_quota FROM projects WHERE team_id = ? ORDER BY id").bind(team).fetch_all(&st.db).await?;
    let mut out = Vec::new();
    for (id, name, quota) in projects {
        let (tokens, cost, seconds, runs) = sums(&st, "project_id = ? AND started_at >= ?", &[id.to_string(), ms.clone()]).await?;
        out.push(json!({ "id": id, "name": name, "quota": quota, "tokens": tokens, "cost_usd": cost, "seconds": seconds, "runs": runs }));
    }
    let (tokens, cost, seconds, runs) = sums(&st, "project_id IN (SELECT id FROM projects WHERE team_id = ?) AND started_at >= ?", &[team.to_string(), ms]).await?;
    let quota: i64 = sqlx::query_scalar("SELECT token_quota FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    Ok(Json(json!({ "team": { "quota": quota, "tokens": tokens, "cost_usd": cost, "seconds": seconds, "runs": runs }, "projects": out })))
}

async fn set_project_quota(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Json(b): Json<Tokens>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, project_team(&st, project).await?, user.id, Role::Admin).await?;
    sqlx::query("UPDATE projects SET token_quota = ? WHERE id = ?").bind(b.tokens.clamp(0, MAX_TOKENS)).bind(project).execute(&st.db).await?;
    Ok(Json(json!({ "quota": b.tokens.clamp(0, MAX_TOKENS) })))
}

async fn set_team_quota(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<Tokens>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Owner).await?;
    sqlx::query("UPDATE teams SET token_quota = ? WHERE id = ?").bind(b.tokens.clamp(0, MAX_TOKENS)).bind(team).execute(&st.db).await?;
    Ok(Json(json!({ "quota": b.tokens.clamp(0, MAX_TOKENS) })))
}

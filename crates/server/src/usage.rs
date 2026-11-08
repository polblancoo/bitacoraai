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
    let this = c(r.input_tokens) + c(r.output_tokens) + c(r.cache_create);
    budget_alerts(&st, task, project, this).await;
    Ok(Json(json!({ "ok": true })))
}

/// Avisa a los dueños y administradores cuando un tope (tarea, proyecto o equipo) cruza el 80 % o se agota con esta ejecución.
/// El corte duro ya lo hace `budget_check`: la IA no arranca otra ejecución con el tope agotado.
async fn budget_alerts(st: &AppState, task: Id, project: Id, this: i64) {
    let Ok(team) = project_team(st, project).await else { return };
    let ms = month_start();
    let Ok((budget, pq, title, pname)): std::result::Result<(i64, i64, String, String), _> =
        sqlx::query_as("SELECT t.token_budget, p.token_quota, t.title, p.name FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?").bind(task).fetch_one(&st.db).await
    else { return };
    let tq: i64 = sqlx::query_scalar("SELECT token_quota FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await.unwrap_or(0);
    let mut hits: Vec<String> = Vec::new();
    let mut scopes: Vec<(String, i64, i64)> = Vec::new();
    if budget > 0 {
        scopes.push((format!("la tarea «{title}»"), sums(st, "task_id = ?", &[task.to_string()]).await.map(|x| x.0).unwrap_or(0), budget));
    }
    if pq > 0 {
        scopes.push((format!("el proyecto «{pname}» (este mes)"), sums(st, "project_id = ? AND started_at >= ?", &[project.to_string(), ms.clone()]).await.map(|x| x.0).unwrap_or(0), pq));
    }
    if tq > 0 {
        scopes.push(("el equipo (este mes)".to_string(), sums(st, "project_id IN (SELECT id FROM projects WHERE team_id = ?) AND started_at >= ?", &[team.to_string(), ms]).await.map(|x| x.0).unwrap_or(0), tq));
    }
    for (what, now, limit) in scopes {
        if let Some(msg) = crossed(now, this, limit) {
            hits.push(format!("{what}: {msg} ({now} de {limit} tokens)"));
        }
    }
    if hits.is_empty() {
        return;
    }
    let admins: Vec<Id> = sqlx::query_scalar("SELECT m.user_id FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND m.role IN ('owner', 'admin') AND u.is_bot = 0")
        .bind(team)
        .fetch_all(&st.db)
        .await
        .unwrap_or_default();
    for h in hits {
        crate::audit::record(st, team, Some(project), Some(task), 0, "alerta_presupuesto", &h).await;
        for a in &admins {
            crate::notify::notify(st, *a, "budget", Some(team), Some(task), format!("💰 {h}")).await;
        }
    }
}

/// Qué umbral cruzó esta ejecución: pasó del 80 % o llegó al tope. `now` ya incluye `this`.
fn crossed(now: i64, this: i64, limit: i64) -> Option<&'static str> {
    if limit <= 0 {
        return None;
    }
    let before = now - this;
    if before < limit && now >= limit {
        Some("agotó su tope: no arranca otra ejecución hasta que lo subas")
    } else if before * 100 < limit * 80 && now * 100 >= limit * 80 {
        Some("llegó al 80 % del tope")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::crossed;

    #[test]
    fn umbrales() {
        assert_eq!(crossed(70, 10, 100), None);
        assert_eq!(crossed(80, 10, 100), Some("llegó al 80 % del tope"));
        assert_eq!(crossed(85, 5, 100), None, "ya estaba pasado el 80 %");
        assert_eq!(crossed(100, 10, 100), Some("agotó su tope: no arranca otra ejecución hasta que lo subas"));
        assert_eq!(crossed(130, 60, 100), Some("agotó su tope: no arranca otra ejecución hasta que lo subas"));
        assert_eq!(crossed(500, 10, 0), None);
    }
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
    crate::extras::log_task(&st, task, user.id, format!("fijó el tope de la tarea en {} tokens", b.tokens.clamp(0, MAX_TOKENS))).await;
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
    crate::extras::log(&st, project, user.id, format!("fijó la cuota mensual del proyecto en {} tokens", b.tokens.clamp(0, MAX_TOKENS))).await;
    Ok(Json(json!({ "quota": b.tokens.clamp(0, MAX_TOKENS) })))
}

async fn set_team_quota(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<Tokens>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Owner).await?;
    sqlx::query("UPDATE teams SET token_quota = ? WHERE id = ?").bind(b.tokens.clamp(0, MAX_TOKENS)).bind(team).execute(&st.db).await?;
    crate::audit::record(&st, team, None, None, user.id, "accion", &format!("fijó la cuota mensual del equipo en {} tokens", b.tokens.clamp(0, MAX_TOKENS))).await;
    Ok(Json(json!({ "quota": b.tokens.clamp(0, MAX_TOKENS) })))
}

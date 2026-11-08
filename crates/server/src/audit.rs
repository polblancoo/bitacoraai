//! Auditoría exportable: una línea de tiempo única del equipo (qué hizo cada persona e IA, permisos dados,
//! revisiones, cambios de tope, gasto de cada ejecución y movimientos del chat) en CSV o JSON.
//! `audit_log` no se recorta nunca (a diferencia de `activity`, que guarda 500 por proyecto).
use crate::{auth::AuthUser, error::{AppError, Result}, extras::csv_cell, routes, AppState};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tm_core::{Id, Role};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/teams/{id}/audit/export", get(export))
}

/// Anota un movimiento en la auditoría permanente. Nunca falla hacia afuera: auditar no debe frenar la acción.
pub async fn record(st: &AppState, team: Id, project: Option<Id>, task: Option<Id>, user: Id, kind: &str, text: &str) {
    let _ = sqlx::query("INSERT INTO audit_log (at, team_id, project_id, task_id, user_id, kind, text) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(Utc::now())
        .bind(team)
        .bind(project)
        .bind(task)
        .bind(user)
        .bind(kind)
        .bind(text.chars().take(1000).collect::<String>())
        .execute(&st.db)
        .await;
}

#[derive(Serialize, Default)]
struct Line {
    fecha: String,
    origen: &'static str,
    quien: String,
    accion: String,
    detalle: String,
    proyecto: String,
    tarea: Option<Id>,
    ip: String,
    modelo: String,
    tokens: i64,
    costo_usd: f64,
}

#[derive(Deserialize)]
struct Params {
    format: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

fn day(s: &Option<String>) -> Result<Option<NaiveDate>> {
    match s.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d").map(Some).map_err(|_| AppError::bad("fecha inválida: usá AAAA-MM-DD")),
    }
}

/// Solo administradores y dueños del equipo (las IAs no).
async fn export(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Query(q): Query<Params>) -> Result<Response> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    let from = day(&q.from)?.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
    // el último día entra completo: se corta antes del día siguiente
    let to = day(&q.to)?.map(|d| (d + Duration::days(1)).format("%Y-%m-%d").to_string()).unwrap_or_else(|| "9999-12-31".into());
    let mut lines: Vec<Line> = Vec::new();

    for r in sqlx::query(
        "SELECT a.at, CASE WHEN a.user_id = 0 THEN 'sistema' ELSE COALESCE(u.username, '(borrado)') END AS who, a.kind, a.text, COALESCE(p.name, '') AS project, a.task_id \
         FROM audit_log a LEFT JOIN users u ON u.id = a.user_id LEFT JOIN projects p ON p.id = a.project_id \
         WHERE a.team_id = ? AND a.at >= ? AND a.at < ? ORDER BY a.id",
    )
    .bind(team).bind(&from).bind(&to)
    .fetch_all(&st.db).await?
    {
        lines.push(Line { fecha: r.get("at"), origen: "equipo", quien: r.get("who"), accion: r.get("kind"), detalle: r.get("text"), proyecto: r.get("project"), tarea: r.get("task_id"), ..Default::default() });
    }
    for r in sqlx::query(
        "SELECT r.started_at, COALESCE(u.username, '(borrada)') AS who, r.model, r.ok, r.seconds, r.task_id, p.name AS project, \
                (r.input_tokens + r.output_tokens + r.cache_create) AS tokens, r.cost_usd \
         FROM task_runs r JOIN projects p ON p.id = r.project_id LEFT JOIN users u ON u.id = r.bot_id \
         WHERE p.team_id = ? AND r.started_at >= ? AND r.started_at < ? ORDER BY r.id",
    )
    .bind(team).bind(&from).bind(&to)
    .fetch_all(&st.db).await?
    {
        let ok: i64 = r.get("ok");
        let secs: i64 = r.get("seconds");
        lines.push(Line {
            fecha: r.get("started_at"), origen: "ia", quien: r.get("who"), accion: if ok != 0 { "ejecucion_ok" } else { "ejecucion_fallida" }.into(),
            detalle: format!("{secs} s"), proyecto: r.get("project"), tarea: r.get("task_id"), modelo: r.get("model"), tokens: r.get("tokens"), costo_usd: r.get("cost_usd"), ..Default::default()
        });
    }
    for r in sqlx::query(
        "SELECT a.at, COALESCE((SELECT username FROM users WHERE id = a.user_id), '?') AS who, a.action, a.detail, a.ip \
         FROM chat_audit a WHERE a.team_id = ? AND a.at >= ? AND a.at < ? ORDER BY a.id",
    )
    .bind(team).bind(&from).bind(&to)
    .fetch_all(&st.db).await?
    {
        lines.push(Line { fecha: r.get("at"), origen: "chat", quien: r.get("who"), accion: r.get("action"), detalle: r.get("detail"), ip: r.get("ip"), ..Default::default() });
    }
    lines.sort_by(|a, b| a.fecha.cmp(&b.fecha));

    let (body, mime, ext) = match q.format.as_deref().unwrap_or("csv") {
        "json" => (serde_json::to_string_pretty(&lines).map_err(|_| AppError::bad("no se pudo exportar"))?, "application/json", "json"),
        "csv" => {
            let mut out = String::from("\u{feff}fecha,origen,quien,accion,detalle,proyecto,tarea,ip,modelo,tokens,costo_usd\n");
            for l in &lines {
                let cells = [
                    csv_cell(&l.fecha), l.origen.into(), csv_cell(&l.quien), csv_cell(&l.accion), csv_cell(&l.detalle), csv_cell(&l.proyecto),
                    l.tarea.map(|t| t.to_string()).unwrap_or_default(), csv_cell(&l.ip), csv_cell(&l.modelo), l.tokens.to_string(), format!("{:.4}", l.costo_usd),
                ];
                out.push_str(&cells.join(","));
                out.push('\n');
            }
            (out, "text/csv; charset=utf-8", "csv")
        }
        _ => return Err(AppError::bad("formato inválido: csv o json")),
    };
    // quedó asentado quién se llevó la auditoría
    record(&st, team, None, None, user.id, "auditoria_exportada", &format!("{} líneas, formato {ext}", lines.len())).await;
    let mut resp = body.into_response();
    resp.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"auditoria-equipo-{team}.{ext}\"")) {
        resp.headers_mut().insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(resp)
}

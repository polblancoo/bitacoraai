//! Informes automáticos: cada persona se suscribe, por equipo, a un resumen diario o semanal (qué hicieron las IAs,
//! cuánto gastaron y qué quedó trabado). Se entrega como cualquier novedad: en la bandeja y, si lo tiene, por Telegram y email.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    notify::notify_via,
    routes::require,
    schedule::next_after,
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::{Id, Role};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/teams/{id}/report", get(get_sub).put(put_sub).delete(del_sub)).route("/api/teams/{id}/report/now", post(send_now))
}

pub fn spawn_reports(st: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            run_due(&st).await;
        }
    });
}

fn window(freq: &str) -> Duration {
    if freq == "weekly" { Duration::days(7) } else { Duration::days(1) }
}

async fn run_due(st: &AppState) {
    let now = Utc::now();
    let Ok(rows) = sqlx::query("SELECT user_id, team_id, freq, weekday, at_minute, tz_offset, via_telegram, via_email FROM report_subs WHERE active = 1 AND next_run <= ?")
        .bind(now)
        .fetch_all(&st.db)
        .await
    else {
        return;
    };
    for r in rows {
        let (user, team): (Id, Id) = (r.get("user_id"), r.get("team_id"));
        let freq: String = r.get("freq");
        let next = next_after(&freq, r.get("weekday"), 1, r.get("at_minute"), r.get("tz_offset"), now);
        // se avanza antes de enviar: una caída del servidor no repite el informe
        if sqlx::query("UPDATE report_subs SET next_run = ?, last_run = ? WHERE user_id = ? AND team_id = ?").bind(next).bind(now).bind(user).bind(team).execute(&st.db).await.is_err() {
            continue;
        }
        // si ya no es miembro, se da de baja
        if require(st, team, user, Role::Viewer).await.is_err() {
            let _ = sqlx::query("DELETE FROM report_subs WHERE user_id = ? AND team_id = ?").bind(user).bind(team).execute(&st.db).await;
            continue;
        }
        if let Ok(text) = build(st, team, &freq, r.get("tz_offset")).await {
            notify_via(st, user, "report", Some(team), None, text, r.get::<i64, _>("via_telegram") != 0, r.get::<i64, _>("via_email") != 0).await;
        }
    }
}

fn fmt_usd(v: f64) -> String {
    format!("USD {v:.2}")
}

fn short(s: &str, n: usize) -> String {
    let t: String = s.chars().take(n).collect();
    if s.chars().count() > n { format!("{t}…") } else { t }
}

/// Arma el texto del informe del equipo para el último día o la última semana.
async fn build(st: &AppState, team: Id, freq: &str, tz: i64) -> Result<String> {
    let now = Utc::now();
    let since = now - window(freq);
    let name: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    let local = |d: DateTime<Utc>| (d + Duration::minutes(tz)).format("%d/%m %H:%M").to_string();
    let mut out = format!("📊 Informe {} · {name}\n{} → {}\n", if freq == "weekly" { "semanal" } else { "diario" }, local(since), local(now));

    // tareas terminadas, por quién las tenía asignada
    let done = sqlx::query(
        "SELECT COALESCE(u.username, 'sin responsable') AS who, COALESCE(u.is_bot, 0) AS bot, COUNT(*) AS n FROM tasks t JOIN projects p ON p.id = t.project_id \
         LEFT JOIN users u ON u.id = t.assignee_id WHERE p.team_id = ? AND t.status = 'done' AND t.updated_at >= ? AND t.title NOT LIKE '🙋 Pedido de #%' \
         GROUP BY who, bot ORDER BY n DESC",
    )
    .bind(team)
    .bind(since)
    .fetch_all(&st.db)
    .await?;
    let total: i64 = done.iter().map(|r| r.get::<i64, _>("n")).sum();
    out.push_str(&format!("\n✅ Tareas terminadas: {total}\n"));
    for r in done.iter().take(8) {
        out.push_str(&format!("  • {}{}: {}\n", if r.get::<i64, _>("bot") == 1 { "🤖 " } else { "" }, r.get::<String, _>("who"), r.get::<i64, _>("n")));
    }

    // trabajo y gasto de las IAs
    let runs = sqlx::query(
        "SELECT COALESCE(u.username, 'IA borrada') AS who, COUNT(*) AS n, COALESCE(SUM(1 - r.ok), 0) AS bad, COALESCE(SUM(r.seconds), 0) AS secs, \
         COALESCE(SUM(r.cost_usd), 0.0) AS cost FROM task_runs r JOIN projects p ON p.id = r.project_id LEFT JOIN users u ON u.id = r.bot_id \
         WHERE p.team_id = ? AND r.started_at >= ? GROUP BY who ORDER BY cost DESC, n DESC",
    )
    .bind(team)
    .bind(since)
    .fetch_all(&st.db)
    .await?;
    out.push_str("\n🤖 Trabajo de las IAs\n");
    if runs.is_empty() {
        out.push_str("  (ninguna ejecución en el período)\n");
    } else {
        let (n, bad, cost): (i64, i64, f64) = runs.iter().fold((0, 0, 0.0), |a, r| (a.0 + r.get::<i64, _>("n"), a.1 + r.get::<i64, _>("bad"), a.2 + r.get::<f64, _>("cost")));
        out.push_str(&format!("  {n} ejecuciones ({bad} con error) · gasto {}\n", fmt_usd(cost)));
        for r in runs.iter().take(6) {
            out.push_str(&format!("  • {}: {} ejec., {} min, {}\n", r.get::<String, _>("who"), r.get::<i64, _>("n"), r.get::<i64, _>("secs") / 60, fmt_usd(r.get::<f64, _>("cost"))));
        }
    }

    // lo que quedó trabado
    let stale = now - Duration::hours(24);
    let stuck = sqlx::query(
        "SELECT t.id, t.title, COALESCE(u.username, '—') AS who FROM tasks t JOIN projects p ON p.id = t.project_id LEFT JOIN users u ON u.id = t.assignee_id \
         WHERE p.team_id = ? AND t.status = 'doing' AND t.updated_at < ? ORDER BY t.updated_at LIMIT 5",
    )
    .bind(team)
    .bind(stale)
    .fetch_all(&st.db)
    .await?;
    let asks = sqlx::query(
        "SELECT t.id, t.title FROM tasks t JOIN projects p ON p.id = t.project_id WHERE p.team_id = ? AND t.status != 'done' AND t.title LIKE '🙋 Pedido de #%' ORDER BY t.id LIMIT 5",
    )
    .bind(team)
    .fetch_all(&st.db)
    .await?;
    let count = |sql: &'static str| {
        let st = st.clone();
        async move { sqlx::query_scalar::<_, i64>(sql).bind(team).bind(now).fetch_one(&st.db).await.unwrap_or(0) }
    };
    let overdue = count("SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE p.team_id = ? AND t.status != 'done' AND t.due IS NOT NULL AND t.due < ?").await;
    let unassigned = count("SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE p.team_id = ? AND t.status = 'todo' AND t.assignee_id IS NULL AND ? IS NOT NULL").await;
    out.push_str("\n⚠️ Para mirar\n");
    if stuck.is_empty() && asks.is_empty() && overdue == 0 && unassigned == 0 {
        out.push_str("  Nada trabado 👌\n");
    }
    for r in &stuck {
        out.push_str(&format!("  • En curso hace más de un día: #{} «{}» ({})\n", r.get::<Id, _>("id"), short(&r.get::<String, _>("title"), 50), r.get::<String, _>("who")));
    }
    for r in &asks {
        out.push_str(&format!("  • 🙋 Pedido sin responder: #{} {}\n", r.get::<Id, _>("id"), short(&r.get::<String, _>("title"), 60)));
    }
    if overdue > 0 {
        out.push_str(&format!("  • {overdue} tareas vencidas\n"));
    }
    if unassigned > 0 {
        out.push_str(&format!("  • {unassigned} tareas por hacer sin responsable\n"));
    }
    Ok(out)
}

fn sub_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    json!({
        "freq": r.get::<String, _>("freq"), "weekday": r.get::<i64, _>("weekday"), "at_minute": r.get::<i64, _>("at_minute"), "tz_offset": r.get::<i64, _>("tz_offset"),
        "active": r.get::<i64, _>("active") != 0, "via_telegram": r.get::<i64, _>("via_telegram") != 0, "via_email": r.get::<i64, _>("via_email") != 0, "next_run": r.get::<DateTime<Utc>, _>("next_run"), "last_run": r.get::<Option<DateTime<Utc>>, _>("last_run"),
    })
}

async fn get_sub(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    require(&st, team, user.id, Role::Viewer).await?;
    let row = sqlx::query("SELECT * FROM report_subs WHERE user_id = ? AND team_id = ?").bind(user.id).bind(team).fetch_optional(&st.db).await?;
    let email: Option<i64> = sqlx::query_scalar("SELECT email_verified FROM users WHERE id = ? AND email IS NOT NULL").bind(user.id).fetch_optional(&st.db).await?;
    let tg: Option<i64> = sqlx::query_scalar("SELECT telegram_chat_id FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(json!({
        "sub": row.as_ref().map(sub_json),
        "email": email == Some(1) && st.mailer.is_some(), "telegram": tg.is_some() && st.telegram.is_some(),
    })))
}

#[derive(Deserialize)]
struct Body {
    freq: String,
    #[serde(default)]
    weekday: i64,
    #[serde(default = "nine")]
    at_minute: i64,
    #[serde(default)]
    tz_offset: i64,
    #[serde(default = "yes")]
    active: bool,
    #[serde(default = "yes")]
    via_telegram: bool,
    #[serde(default = "yes")]
    via_email: bool,
}

fn nine() -> i64 { 540 }
fn yes() -> bool { true }

async fn put_sub(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<Body>) -> Result<Json<Value>> {
    user.human_only()?;
    require(&st, team, user.id, Role::Viewer).await?;
    if !matches!(b.freq.as_str(), "daily" | "weekly") || !(0..=6).contains(&b.weekday) || !(0..1440).contains(&b.at_minute) || !(-840..=840).contains(&b.tz_offset) {
        return Err(AppError::bad("frecuencia, día u hora inválidos"));
    }
    let next = next_after(&b.freq, b.weekday, 1, b.at_minute, b.tz_offset, Utc::now());
    sqlx::query(
        "INSERT INTO report_subs (user_id, team_id, freq, weekday, at_minute, tz_offset, active, via_telegram, via_email, next_run) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(user_id, team_id) DO UPDATE SET freq = excluded.freq, weekday = excluded.weekday, at_minute = excluded.at_minute, \
         tz_offset = excluded.tz_offset, active = excluded.active, via_telegram = excluded.via_telegram, via_email = excluded.via_email, next_run = excluded.next_run",
    )
    .bind(user.id)
    .bind(team)
    .bind(&b.freq)
    .bind(b.weekday)
    .bind(b.at_minute)
    .bind(b.tz_offset)
    .bind(i64::from(b.active))
    .bind(i64::from(b.via_telegram))
    .bind(i64::from(b.via_email))
    .bind(next)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({ "next_run": next })))
}

async fn del_sub(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    sqlx::query("DELETE FROM report_subs WHERE user_id = ? AND team_id = ?").bind(user.id).bind(team).execute(&st.db).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct NowBody {
    #[serde(default)]
    freq: Option<String>,
    #[serde(default)]
    tz_offset: i64,
    #[serde(default = "yes")]
    via_telegram: bool,
    #[serde(default = "yes")]
    via_email: bool,
}

/// «Enviarme uno ahora»: arma el informe del período y lo manda por los mismos canales; devuelve el texto para verlo.
async fn send_now(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<NowBody>) -> Result<Json<Value>> {
    user.human_only()?;
    require(&st, team, user.id, Role::Viewer).await?;
    if st.user_limiter.hit(&format!("report-now:{}", user.id), 10, std::time::Duration::from_secs(3600)).is_err() {
        return Err(AppError::bad("Pediste muchos informes seguidos: probá en un rato"));
    }
    let freq = b.freq.as_deref().filter(|f| matches!(*f, "daily" | "weekly")).unwrap_or("daily");
    let text = build(&st, team, freq, b.tz_offset.clamp(-840, 840)).await?;
    notify_via(&st, user.id, "report", Some(team), None, text.clone(), b.via_telegram, b.via_email).await;
    let mut sent = vec!["bandeja"];
    if b.via_telegram && st.telegram.is_some() && sqlx::query_scalar::<_, Option<i64>>("SELECT telegram_chat_id FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await.ok().flatten().is_some() {
        sent.push("Telegram");
    }
    if b.via_email && st.mailer.is_some() && sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE id = ? AND email_verified = 1 AND email IS NOT NULL").bind(user.id).fetch_one(&st.db).await.unwrap_or(0) > 0 {
        sent.push("email");
    }
    Ok(Json(json!({ "text": text, "sent": sent })))
}

//! Resumen de novedades: en vez de mandar un Telegram y un email por cada aviso, el servidor junta los avisos y manda
//! uno solo por canal a la hora que cada persona elija (por defecto, una vez al día a las 9:00). También se puede volver
//! a recibir cada aviso al instante. La bandeja de la app siempre se llena en el momento.
use crate::{auth::AuthUser, error::{AppError, Result}, schedule::next_after, AppState};
use axum::{extract::State, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::Id;

const DEFAULT_HOUR: i64 = 9;
const MAX_ITEMS: usize = 40;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/me/notify-prefs", get(get_prefs).put(put_prefs))
}

#[derive(Clone)]
pub struct Prefs {
    pub digest: bool,
    pub hour: i64,
    pub tz: i64,
    pub last: Option<DateTime<Utc>>,
    /// 0 = una vez por día a `hour`; N = cada N horas
    pub every_h: i64,
    pub mute_until: Option<DateTime<Utc>>,
    /// modo silencioso o pausa vigente: nada sale por Telegram ni email (la bandeja de la app sí se llena)
    pub silent: bool,
}

fn server_tz() -> i64 {
    i64::from(chrono::Local::now().offset().local_minus_utc()) / 60
}

pub async fn prefs(st: &AppState, user: Id) -> Prefs {
    let row = sqlx::query("SELECT mode, hour, tz_offset, last_digest, every_h, mute_until FROM notif_prefs WHERE user_id = ?").bind(user).fetch_optional(&st.db).await.ok().flatten();
    match row {
        Some(r) => {
            let mode: String = r.get("mode");
            let mute_until: Option<DateTime<Utc>> = r.get("mute_until");
            Prefs {
                digest: mode != "instant" && mode != "silent",
                hour: r.get("hour"),
                tz: r.get::<Option<i64>, _>("tz_offset").unwrap_or_else(server_tz),
                last: r.get("last_digest"),
                every_h: r.get("every_h"),
                silent: mode == "silent" || mute_until.is_some_and(|m| m > Utc::now()),
                mute_until,
            }
        }
        None => Prefs { digest: true, hour: DEFAULT_HOUR, tz: server_tz(), last: None, every_h: 0, mute_until: None, silent: false },
    }
}

/// Los pedidos de ayuda de una IA (🙋) y los informes (que ya tienen su propio horario) no esperan al resumen.
pub fn instant_kind(kind: &str, text: &str) -> bool {
    kind == "report" || text.contains('🙋')
}

async fn get_prefs(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let p = prefs(&st, user.id).await;
    let mode = if p.digest { "digest" } else if p.silent { "silent" } else { "instant" };
    let muted = p.mute_until.filter(|m| *m > Utc::now());
    Ok(Json(json!({ "mode": mode, "hour": p.hour, "tz_offset": p.tz, "every_h": p.every_h, "mute_until": muted })))
}

#[derive(Deserialize)]
struct PrefsBody {
    mode: String,
    hour: i64,
    tz_offset: i64,
    #[serde(default)]
    every_h: i64,
    /// pausa por N horas desde ahora (0 = quitar la pausa)
    #[serde(default)]
    mute_hours: i64,
}

async fn put_prefs(State(st): State<AppState>, user: AuthUser, Json(b): Json<PrefsBody>) -> Result<Json<Value>> {
    user.human_only()?;
    if !matches!(b.mode.as_str(), "digest" | "instant" | "silent") || !matches!(b.every_h, 0 | 1 | 2 | 3 | 6 | 12) || !(0..=24 * 30).contains(&b.mute_hours) || !(0..24).contains(&b.hour) || !(-840..=840).contains(&b.tz_offset) {
        return Err(AppError::bad("preferencia inválida"));
    }
    sqlx::query(
        "INSERT INTO notif_prefs (user_id, mode, hour, tz_offset, last_digest, every_h, mute_until) VALUES (?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(user_id) DO UPDATE SET mode = excluded.mode, hour = excluded.hour, tz_offset = excluded.tz_offset, every_h = excluded.every_h, mute_until = excluded.mute_until",
    )
    .bind(user.id)
    .bind(&b.mode)
    .bind(b.hour)
    .bind(b.tz_offset)
    .bind(Utc::now())
    .bind(b.every_h)
    .bind((b.mute_hours > 0).then(|| Utc::now() + chrono::Duration::hours(b.mute_hours)))
    .execute(&st.db)
    .await?;
    // si vuelve a "uno por uno", lo que estaba esperando sale ahora
    if b.mode == "instant" {
        flush(&st, user.id).await;
    }
    Ok(Json(json!({ "ok": true })))
}

pub fn spawn(st: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            run_due(&st).await;
        }
    });
}

async fn run_due(st: &AppState) {
    let now = Utc::now();
    let users: Vec<Id> = sqlx::query_scalar("SELECT DISTINCT user_id FROM notifications WHERE ext_pending > 0").fetch_all(&st.db).await.unwrap_or_default();
    for user in users {
        let p = prefs(st, user).await;
        let Some(last) = p.last else {
            // primera vez que se ve a esta persona: el resumen sale en el próximo horario, no de golpe
            let _ = sqlx::query("INSERT INTO notif_prefs (user_id, last_digest) VALUES (?, ?) ON CONFLICT(user_id) DO UPDATE SET last_digest = excluded.last_digest")
                .bind(user)
                .bind(now)
                .execute(&st.db)
                .await;
            continue;
        };
        let due = if p.every_h > 0 { last + chrono::Duration::hours(p.every_h) } else { next_after("daily", 0, 1, p.hour * 60, p.tz, last) };
        if !p.digest || p.silent || due > now {
            continue;
        }
        let _ = sqlx::query("UPDATE notif_prefs SET last_digest = ? WHERE user_id = ?").bind(now).bind(user).execute(&st.db).await;
        flush(st, user).await;
    }
}

/// Manda lo que esté esperando (un Telegram y un email con todo junto) y lo da por enviado.
pub async fn flush(st: &AppState, user: Id) {
    let rows = sqlx::query("SELECT id, text, ext_pending FROM notifications WHERE user_id = ? AND ext_pending > 0 ORDER BY id").bind(user).fetch_all(&st.db).await.unwrap_or_default();
    if rows.is_empty() {
        return;
    }
    let pick = |bit: i64| -> Vec<String> { rows.iter().filter(|r| r.get::<i64, _>("ext_pending") & bit != 0).map(|r| r.get::<String, _>("text")).collect() };
    let _ = sqlx::query("UPDATE notifications SET ext_pending = 0 WHERE user_id = ? AND ext_pending > 0").bind(user).execute(&st.db).await;
    let body = |items: &[String]| -> String {
        let mut out = format!("{} novedades:\n", items.len());
        for t in items.iter().take(MAX_ITEMS) {
            let t = crate::notify::short(t);
            let mut ls = t.lines();
            let first = ls.next().unwrap_or("");
            let more = ls.next().map(|l| format!(" — {}", l.chars().take(120).collect::<String>())).unwrap_or_default();
            out.push_str(&format!("\n• {first}{more}"));
        }
        if items.len() > MAX_ITEMS {
            out.push_str(&format!("\n… y {} más en la bandeja de la app.", items.len() - MAX_ITEMS));
        }
        out
    };
    let (tg_items, mail_items) = (pick(1), pick(2));
    if !tg_items.is_empty() {
        if let Some(tg) = st.telegram.get() {
            let chat: Option<i64> = sqlx::query_scalar::<_, Option<i64>>("SELECT telegram_chat_id FROM users WHERE id = ?").bind(user).fetch_optional(&st.db).await.ok().flatten().flatten();
            if let Some(chat) = chat {
                let msg = body(&tg_items);
                if let Err(e) = tg.send(chat, &msg).await {
                    tracing::warn!("resumen: no se pudo enviar a Telegram: {e}");
                }
            }
        }
    }
    if !mail_items.is_empty() {
        if let Some(mailer) = st.mailer.get() {
            let to: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = ? AND email_verified = 1 AND email IS NOT NULL").bind(user).fetch_optional(&st.db).await.ok().flatten();
            if let Some(to) = to {
                let subject = format!("{} novedades", mail_items.len());
                if let Err(e) = mailer.send(&to, &subject, body(&mail_items)).await {
                    tracing::warn!("resumen: no se pudo enviar el email: {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedidos_de_ia_e_informes_salen_al_instante() {
        assert!(instant_kind("assigned", "Ana te asignó «🙋 Pedido de #3: algo»"));
        assert!(instant_kind("report", "Informe"));
        assert!(!instant_kind("assigned", "Ana te asignó «Revisar el login»"));
        assert!(!instant_kind("mention", "Ana te mencionó"));
    }

    #[test]
    fn el_resumen_sale_a_la_hora_elegida_en_la_zona_de_la_persona() {
        // 9:00 en UTC-3 = 12:00 UTC
        let last: DateTime<Utc> = "2026-11-01T13:00:00Z".parse().unwrap();
        let next = next_after("daily", 0, 1, 9 * 60, -180, last);
        assert_eq!(next, "2026-11-02T12:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }
}

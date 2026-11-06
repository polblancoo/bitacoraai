//! Novedades dentro de la app y emails por SMTP (opcional).
use crate::{limits, AppState};
use lettre::{
    message::{header, header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use sqlx::Row;
use std::time::Duration;
use tm_core::{Event, Id, Notification};

/// Cliente SMTP; existe solo si el servidor tiene `TM_SMTP_URL` y `TM_SMTP_FROM`.
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Mailer {
    /// `TM_SMTP_URL`: `smtps://usuario:clave@host:465`, `smtp://host:587?tls=required` o `smtp://localhost:2525`.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("TM_SMTP_URL").ok()?;
        let from: Mailbox = match std::env::var("TM_SMTP_FROM").ok()?.parse() {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("TM_SMTP_FROM inválido: {e}");
                return None;
            }
        };
        match AsyncSmtpTransport::<Tokio1Executor>::from_url(&url) {
            Ok(b) => {
                let b = match (std::env::var("TM_SMTP_USER"), std::env::var("TM_SMTP_PASSWORD")) {
                    (Ok(u), Ok(p)) => b.credentials(Credentials::new(u, p)),
                    _ => b,
                };
                tracing::info!("emails habilitados (SMTP)");
                Some(Self { transport: b.build(), from })
            }
            Err(e) => {
                tracing::error!("TM_SMTP_URL inválida: {e}");
                None
            }
        }
    }

    pub async fn send(&self, to: &str, subject: &str, body: String) -> Result<(), String> {
        let to: Mailbox = to.parse().map_err(|e| format!("destinatario inválido: {e}"))?;
        // texto plano + HTML con encabezado de la marca (el logo va dentro del mensaje, sin descargar nada)
        let logo = SinglePart::builder().header(ContentType::parse("image/png").map_err(|e| e.to_string())?).header(header::ContentDisposition::inline()).header(header::ContentId::from("<logo>".to_string())).body(LOGO.to_vec());
        let html = SinglePart::html(html_body(&body));
        let msg = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(subject)
            .multipart(MultiPart::alternative().singlepart(SinglePart::plain(body)).multipart(MultiPart::related().singlepart(html).singlepart(logo)))
            .map_err(|e| e.to_string())?;
        self.transport.send(msg).await.map(|_| ()).map_err(|e| e.to_string())
    }
}

/// Nombre de la app en lo que se ve (pantallas, correos y avisos). Los repositorios y los paquetes conservan su nombre.
pub const APP_NAME: &str = "CrewHelm";
const LOGO: &[u8] = include_bytes!("../web/icon-192.png");

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Cuerpo HTML del correo: banda con el logo y el nombre, y el texto del aviso respetando los saltos de línea.
fn html_body(text: &str) -> String {
    format!(
        "<!doctype html><html><body style=\"margin:0;background:#f3f4f8;font-family:-apple-system,Segoe UI,Roboto,Arial,sans-serif\">\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\"><tr><td align=\"center\" style=\"padding:24px 12px\">\
         <table role=\"presentation\" width=\"560\" cellpadding=\"0\" cellspacing=\"0\" style=\"max-width:100%;background:#ffffff;border-radius:12px;overflow:hidden\">\
         <tr><td style=\"background:#1a1b26;padding:16px 22px\"><img src=\"cid:logo\" width=\"36\" height=\"36\" alt=\"\" style=\"vertical-align:middle;border-radius:8px\"> \
         <span style=\"color:#ffffff;font-size:18px;font-weight:600;vertical-align:middle;margin-left:8px\">{APP_NAME}</span></td></tr>\
         <tr><td style=\"padding:22px;color:#1f2330;font-size:15px;line-height:1.5;white-space:pre-wrap\">{}</td></tr>\
         </table></td></tr></table></body></html>",
        esc(text)
    )
}

/// Texto corto para Telegram, correo y resumen: sin el "X comentó en", sin ruido técnico de las herramientas
/// y con el estado (✅ ❌ ▶ …) adelante. La bandeja de la app conserva el texto completo.
pub fn short(text: &str) -> String {
    const NOISE: [&str; 5] = ["DeprecationWarning", "(node:", "--trace-deprecation", "ExperimentalWarning", "--trace-warnings"];
    let clean = |s: &str| -> String {
        let lines: Vec<&str> = s.lines().map(str::trim).filter(|l| !l.is_empty() && !NOISE.iter().any(|n| l.contains(n))).collect();
        lines.join("\n")
    };
    let cut = |s: String, n: usize| -> String {
        if s.chars().count() <= n { s } else { format!("{}…", s.chars().take(n).collect::<String>().trim_end()) }
    };
    // «Alguien comentó en «Tarea»: ✅ Terminé…» -> «✅ «Tarea» · Alguien\nTerminé…»
    if let Some((who, rest)) = text.split_once(" comentó en «") {
        if let Some((title, msg)) = rest.split_once("»: ") {
            let msg = clean(msg);
            let mut it = msg.chars();
            let lead = it.next().filter(|c| !c.is_alphanumeric() && !c.is_whitespace() && !matches!(c, '«' | '¿' | '¡' | '`' | '*' | '#' | '-'));
            let body = match lead {
                Some(_) => it.as_str().trim_start_matches('\u{fe0f}').trim().to_string(),
                None => msg.clone(),
            };
            let head = format!("{} «{title}» · {who}", lead.map(String::from).unwrap_or_else(|| "💬".into()));
            return if body.is_empty() { head } else { format!("{head}\n{}", cut(body, 280)) };
        }
    }
    // «🐙 La IA terminó «X». Si dejó su trabajo en la rama R, abrí el Pull Request…» -> corto
    if let (Some(a), Some(b)) = (text.find("La IA terminó «"), text.find("en la rama ")) {
        let title = text[a + "La IA terminó «".len()..].split('»').next().unwrap_or("");
        let branch = text[b + "en la rama ".len()..].split(',').next().unwrap_or("");
        return format!("🐙 «{title}» terminó · rama {branch}: abrí el Pull Request desde la tarea");
    }
    cut(clean(text), 300)
}

/// Valida un email de forma conservadora (sin espacios, con un `@`, dominio con punto, sin saltos de línea).
pub fn valid_email(e: &str) -> bool {
    let Some((local, domain)) = e.split_once('@') else { return false };
    e.len() <= limits::EMAIL_MAX
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !e.contains(|c: char| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | ',' | ';' | '"'))
        && e.matches('@').count() == 1
}

pub async fn username_of(st: &AppState, id: Id) -> String {
    sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|_| "alguien".into())
}

/// Registra una novedad, la empuja por WebSocket y, si el destinatario tiene un email verificado y
/// el servidor tiene SMTP, la envía por correo (con tope por hora).
pub async fn notify(st: &AppState, user: Id, kind: &str, team: Option<Id>, task: Option<Id>, text: String) {
    notify_via(st, user, kind, team, task, text, true, true).await;
}

/// Igual que `notify`, pero se puede apagar Telegram o el email (la bandeja de la app siempre se llena).
#[allow(clippy::too_many_arguments)]
pub async fn notify_via(st: &AppState, user: Id, kind: &str, team: Option<Id>, task: Option<Id>, text: String, via_tg: bool, via_mail: bool) {
    let now = chrono::Utc::now();
    // el mismo texto corto en la bandeja, Telegram y el correo
    let text = short(&text);
    // el mismo aviso repetido en pocos minutos (doble clic, reintentos) se registra una sola vez
    let repeated: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM notifications WHERE user_id = ? AND text = ? AND created_at > ?)")
        .bind(user)
        .bind(&text)
        .bind(now - chrono::Duration::minutes(10))
        .fetch_one(&st.db)
        .await
        .unwrap_or(false);
    if repeated {
        return;
    }
    // con resumen (lo normal), Telegram y email esperan al horario de la persona; la bandeja de la app se llena al instante
    let prefs = crate::digest::prefs(st, user).await;
    let silent = prefs.silent;
    let deferred = !silent && prefs.digest && !crate::digest::instant_kind(kind, &text);
    let pending = if deferred { i64::from(via_tg) | (i64::from(via_mail) << 1) } else { 0 };
    let id = match sqlx::query("INSERT INTO notifications (user_id, kind, team_id, task_id, text, read, created_at, ext_pending) VALUES (?, ?, ?, ?, ?, 0, ?, ?)")
        .bind(user)
        .bind(kind)
        .bind(team)
        .bind(task)
        .bind(&text)
        .bind(now)
        .bind(pending)
        .execute(&st.db)
        .await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            tracing::error!("notify: {e}");
            return;
        }
    };
    // poda: se conservan las últimas 200 por usuario
    let _ = sqlx::query("DELETE FROM notifications WHERE user_id = ? AND id NOT IN (SELECT id FROM notifications WHERE user_id = ? ORDER BY id DESC LIMIT 200)")
        .bind(user)
        .bind(user)
        .execute(&st.db)
        .await;
    st.emit_to(
        user,
        Event::Notification(Notification {
            id,
            user_id: user,
            kind: kind.into(),
            team_id: team,
            task_id: task,
            text: text.clone(),
            read: false,
            created_at: now,
        }),
    );

    if deferred || silent {
        return;
    }
    if let Some(tg) = st.telegram.clone().filter(|_| via_tg) {
        let chat: Option<i64> = sqlx::query_scalar::<_, Option<i64>>("SELECT telegram_chat_id FROM users WHERE id = ?")
            .bind(user)
            .fetch_optional(&st.db)
            .await
            .ok()
            .flatten()
            .flatten();
        if let Some(chat) = chat {
            if st.user_limiter.hit(&format!("tg-out:{user}"), 60, Duration::from_secs(3600)).is_ok() {
                let msg = text.clone();
                tokio::spawn(async move {
                    if let Err(e) = tg.send(chat, &msg).await {
                        tracing::warn!("no se pudo enviar a Telegram: {e}");
                    }
                });
            }
        }
    }

    let Some(mailer) = st.mailer.clone().filter(|_| via_mail) else { return };
    let Ok(Some(row)) = sqlx::query("SELECT email FROM users WHERE id = ? AND email_verified = 1 AND email IS NOT NULL")
        .bind(user)
        .fetch_optional(&st.db)
        .await
    else {
        return;
    };
    if st.email_limiter.hit(&user, limits::EMAILS_PER_HOUR, Duration::from_secs(3600)).is_err() {
        return;
    }
    let to: String = row.get("email");
    let subject = text.lines().next().unwrap_or("").chars().take(80).collect::<String>();
    tokio::spawn(async move {
        if let Err(e) = mailer.send(&to, &subject, text).await {
            tracing::warn!("no se pudo enviar el email: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::short;

    #[test]
    fn comentario_de_ia_sin_prefijo_ni_ruido() {
        let t = "GEMMINI comentó en «revisar»: ❌ No pude completarla: terminó con error (exit status: 1)\n\n(node:1) [DEP0040] DeprecationWarning: x\n(Use `node --trace-deprecation ...`)";
        assert_eq!(short(t), "❌ «revisar» · GEMMINI\nNo pude completarla: terminó con error (exit status: 1)");
        assert_eq!(short("op comentó en «t»: ✅ Terminé.\n\nHice X"), "✅ «t» · op\nTerminé.\nHice X");
        assert_eq!(short("pablo te asignó «x» (prioridad alta)"), "pablo te asignó «x» (prioridad alta)");
    }

    #[test]
    fn aviso_de_github_corto() {
        let t = "🐙 La IA terminó «ver». Si dejó su trabajo en la rama tm/tarea-9, abrí el Pull Request desde la tarea (botón «Abrir Pull Request»).";
        assert_eq!(short(t), "🐙 «ver» terminó · rama tm/tarea-9: abrí el Pull Request desde la tarea");
    }
}

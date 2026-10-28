//! Novedades dentro de la app y emails por SMTP (opcional).
use crate::{limits, AppState};
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
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
        let msg = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(subject)
            .body(body)
            .map_err(|e| e.to_string())?;
        self.transport.send(msg).await.map(|_| ()).map_err(|e| e.to_string())
    }
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
    let now = chrono::Utc::now();
    let id = match sqlx::query("INSERT INTO notifications (user_id, kind, team_id, task_id, text, read, created_at) VALUES (?, ?, ?, ?, ?, 0, ?)")
        .bind(user)
        .bind(kind)
        .bind(team)
        .bind(task)
        .bind(&text)
        .bind(now)
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

    if let Some(tg) = st.telegram.clone() {
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

    let Some(mailer) = st.mailer.clone() else { return };
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
    let subject = format!("[Rust TUI Manager] {}", text.chars().take(80).collect::<String>().replace(['\r', '\n'], " "));
    tokio::spawn(async move {
        if let Err(e) = mailer.send(&to, &subject, format!("{text}\n\n— Rust TUI Manager")).await {
            tracing::warn!("no se pudo enviar el email: {e}");
        }
    });
}

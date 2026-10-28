//! Bot de Telegram (opcional): vincula cuentas con un código y entrega las novedades.
//! Se activa con `TM_TELEGRAM_TOKEN` (el token que da @BotFather). Recibe mensajes por long polling,
//! así que no necesita una URL pública. `TM_TELEGRAM_API` permite apuntar a otra API (pruebas).
use crate::{auth::sha256_hex, AppState};
use chrono::Utc;
use serde::Deserialize;
use sqlx::Row;
use std::time::Duration;

pub struct Telegram {
    token: String,
    api: String,
    pub bot: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct Reply<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}
#[derive(Deserialize)]
struct Message {
    chat: Chat,
    text: Option<String>,
}
#[derive(Deserialize)]
struct Chat {
    id: i64,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Deserialize)]
struct Me {
    username: Option<String>,
}

/// Los errores de reqwest incluyen la URL, y la URL lleva el token: se quita antes de loguear.
fn clean(e: reqwest::Error) -> String {
    e.without_url().to_string()
}

impl Telegram {
    pub async fn from_env() -> Option<Self> {
        let token = std::env::var("TM_TELEGRAM_TOKEN").ok().filter(|t| !t.trim().is_empty())?;
        let api = std::env::var("TM_TELEGRAM_API").unwrap_or_else(|_| "https://api.telegram.org".into());
        let http = reqwest::Client::builder().timeout(Duration::from_secs(40)).build().ok()?;
        let mut t = Self { token: token.trim().into(), api: api.trim_end_matches('/').into(), bot: String::new(), http };
        match t.call::<Me>("getMe", serde_json::json!({})).await {
            Ok(me) => {
                t.bot = me.username.unwrap_or_default();
                tracing::info!("Telegram habilitado (bot @{})", t.bot);
                Some(t)
            }
            Err(e) => {
                tracing::error!("TM_TELEGRAM_TOKEN no funciona: {e}");
                None
            }
        }
    }

    async fn call<T: for<'a> Deserialize<'a>>(&self, method: &str, body: serde_json::Value) -> Result<T, String> {
        let r: Reply<T> = self
            .http
            .post(format!("{}/bot{}/{method}", self.api, self.token))
            .json(&body)
            .send()
            .await
            .map_err(clean)?
            .json()
            .await
            .map_err(clean)?;
        match (r.ok, r.result) {
            (true, Some(v)) => Ok(v),
            _ => Err(r.description.unwrap_or_else(|| "respuesta inesperada".into())),
        }
    }

    /// Texto plano (sin parse_mode): lo que escriban los usuarios no puede inyectar formato.
    pub async fn send(&self, chat: i64, text: &str) -> Result<(), String> {
        let text: String = text.chars().take(4000).collect();
        self.call::<serde_json::Value>("sendMessage", serde_json::json!({ "chat_id": chat, "text": text, "disable_web_page_preview": true }))
            .await
            .map(|_| ())
    }
}

pub fn code_hash(code: &str) -> String {
    sha256_hex(&format!("tg:{}", code.trim().to_lowercase()))
}

/// Atiende los mensajes que llegan al bot: `/start <código>` vincula, `/stop` desvincula.
pub fn spawn_poller(st: AppState) {
    let Some(tg) = st.telegram.clone() else { return };
    tokio::spawn(async move {
        let mut offset = 0i64;
        loop {
            let body = serde_json::json!({ "timeout": 25, "offset": offset, "allowed_updates": ["message"] });
            match tg.call::<Vec<Update>>("getUpdates", body).await {
                Ok(updates) => {
                    for u in updates {
                        offset = offset.max(u.update_id + 1);
                        if let Some(m) = u.message.filter(|m| m.chat.kind == "private") {
                            handle(&st, &tg, m.chat.id, m.text.unwrap_or_default()).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("telegram getUpdates: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });
}

async fn handle(st: &AppState, tg: &Telegram, chat: i64, text: String) {
    // tope por chat: frena la fuerza bruta sobre los códigos
    if st.user_limiter.hit(&format!("tg:{chat}"), 10, Duration::from_secs(900)).is_err() {
        return;
    }
    let mut it = text.split_whitespace();
    let reply = match (it.next().unwrap_or(""), it.next()) {
        ("/start", Some(code)) => link(st, chat, code).await,
        ("/stop", _) => {
            let _ = sqlx::query("UPDATE users SET telegram_chat_id = NULL WHERE telegram_chat_id = ?").bind(chat).execute(&st.db).await;
            "Listo: ya no vas a recibir avisos por acá.".to_string()
        }
        ("/tareas", _) => my_tasks(st, chat).await,
        ("/hecho", Some(n)) => done(st, chat, n).await,
        ("/hecho", None) => "Decime cuál: /hecho NÚMERO (el número sale de /tareas).".to_string(),
        _ => "Hola. Para vincular tu cuenta, generá un código en la app (Configuración → Telegram) y mandámelo así: /start CÓDIGO\n\nDespués podés usar:\n/tareas — tus tareas abiertas\n/hecho N — marcar la tarea N como hecha\n/stop — desvincular".to_string(),
    };
    if let Err(e) = tg.send(chat, &reply).await {
        tracing::warn!("telegram sendMessage: {e}");
    }
}

async fn link(st: &AppState, chat: i64, code: &str) -> String {
    let row = sqlx::query("SELECT id, username, tg_code_expires FROM users WHERE tg_code_hash = ?")
        .bind(code_hash(code))
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten();
    let Some(row) = row else { return "Código inválido o vencido. Generá uno nuevo en la app.".into() };
    let exp: Option<chrono::DateTime<Utc>> = row.get("tg_code_expires");
    if exp.is_none_or(|e| e < Utc::now()) {
        return "Código inválido o vencido. Generá uno nuevo en la app.".into();
    }
    let (id, name): (i64, String) = (row.get("id"), row.get("username"));
    // un chat pertenece a una sola cuenta
    let _ = sqlx::query("UPDATE users SET telegram_chat_id = NULL WHERE telegram_chat_id = ?").bind(chat).execute(&st.db).await;
    let _ = sqlx::query("UPDATE users SET telegram_chat_id = ?, tg_code_hash = NULL, tg_code_expires = NULL WHERE id = ?")
        .bind(chat)
        .bind(id)
        .execute(&st.db)
        .await;
    format!("✅ Vinculado a {name}. Vas a recibir por acá las novedades y los mensajes de tu equipo.")
}

const NOT_LINKED: &str = "Esta conversación no está vinculada a ninguna cuenta. Generá un código en la app y mandámelo con /start CÓDIGO.";

async fn linked_user(st: &AppState, chat: i64) -> Option<i64> {
    sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE telegram_chat_id = ?").bind(chat).fetch_optional(&st.db).await.ok().flatten()
}

async fn my_tasks(st: &AppState, chat: i64) -> String {
    let Some(user) = linked_user(st, chat).await else { return NOT_LINKED.into() };
    let rows = sqlx::query(
        "SELECT t.id, t.title, t.status, t.due, p.name AS project FROM tasks t JOIN projects p ON p.id = t.project_id \
         WHERE t.assignee_id = ? AND t.status != 'done' ORDER BY t.due IS NULL, t.due, t.id LIMIT 15",
    )
    .bind(user)
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return "No tenés tareas abiertas. 🎉".into();
    }
    let mut out = String::from("Tus tareas abiertas:\n");
    for r in rows {
        let due: Option<chrono::DateTime<Utc>> = r.get("due");
        let due = due.map(|d| format!(" · vence {}", d.format("%d/%m"))).unwrap_or_default();
        out.push_str(&format!("#{} {} ({}{due})\n", r.get::<i64, _>("id"), r.get::<String, _>("title"), r.get::<String, _>("project")));
    }
    out.push_str("\nPara cerrar una: /hecho NÚMERO");
    out
}

async fn done(st: &AppState, chat: i64, n: &str) -> String {
    let Some(user) = linked_user(st, chat).await else { return NOT_LINKED.into() };
    let Ok(id) = n.trim_start_matches('#').parse::<i64>() else { return "El número no es válido. Mirá /tareas.".into() };
    match crate::extras::set_done(st, user, id).await {
        Ok(t) => format!("✅ «{}» marcada como hecha.", t.title),
        Err(e) => format!("No pude: {e}"),
    }
}

//! Asistente del correo saliente: la cuenta SMTP se carga desde la app (CTO o administrador), se prueba antes de guardar,
//! la clave se guarda cifrada y el servidor la usa al instante, sin editar archivos ni reiniciar.
//! Lo que ya estaba por variables de entorno (`TM_SMTP_*`) sigue andando; lo guardado acá lo reemplaza.
use crate::{auth::AuthUser, error::{AppError, Result}, notify::Mailer, sysops::need, AppState};
use axum::{extract::State, routing::{get, post}, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/admin/mail", get(show).put(save).delete(clear)).route("/api/admin/mail/test", post(test))
}

async fn meta(st: &AppState, k: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM meta WHERE key = ?").bind(format!("smtp_{k}")).fetch_optional(&st.db).await.ok().flatten()
}

async fn put_meta(st: &AppState, k: &str, v: &str) -> Result<()> {
    sqlx::query("INSERT INTO meta(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value").bind(format!("smtp_{k}")).bind(v).execute(&st.db).await?;
    Ok(())
}

struct Cfg {
    host: String,
    port: u16,
    sec: String,
    user: String,
    pass: String,
    from: String,
}

async fn saved(st: &AppState) -> Option<Cfg> {
    let host = meta(st, "host").await?;
    Some(Cfg {
        host,
        port: meta(st, "port").await?.parse().ok()?,
        sec: meta(st, "sec").await.unwrap_or_else(|| "ssl".into()),
        user: meta(st, "user").await.unwrap_or_default(),
        pass: st.cred.open(&meta(st, "pass").await.unwrap_or_default()).unwrap_or_default(),
        from: meta(st, "from").await?,
    })
}

fn build(c: &Cfg) -> std::result::Result<Mailer, String> {
    Mailer::from_parts(&c.host, c.port, &c.sec, &c.user, &c.pass, &c.from)
}

/// Al arrancar: si hay cuenta guardada en la base, es la que vale.
pub async fn load(st: &AppState) {
    if let Some(c) = saved(st).await {
        match build(&c) {
            Ok(m) => {
                st.mailer.set(Some(Arc::new(m)));
                tracing::info!("emails habilitados (SMTP guardado en la app: {})", c.host);
            }
            Err(e) => tracing::error!("correo guardado inválido: {e}"),
        }
    }
}

/// Traduce los errores típicos de SMTP a algo que un CTO pueda accionar.
fn friendly(e: &str) -> String {
    let l = e.to_lowercase();
    let why = if l.contains("535") || l.contains("authentication") || l.contains("username and password") || l.contains("534") {
        "el servidor rechazó el usuario o la clave. En Gmail y Yahoo hace falta una «contraseña de aplicación» (no la clave normal); en Outlook, que SMTP autenticado esté habilitado para la cuenta"
    } else if l.contains("timed out") || l.contains("timeout") {
        "no hubo respuesta a tiempo: el puerto suele estar bloqueado por el firewall o el proveedor (probá 587 con STARTTLS, o 465 con SSL)"
    } else if l.contains("dns") || l.contains("resolve") || l.contains("lookup") || l.contains("name or service") {
        "no encuentro ese servidor: revisá cómo está escrito y que esta máquina tenga internet"
    } else if l.contains("refused") {
        "el servidor rechazó la conexión: revisá el puerto y que sea el SMTP de salida"
    } else if l.contains("tls") || l.contains("certificate") || l.contains("handshake") || l.contains("invalid") && l.contains("record") {
        "falló el cifrado: el puerto 465 va con «SSL/TLS» y el 587 con «STARTTLS»; probá cambiar el tipo de seguridad"
    } else if l.contains("sender") || l.contains("from address") || l.contains("not owned") || l.contains("553") || l.contains("550") {
        "el proveedor no deja enviar con ese remitente: usá la misma dirección de la cuenta (o un remitente verificado)"
    } else {
        return format!("No se pudo: {e}");
    };
    format!("No se pudo: {why}. (Detalle técnico: {e})")
}

async fn show(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let env = std::env::var("TM_SMTP_URL").is_ok();
    Ok(Json(match saved(&st).await {
        Some(c) => json!({ "active": st.mailer.is_some(), "source": "app", "host": c.host, "port": c.port, "sec": c.sec, "user": c.user, "has_pass": !c.pass.is_empty(), "from": c.from }),
        None => json!({ "active": st.mailer.is_some(), "source": if env { "env" } else { "none" } }),
    }))
}

#[derive(Deserialize)]
struct In {
    host: String,
    port: u16,
    sec: String,
    #[serde(default)]
    user: String,
    /// vacío = conservar la clave ya guardada
    #[serde(default)]
    pass: String,
    from: String,
    #[serde(default)]
    to: String,
}

async fn cfg_of(st: &AppState, b: &In) -> Result<Cfg> {
    let mut pass = b.pass.clone();
    if pass.is_empty() {
        if let Some(c) = saved(st).await.filter(|c| c.user == b.user) {
            pass = c.pass;
        }
    }
    if !b.user.is_empty() && pass.is_empty() {
        return Err(AppError::bad("falta la clave de la cuenta"));
    }
    Ok(Cfg { host: b.host.trim().to_string(), port: b.port, sec: b.sec.clone(), user: b.user.trim().to_string(), pass, from: b.from.trim().to_string() })
}

/// Conecta, valida el login y (si se pide) manda un correo de prueba. No guarda nada.
async fn test(State(st): State<AppState>, user: AuthUser, Json(b): Json<In>) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let c = cfg_of(&st, &b).await?;
    let m = build(&c).map_err(|e| AppError::bad(&e))?;
    if let Err(e) = m.check().await {
        return Ok(Json(json!({ "ok": false, "msg": friendly(&e) })));
    }
    if !b.to.trim().is_empty() {
        let body = "Este es un correo de prueba de BitacoraAI.\n\nSi lo estás leyendo, el envío de correo del servidor quedó bien configurado.".to_string();
        if let Err(e) = m.send(b.to.trim(), "[BitacoraAI] Prueba de correo", body).await {
            return Ok(Json(json!({ "ok": false, "msg": friendly(&e) })));
        }
        return Ok(Json(json!({ "ok": true, "msg": format!("Listo: el servidor aceptó la cuenta y mandó un correo de prueba a {}. Revisá la bandeja (y el spam).", b.to.trim()) })));
    }
    Ok(Json(json!({ "ok": true, "msg": "Listo: el servidor respondió y aceptó el usuario y la clave." })))
}

/// Prueba la conexión y, si anda, guarda (clave cifrada) y activa al instante.
async fn save(State(st): State<AppState>, user: AuthUser, Json(b): Json<In>) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let c = cfg_of(&st, &b).await?;
    let m = build(&c).map_err(|e| AppError::bad(&e))?;
    if let Err(e) = m.check().await {
        return Ok(Json(json!({ "ok": false, "msg": friendly(&e) })));
    }
    for (k, v) in [("host", c.host.as_str()), ("port", &c.port.to_string()), ("sec", &c.sec), ("user", &c.user), ("pass", &st.cred.seal(&c.pass)), ("from", &c.from)] {
        put_meta(&st, k, v).await?;
    }
    st.mailer.set(Some(Arc::new(m)));
    tracing::info!("correo configurado desde la app ({})", c.host);
    Ok(Json(json!({ "ok": true, "msg": "Guardado y activo: el servidor ya manda correos." })))
}

/// Borra lo guardado en la app (queda lo que haya en variables de entorno, si hay).
async fn clear(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    sqlx::query("DELETE FROM meta WHERE key LIKE 'smtp\\_%' ESCAPE '\\'").execute(&st.db).await?;
    st.mailer.set(Mailer::from_env().map(Arc::new));
    Ok(Json(json!({ "ok": true })))
}

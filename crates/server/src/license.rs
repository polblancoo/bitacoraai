//! Licencia comercial y registro anónimo de instalaciones.
//!
//! El software es AGPL-3.0-only y gratis. Quien lo use en una empresa sin querer cumplir la AGPL (publicar sus cambios)
//! puede comprar una licencia comercial: acá está el aviso (siempre descartable, nunca bloquea nada), el pago en USDT
//! (Tron o Ethereum) con QR, y la activación del código. Una vez por día el servidor avisa que existe (id al azar guardado
//! en la base, versión, país por IP del lado del servicio y totales: personas, IAs, proyectos y equipos; jamás nombres ni
//! contenido). Se apaga con `TM_TELEMETRY=0` o desde la ventana «Licencia».
use crate::{auth::AuthUser, error::{AppError, Result}, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::time::Duration;

const APP: &str = "rust-tui-manager";
// Dirección pública del servicio y su clave publicable (no da acceso a nada por sí sola). Se pueden cambiar con TM_LICENSE_URL / TM_LICENSE_KEY.
const DEFAULT_URL: &str = "https://gciwvjspjsveabbngzlu.supabase.co";
const DEFAULT_KEY: &str = "sb_publishable_PiraM8fuEPGBLlKKsYDo7Q_RKgdXv0P";
/// Uso gratuito sin aviso: hasta estas cantidades.
const FREE_HUMANS: i64 = 3;
const FREE_AIS: i64 = 2;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/license", get(status))
        .route("/api/license/settings", put(settings))
        .route("/api/license/plans", get(plans))
        .route("/api/license/upgrade", get(upgrade))
        .route("/api/license/order", post(order))
        .route("/api/license/order/{id}", get(order_state))
        .route("/api/license/activate", post(activate))
}

pub fn spawn_ping(st: AppState) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(45)).await;
        loop {
            ping(&st).await;
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
        }
    });
}

pub(crate) async fn meta_get(db: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM meta WHERE key = ?").bind(key).fetch_optional(db).await.ok().flatten()
}

pub(crate) async fn meta_set(db: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query("INSERT INTO meta (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key).bind(value).execute(db).await?;
    Ok(())
}

fn telemetry_env_off() -> bool {
    std::env::var("TM_TELEMETRY").is_ok_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
}

async fn telemetry_on(db: &SqlitePool) -> bool {
    !telemetry_env_off() && meta_get(db, "lic_telemetry").await.as_deref() != Some("0")
}

/// Id al azar (64 hex) guardado en la base: no sale de ningún dato de la máquina.
async fn install_id(db: &SqlitePool) -> Result<String> {
    if let Some(v) = meta_get(db, "lic_install_id").await.filter(|v| v.len() == 64) {
        return Ok(v);
    }
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|_| AppError(StatusCode::INTERNAL_SERVER_ERROR, "error interno".into()))?;
    let id: String = b.iter().map(|x| format!("{x:02x}")).collect();
    meta_set(db, "lic_install_id", &id).await?;
    Ok(id)
}

async fn counts(db: &SqlitePool) -> (i64, i64, i64, i64) {
    let one = |sql: &'static str| async move { sqlx::query_scalar::<_, i64>(sql).fetch_one(db).await.unwrap_or(0) };
    (
        one("SELECT COUNT(*) FROM users WHERE id NOT IN (SELECT user_id FROM api_tokens)").await,
        one("SELECT COUNT(*) FROM users WHERE id IN (SELECT user_id FROM api_tokens)").await,
        one("SELECT COUNT(*) FROM projects").await,
        one("SELECT COUNT(*) FROM teams").await,
    )
}

fn base_url() -> String {
    std::env::var("TM_LICENSE_URL").unwrap_or_else(|_| DEFAULT_URL.into()).trim_end_matches('/').to_string()
}

/// Llama a una función del servicio de licencias. Devuelve el JSON o un error entendible.
async fn call(function: &str, body: Value) -> Result<Value> {
    let key = std::env::var("TM_LICENSE_KEY").unwrap_or_else(|_| DEFAULT_KEY.into());
    let http = reqwest::Client::builder().timeout(Duration::from_secs(15)).build()
        .map_err(|_| AppError(StatusCode::INTERNAL_SERVER_ERROR, "error interno".into()))?;
    let r = http.post(format!("{}/functions/v1/{function}", base_url())).header("apikey", key).json(&body).send().await
        .map_err(|_| AppError(StatusCode::BAD_GATEWAY, "no pude conectar con el servicio de licencias (¿hay internet?)".into()))?;
    let ok = r.status().is_success();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    if ok {
        Ok(v)
    } else {
        Err(AppError::bad(v["error"].as_str().unwrap_or("el servicio de licencias rechazó el pedido")))
    }
}

/// Aviso diario: sin internet o sin respuesta no pasa nada.
async fn ping(st: &AppState) {
    if !telemetry_on(&st.db).await {
        return;
    }
    let Ok(id) = install_id(&st.db).await else { return };
    let (humans, ais, projects, teams) = counts(&st.db).await;
    let mut body = json!({
        "app": APP, "id_pc": id, "version": env!("CARGO_PKG_VERSION"), "windows": std::env::consts::OS,
        "usuarios": humans, "ias": ais, "proyectos": projects, "equipos": teams,
    });
    if let Some(t) = meta_get(&st.db, "lic_tipo").await {
        body["tipo"] = json!(t);
    }
    if let Ok(v) = call("registro", body).await {
        let info = v.get("licencia_info").cloned().unwrap_or(Value::Null);
        // Si el servicio no devuelve licencia no se pisa la que quedó guardada al activar (puede no estar registrada todavía).
        if !info.is_null() {
            let _ = meta_set(&st.db, "lic_info", &info.to_string()).await;
            if let Some(t) = info["token"].as_str() {
                let _ = meta_set(&st.db, "lic_token", t).await;
            }
        }
        let _ = meta_set(&st.db, "lic_msg", v["mensaje"].as_str().unwrap_or("")).await;
        let _ = meta_set(&st.db, "lic_min", v["version_minima"].as_str().unwrap_or("")).await;
    }
}

/// Funciones del módulo «pro» que esta instalación tiene habilitadas por su licencia firmada (vacío si no hay o no es válida).
pub(crate) async fn pro_features(db: &SqlitePool) -> Vec<String> {
    let (Some(token), Ok(id)) = (meta_get(db, "lic_token").await, install_id(db).await) else { return Vec::new() };
    crate::license_token::verify(&token, &id).map(|c| c.funciones).unwrap_or_default()
}

pub(crate) async fn instance_admin(db: &SqlitePool, user: i64) -> bool {
    sqlx::query_scalar::<_, Option<i64>>("SELECT MIN(id) FROM users WHERE id NOT IN (SELECT user_id FROM api_tokens)")
        .fetch_one(db).await.ok().flatten() == Some(user)
}

fn human(user: &AuthUser) -> Result<()> {
    if user.bot { Err(AppError::forbidden()) } else { Ok(()) }
}

async fn status(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    human(&user)?;
    let (humans, ais, projects, teams) = counts(&st.db).await;
    let tipo = meta_get(&st.db, "lic_tipo").await;
    let info: Value = meta_get(&st.db, "lic_info").await.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let valid = info["hasta"].as_str().and_then(|h| DateTime::parse_from_rfc3339(h).ok()).is_some_and(|h| h.with_timezone(&Utc) > Utc::now());
    let over = humans > FREE_HUMANS || ais > FREE_AIS;
    // Licencia vigente pero el uso supera lo que incluye el plan: hay que mejorar y pagar la diferencia.
    let (max_h, max_a) = (info["max_usuarios"].as_i64(), info["max_ias"].as_i64());
    let exceeds_plan = valid && (max_h.is_some_and(|m| humans > m) || max_a.is_some_and(|m| ais > m));
    Ok(Json(json!({
        "pro": { "module": crate::pro::info(), "enabled": pro_features(&st.db).await },
        "plan_limits": if valid { json!({ "humans": max_h, "ais": max_a }) } else { Value::Null }, "exceeds_plan": exceeds_plan,
        "humans": humans, "ais": ais, "projects": projects, "teams": teams,
        "free": { "humans": FREE_HUMANS, "ais": FREE_AIS },
        "tipo": tipo, "over": over, "licensed": valid, "license": if valid { info } else { Value::Null },
        "banner": exceeds_plan || (!valid && (over || tipo.as_deref() == Some("empresa"))),
        "telemetry": telemetry_on(&st.db).await, "telemetry_forced_off": telemetry_env_off(),
        "message": meta_get(&st.db, "lic_msg").await.unwrap_or_default(),
        "admin": instance_admin(&st.db, user.id).await, "version": env!("CARGO_PKG_VERSION"),
    })))
}

#[derive(Deserialize)]
struct SettingsBody {
    tipo: Option<String>,
    telemetry: Option<bool>,
}

async fn settings(State(st): State<AppState>, user: AuthUser, Json(b): Json<SettingsBody>) -> Result<Json<Value>> {
    human(&user)?;
    if !instance_admin(&st.db, user.id).await {
        return Err(AppError::forbidden());
    }
    if let Some(t) = b.tipo {
        if t != "personal" && t != "empresa" {
            return Err(AppError::bad("tipo inválido"));
        }
        meta_set(&st.db, "lic_tipo", &t).await?;
    }
    if let Some(on) = b.telemetry {
        meta_set(&st.db, "lic_telemetry", if on { "1" } else { "0" }).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct LangQ {
    lang: Option<String>,
}

async fn plans(State(st): State<AppState>, user: AuthUser, Query(q): Query<LangQ>) -> Result<Json<Value>> {
    human(&user)?;
    let id = install_id(&st.db).await?;
    Ok(Json(call("licencia", json!({ "accion": "planes", "app": APP, "id_pc": id, "idioma": q.lang.unwrap_or_default() })).await?))
}

/// ¿El uso actual supera el plan? A qué plan pasar y cuánto falta pagar.
async fn upgrade(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    human(&user)?;
    let id = install_id(&st.db).await?;
    let (humans, ais, _, _) = counts(&st.db).await;
    Ok(Json(call("licencia", json!({ "accion": "mejora_info", "app": APP, "id_pc": id, "usuarios": humans, "ias": ais })).await?))
}

#[derive(Deserialize)]
struct OrderBody {
    plan: String,
    email: String,
    cripto: String,
    red: String,
    /// Versión de los términos que la persona aceptó (la que mostró «planes»).
    accept: String,
    lang: Option<String>,
    /// Pagar la diferencia para pasar al plan que cubre el uso actual.
    #[serde(default)]
    upgrade: bool,
}

/// Contenido del QR: en Ethereum la transferencia del token ya armada (EIP-681); en Solana, un pago Solana Pay con el monto y el token; si no, la dirección.
fn qr_payload(red: &str, direccion: &str, contrato: &str, monto: &str) -> String {
    if red == "ethereum" {
        let units = monto.replace('.', "");
        let units = units.trim_start_matches('0');
        format!("ethereum:{contrato}@1/transfer?address={direccion}&uint256={}", if units.is_empty() { "0" } else { units })
    } else if red == "solana" {
        format!("solana:{direccion}?amount={monto}&spl-token={contrato}")
    } else {
        direccion.to_string()
    }
}

fn qr_svg(payload: &str) -> Option<String> {
    let code = qrcode::QrCode::new(payload.as_bytes()).ok()?;
    Some(code.render::<qrcode::render::svg::Color>().min_dimensions(220, 220).build())
}

async fn order(State(st): State<AppState>, user: AuthUser, Json(b): Json<OrderBody>) -> Result<Json<Value>> {
    human(&user)?;
    if !instance_admin(&st.db, user.id).await {
        return Err(AppError::forbidden());
    }
    let id = install_id(&st.db).await?;
    let (humans, ais, _, _) = counts(&st.db).await;
    let mut v = call("licencia", json!({
        "accion": "crear_pedido", "app": APP, "id_pc": id, "plan": b.plan, "email": b.email.trim(), "cripto": b.cripto, "red": b.red,
        "acepto": b.accept, "idioma": b.lang.unwrap_or_default(), "mejora": b.upgrade, "usuarios": humans, "ias": ais,
    })).await?;
    let payload = qr_payload(v["red"].as_str().unwrap_or(""), v["direccion"].as_str().unwrap_or(""), v["contrato"].as_str().unwrap_or(""), v["monto"].as_str().unwrap_or("0"));
    v["qr_svg"] = qr_svg(&payload).map(Value::from).unwrap_or(Value::Null);
    Ok(Json(v))
}

#[derive(Deserialize)]
struct TokenQ {
    token: String,
}

async fn order_state(State(st): State<AppState>, user: AuthUser, Path(id): Path<String>, Query(q): Query<TokenQ>) -> Result<Json<Value>> {
    human(&user)?;
    let mut v = call("licencia", json!({ "accion": "estado", "app": APP, "id": id, "token": q.token })).await?;
    if let Some(code) = v["codigo"].as_str().map(str::to_string) {
        // Pagado: la licencia se activa sola.
        match activate_code(&st, &code).await {
            Ok(info) => v["licencia"] = info,
            Err(e) => v["activacion_error"] = json!(e.1),
        }
    }
    Ok(Json(v))
}

async fn activate_code(st: &AppState, code: &str) -> Result<Value> {
    let id = install_id(&st.db).await?;
    let info = call("licencia", json!({ "accion": "activar", "app": APP, "id_pc": id, "codigo": code })).await?;
    meta_set(&st.db, "lic_info", &info.to_string()).await?;
    meta_set(&st.db, "lic_code", code).await?;
    if let Some(t) = info["token"].as_str() {
        let _ = meta_set(&st.db, "lic_token", t).await;
    }
    Ok(info)
}

#[derive(Deserialize)]
struct ActivateBody {
    codigo: String,
}

async fn activate(State(st): State<AppState>, user: AuthUser, Json(b): Json<ActivateBody>) -> Result<Json<Value>> {
    human(&user)?;
    if !instance_admin(&st.db, user.id).await {
        return Err(AppError::forbidden());
    }
    let code = b.codigo.trim();
    if code.len() < 8 || code.len() > 40 {
        return Err(AppError::bad("código inválido"));
    }
    Ok(Json(activate_code(&st, code).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_ethereum_usa_unidades_del_token() {
        let p = qr_payload("ethereum", "0xAbc", "0xdAC17F958D2ee523a2206206994597C13D831ec7", "120.003417");
        assert_eq!(p, "ethereum:0xdAC17F958D2ee523a2206206994597C13D831ec7@1/transfer?address=0xAbc&uint256=120003417");
        assert!(qr_payload("ethereum", "0xA", "0xC", "0.000417").ends_with("uint256=417"));
    }

    #[test]
    fn qr_solana_es_solana_pay() {
        assert_eq!(qr_payload("solana", "G5PV", "Es9v", "120.003417"), "solana:G5PV?amount=120.003417&spl-token=Es9v");
        assert!(qr_svg("solana:G5PV?amount=1&spl-token=Es9v").is_some_and(|s| s.contains("<svg")));
    }

    #[test]
    fn qr_otra_red_es_la_direccion() {
        assert_eq!(qr_payload("tron", "TXyz", "TR7", "120.003417"), "TXyz");
    }
}

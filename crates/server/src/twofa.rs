//! Verificación en dos pasos (TOTP, RFC 6238: Google Authenticator, Authy, 1Password, Aegis…).
//! Se activa por persona; al entrar piden el código de 6 dígitos después de la contraseña. Un código ya usado no sirve
//! de nuevo, hay 8 códigos de recuperación de un solo uso y, como la carpeta de red (WebDAV) no puede pedir códigos,
//! quien tiene 2FA entra ahí con una «contraseña de aplicación».
use crate::{auth::{random_hex, sha256_hex, verify_password, AuthUser}, error::{AppError, Result}, sysops, AppState};
use axum::{extract::{Path, State}, routing::{get, post}, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use sqlx::Row;
use tm_core::Id;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/me/2fa", get(status))
        .route("/api/me/2fa/setup", post(setup))
        .route("/api/me/2fa/enable", post(enable))
        .route("/api/me/2fa/disable", post(disable))
        .route("/api/me/dav-password", post(dav_password))
        .route("/api/admin/users/{id}/2fa/reset", post(admin_reset))
        .route("/api/admin/2fa-policy", get(policy_read).put(policy_save))
}

/// Qué grupos tienen que usar 2FA, según lo que decidió el administrador del servidor: (administradores, CTO, resto de las personas).
/// Es un pedido, no un bloqueo: la app avisa a esas personas y el administrador ve quién falta.
pub async fn policy(st: &AppState) -> (bool, bool, bool) {
    let on = |k: &'static str| async move { crate::license::meta_get(&st.db, k).await.is_some_and(|v| v == "1") };
    (on("req2fa_admin").await, on("req2fa_cto").await, on("req2fa_user").await)
}

fn applies(p: (bool, bool, bool), role: &str) -> bool {
    match role { "admin" => p.0, "cto" => p.1, _ => p.2 }
}

/// ¿El servidor le pide a esta persona que active el 2FA (por su rol) y todavía no lo hizo?
pub async fn asked(st: &AppState, user: Id) -> bool {
    let role: Option<String> = sqlx::query_scalar("SELECT server_role FROM users WHERE id = ? AND is_bot = 0").bind(user).fetch_optional(&st.db).await.ok().flatten();
    let p = policy(st).await;
    match role {
        Some(r) => applies(p, &r) && !enabled(st, user).await,
        None => false,
    }
}

async fn policy_read(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    sysops::need(&st, &user, false).await?;
    let (a, c, u) = policy(&st).await;
    let rows = sqlx::query("SELECT server_role, COUNT(*), SUM(totp_enabled) FROM users WHERE is_bot = 0 AND id NOT IN (SELECT user_id FROM api_tokens) GROUP BY server_role").fetch_all(&st.db).await?;
    let mut counts = serde_json::Map::new();
    for r in rows {
        counts.insert(r.get::<String, _>(0), json!({ "total": r.get::<i64, _>(1), "enrolled": r.get::<Option<i64>, _>(2).unwrap_or(0) }));
    }
    // quiénes faltan en los grupos a los que se les pide
    let pend = sqlx::query("SELECT username, server_role FROM users WHERE is_bot = 0 AND totp_enabled = 0 AND id NOT IN (SELECT user_id FROM api_tokens) ORDER BY username").fetch_all(&st.db).await?;
    let pending: Vec<Value> = pend.iter().filter(|r| applies((a, c, u), &r.get::<String, _>(1))).map(|r| json!(r.get::<String, _>(0))).collect();
    Ok(Json(json!({ "admin": a, "cto": c, "user": u, "counts": counts, "pending": pending, "can_edit": sysops::need(&st, &user, true).await.is_ok() })))
}

#[derive(Deserialize)]
struct Policy {
    admin: bool,
    cto: bool,
    user: bool,
}

/// Solo el administrador del servidor decide a quiénes se les pide (incluido él, por separado). A quienes pasan a estar incluidos y
/// todavía no lo activaron les llega un aviso (bandeja, Telegram y correo según lo que tengan conectado).
async fn policy_save(State(st): State<AppState>, user: AuthUser, Json(b): Json<Policy>) -> Result<Json<Value>> {
    sysops::need(&st, &user, true).await?;
    let before = policy(&st).await;
    for (k, v) in [("req2fa_admin", b.admin), ("req2fa_cto", b.cto), ("req2fa_user", b.user)] {
        crate::license::meta_set(&st.db, k, if v { "1" } else { "0" }).await?;
    }
    let now = (b.admin, b.cto, b.user);
    if before != now {
        audit(&st, user.id, user.id, "politica_2fa").await;
        tracing::info!("2fa: política cambiada por #{}: administradores={} cto={} resto={}", user.id, b.admin, b.cto, b.user);
        let rows = sqlx::query("SELECT id, server_role FROM users WHERE is_bot = 0 AND totp_enabled = 0 AND id NOT IN (SELECT user_id FROM api_tokens)").fetch_all(&st.db).await?;
        for r in rows {
            let (id, role): (Id, String) = (r.get(0), r.get(1));
            if applies(now, &role) && !applies(before, &role) {
                crate::notify::notify(&st, id, "security", None, None, "🔐 El administrador pide activar la verificación en dos pasos: Mi perfil → Verificación en dos pasos.".into()).await;
            }
        }
    }
    policy_read(State(st), user).await
}

const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn b32_encode(data: &[u8]) -> String {
    let (mut out, mut buf, mut bits) = (String::new(), 0u32, 0);
    for &b in data {
        buf = (buf << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            out.push(B32[((buf >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        out.push(B32[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn b32_decode(s: &str) -> Option<Vec<u8>> {
    let (mut out, mut buf, mut bits) = (Vec::new(), 0u32, 0);
    for c in s.bytes().filter(|c| *c != b'=' && *c != b' ') {
        let v = B32.iter().position(|x| *x == c.to_ascii_uppercase())? as u32;
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            out.push((buf >> (bits - 8)) as u8);
            bits -= 8;
        }
    }
    Some(out)
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..20].copy_from_slice(&Sha1::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let (mut ipad, mut opad) = ([0x36u8; 64], [0x5cu8; 64]);
    for i in 0..64 {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let inner = Sha1::new().chain_update(ipad).chain_update(msg).finalize();
    Sha1::new().chain_update(opad).chain_update(inner).finalize().into()
}

/// Código de 6 dígitos del paso `counter` (RFC 4226 con truncado dinámico).
fn code_at(secret: &[u8], counter: u64) -> u32 {
    let h = hmac_sha1(secret, &counter.to_be_bytes());
    let o = (h[19] & 0xf) as usize;
    (u32::from_be_bytes([h[o] & 0x7f, h[o + 1], h[o + 2], h[o + 3]])) % 1_000_000
}

/// ¿Sirve este código? Acepta el paso actual y uno antes/después (relojes algo corridos) y nunca uno ya usado (`last`).
/// Devuelve el paso aceptado.
fn check(secret: &[u8], code: &str, now: u64, last: i64) -> Option<i64> {
    let code = code.trim().replace(' ', "");
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let step = (now / 30) as i64;
    // se compara siempre contra los tres para no filtrar cuál fue por el tiempo de respuesta
    let mut hit = None;
    for s in [step - 1, step, step + 1] {
        let ok = format!("{:06}", code_at(secret, s as u64)) == code && s > last;
        if ok && hit.is_none_or(|h| s > h) {
            hit = Some(s);
        }
    }
    hit
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub const NEEDS_CODE: &str = "Hace falta el código de verificación en dos pasos";

/// ¿La persona tiene 2FA activo? (lo usa el login y la carpeta de red)
pub async fn enabled(st: &AppState, user: Id) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT totp_enabled FROM users WHERE id = ?").bind(user).fetch_one(&st.db).await.unwrap_or(0) != 0
}

/// Comprueba el segundo paso de quien ya puso bien la contraseña: código de la app o de recuperación (que se gasta).
pub async fn verify_second(st: &AppState, user: Id, code: &str) -> bool {
    let Ok(r) = sqlx::query("SELECT totp_secret, totp_last FROM users WHERE id = ? AND totp_enabled = 1").bind(user).fetch_one(&st.db).await else { return false };
    let secret = r.get::<Option<String>, _>("totp_secret").and_then(|s| st.cred.open(&s)).and_then(|s| b32_decode(&s));
    let compact: String = code.trim().chars().filter(|c| *c != '-' && *c != ' ').collect::<String>().to_lowercase();
    if compact.chars().all(|c| c.is_ascii_digit()) && compact.len() == 6 {
        let Some(secret) = secret else { return false };
        let Some(step) = check(&secret, &compact, now(), r.get("totp_last")) else { return false };
        // solo gana quien avanza el contador: dos entradas simultáneas con el mismo código no pasan las dos
        return sqlx::query("UPDATE users SET totp_last = ? WHERE id = ? AND totp_last < ?").bind(step).bind(user).bind(step).execute(&st.db).await.is_ok_and(|x| x.rows_affected() == 1);
    }
    if compact.len() == 12 && compact.chars().all(|c| c.is_ascii_hexdigit()) {
        return sqlx::query("UPDATE recovery_codes SET used_at = ? WHERE user_id = ? AND hash = ? AND used_at IS NULL")
            .bind(chrono::Utc::now()).bind(user).bind(sha256_hex(&compact)).execute(&st.db).await.is_ok_and(|x| x.rows_affected() == 1);
    }
    false
}

/// ¿Esta contraseña es la de aplicación de la persona? (carpeta de red)
pub async fn dav_token_ok(st: &AppState, user: Id, pass: &str) -> bool {
    let h: Option<String> = sqlx::query_scalar("SELECT dav_token_hash FROM users WHERE id = ?").bind(user).fetch_one(&st.db).await.ok().flatten();
    h.is_some_and(|h| h == sha256_hex(pass))
}

async fn status(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recovery_codes WHERE user_id = ? AND used_at IS NULL").bind(user.id).fetch_one(&st.db).await?;
    let has_dav: Option<String> = sqlx::query_scalar("SELECT dav_token_hash FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(json!({ "enabled": enabled(&st, user.id).await, "recovery_left": left, "dav_password": has_dav.is_some(), "asked": asked(&st, user.id).await })))
}

/// Genera un secreto nuevo (todavía sin activar) y lo muestra como QR para escanear con la app.
async fn setup(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    if enabled(&st, user.id).await {
        return Err(AppError::bad("Ya tenés la verificación en dos pasos activa: desactivala primero para cambiar de aplicación"));
    }
    let mut raw = [0u8; 20];
    getrandom::fill(&mut raw).map_err(|_| AppError::bad("no se pudo generar el secreto"))?;
    let secret = b32_encode(&raw);
    sqlx::query("UPDATE users SET totp_secret = ?, totp_enabled = 0, totp_last = 0 WHERE id = ?").bind(st.cred.seal(&secret)).bind(user.id).execute(&st.db).await?;
    let name = sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    let uri = format!("otpauth://totp/CrewHelm:{}?secret={secret}&issuer=CrewHelm&algorithm=SHA1&digits=6&period=30", name.replace(['/', ':', '?', '&', '#', '%', ' '], "_"));
    let qr = qrcode::QrCode::new(uri.as_bytes()).ok().map(|c| c.render::<qrcode::render::svg::Color>().min_dimensions(200, 200).build());
    Ok(Json(json!({ "secret": secret, "uri": uri, "qr_svg": qr })))
}

#[derive(Deserialize)]
struct Code {
    code: String,
    #[serde(default)]
    password: String,
}

fn new_recovery_codes() -> Vec<String> {
    (0..8).map(|_| random_hex(6)).collect()
}

/// Confirma con un código de la app (prueba de que quedó bien escaneado) y activa. Muestra los códigos de recuperación UNA vez.
async fn enable(State(st): State<AppState>, user: AuthUser, Json(b): Json<Code>) -> Result<Json<Value>> {
    user.human_only()?;
    let key = format!("2fa:{}", user.id);
    st.user_limiter.hit(&key, 8, std::time::Duration::from_secs(15 * 60))?;
    let r = sqlx::query("SELECT totp_secret, totp_enabled, totp_last FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    if r.get::<i64, _>("totp_enabled") != 0 {
        return Err(AppError::bad("ya está activa"));
    }
    let secret = r.get::<Option<String>, _>("totp_secret").and_then(|s| st.cred.open(&s)).and_then(|s| b32_decode(&s)).ok_or_else(|| AppError::bad("primero tocá «Activar» para generar el código QR"))?;
    let Some(step) = check(&secret, &b.code, now(), 0) else { return Err(AppError::bad("Ese código no coincide: revisá la hora del teléfono y probá con el siguiente")) };
    sqlx::query("UPDATE users SET totp_enabled = 1, totp_last = ? WHERE id = ?").bind(step).bind(user.id).execute(&st.db).await?;
    sqlx::query("DELETE FROM recovery_codes WHERE user_id = ?").bind(user.id).execute(&st.db).await?;
    let codes = new_recovery_codes();
    for c in &codes {
        sqlx::query("INSERT INTO recovery_codes (user_id, hash) VALUES (?, ?)").bind(user.id).bind(sha256_hex(c)).execute(&st.db).await?;
    }
    st.user_limiter.reset(&key);
    audit(&st, user.id, user.id, "2fa_activada").await;
    Ok(Json(json!({ "recovery": codes.iter().map(|c| format!("{}-{}-{}", &c[0..4], &c[4..8], &c[8..12])).collect::<Vec<_>>() })))
}

/// Desactivar pide la contraseña y un código vigente: con la sesión abierta sola no alcanza.
async fn disable(State(st): State<AppState>, user: AuthUser, Json(b): Json<Code>) -> Result<Json<Value>> {
    user.human_only()?;
    let key = format!("2fa:{}", user.id);
    st.user_limiter.hit(&key, 8, std::time::Duration::from_secs(15 * 60))?;
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    if !verify_password(&b.password, &hash) || !verify_second(&st, user.id, &b.code).await {
        return Err(AppError::bad("Contraseña o código incorrectos"));
    }
    clear(&st, user.id).await?;
    st.user_limiter.reset(&key);
    audit(&st, user.id, user.id, "2fa_desactivada").await;
    Ok(Json(json!({ "enabled": false })))
}

async fn clear(st: &AppState, user: Id) -> Result<()> {
    sqlx::query("UPDATE users SET totp_secret = NULL, totp_enabled = 0, totp_last = 0 WHERE id = ?").bind(user).execute(&st.db).await?;
    sqlx::query("DELETE FROM recovery_codes WHERE user_id = ?").bind(user).execute(&st.db).await?;
    Ok(())
}

/// Contraseña de aplicación para la carpeta de red (se muestra una sola vez; generar otra anula la anterior).
async fn dav_password(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let t = format!("crew-{}", random_hex(12));
    sqlx::query("UPDATE users SET dav_token_hash = ? WHERE id = ?").bind(sha256_hex(&t)).bind(user.id).execute(&st.db).await?;
    audit(&st, user.id, user.id, "contrasena_de_aplicacion").await;
    Ok(Json(json!({ "password": t })))
}

/// Si alguien pierde el teléfono y los códigos de recuperación, el administrador del servidor le quita el 2FA.
async fn admin_reset(State(st): State<AppState>, user: AuthUser, Path(target): Path<Id>) -> Result<Json<Value>> {
    sysops::need(&st, &user, true).await?;
    clear(&st, target).await?;
    // además se cierran sus sesiones abiertas
    sqlx::query("UPDATE users SET token_version = token_version + 1 WHERE id = ?").bind(target).execute(&st.db).await?;
    audit(&st, user.id, target, "2fa_restablecida_por_admin").await;
    Ok(Json(json!({ "ok": true })))
}

/// Queda en la auditoría de cada equipo de la persona afectada.
async fn audit(st: &AppState, actor: Id, target: Id, kind: &str) {
    tracing::info!("2fa: {kind} (hecho por #{actor}, cuenta #{target})");
    let teams: Vec<Id> = sqlx::query("SELECT team_id FROM team_members WHERE user_id = ?").bind(target).fetch_all(&st.db).await.unwrap_or_default().iter().map(|r| r.get(0)).collect();
    for t in teams {
        crate::audit::record(st, t, None, None, actor, kind, &format!("cuenta #{target}")).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectores_rfc_4226_y_6238() {
        // RFC 4226 apéndice D: secreto «12345678901234567890»
        let s = b"12345678901234567890";
        for (i, esperado) in [755224, 287082, 359152, 969429, 338314, 254676, 287922, 162583, 399871, 520489].iter().enumerate() {
            assert_eq!(code_at(s, i as u64), *esperado);
        }
        // RFC 6238: t=59 → 94287082 (8 dígitos) → 6 dígitos 287082
        assert_eq!(code_at(s, 59 / 30), 287082);
    }

    #[test]
    fn base32_ida_y_vuelta() {
        assert_eq!(b32_encode(b"12345678901234567890"), "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_eq!(b32_decode("gezdgnbvgy3tqojqgezdgnbvgy3tqojq").unwrap(), b"12345678901234567890");
        assert_eq!(b32_decode("11"), None);
    }

    #[test]
    fn ventana_y_reuso() {
        let s = b"12345678901234567890";
        let ahora = 1_700_000_000u64;
        let c = format!("{:06}", code_at(s, ahora / 30));
        assert_eq!(check(s, &c, ahora, 0), Some((ahora / 30) as i64));
        assert!(check(s, &c, ahora + 30, 0).is_some(), "un paso después todavía vale");
        assert!(check(s, &c, ahora + 90, 0).is_none(), "tres pasos después ya no");
        assert!(check(s, &c, ahora, (ahora / 30) as i64).is_none(), "el mismo código no sirve dos veces");
        assert!(check(s, "12345", ahora, 0).is_none());
        assert!(check(s, "abcdef", ahora, 0).is_none());
    }
}

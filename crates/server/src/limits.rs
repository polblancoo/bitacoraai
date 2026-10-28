//! Límites de entrada y antiabuso: tamaños de campos, topes por recurso y rate limit.
use crate::error::{AppError, Result};
use axum::http::HeaderMap;
use std::{
    collections::HashMap,
    hash::Hash,
    net::{IpAddr, SocketAddr},
    sync::Mutex,
    time::{Duration, Instant},
};

pub const MAX_BODY_BYTES: usize = 64 * 1024;

pub const USERNAME_MIN: usize = 3;
pub const USERNAME_MAX: usize = 32;
pub const PASSWORD_MIN: usize = 8;
// argon2 es costoso: sin tope, una contraseña enorme es un vector de DoS.
pub const PASSWORD_MAX: usize = 128;
pub const NAME_MAX: usize = 100;
pub const TITLE_MAX: usize = 200;
pub const DESCRIPTION_MAX: usize = 10_000;
pub const COMMENT_MAX: usize = 4_000;
pub const COLOR_MAX: usize = 9;

pub const MAX_TEAMS_PER_USER: i64 = 50;
pub const MAX_MEMBERS_PER_TEAM: i64 = 100;
pub const MAX_PROJECTS_PER_TEAM: i64 = 200;
pub const MAX_TASKS_PER_PROJECT: i64 = 5_000;
pub const MAX_COMMENTS_PER_TASK: i64 = 2_000;
pub const MAX_SUBTASKS_PER_TASK: i64 = 100;
pub const MAX_LABELS_PER_TEAM: i64 = 100;
pub const MAX_LABELS_PER_TASK: usize = 10;
pub const LABEL_MAX: usize = 30;
pub const MAX_INVITES_PER_TEAM: i64 = 20;

pub const MAX_WS_PER_USER: usize = 8;

/// Largo en caracteres, no en bytes.
pub fn max_len(field: &str, value: &str, max: usize) -> Result<()> {
    if value.chars().count() > max {
        return Err(AppError::bad(&format!("{field}: máximo {max} caracteres")));
    }
    Ok(())
}

pub fn username(value: &str) -> Result<()> {
    let n = value.chars().count();
    if !(USERNAME_MIN..=USERNAME_MAX).contains(&n)
        || !value.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(AppError::bad("usuario: 3-32 caracteres (letras, números, _ - .)"));
    }
    Ok(())
}

pub fn color(value: &str) -> Result<()> {
    let ok = value.len() <= COLOR_MAX
        && value.starts_with('#')
        && value.len() > 1
        && value[1..].chars().all(|c| c.is_ascii_hexdigit());
    if ok { Ok(()) } else { Err(AppError::bad("color inválido (#rrggbb)")) }
}

pub fn too_many(what: &str, max: i64) -> AppError {
    AppError(axum::http::StatusCode::UNPROCESSABLE_ENTITY, format!("límite alcanzado: máximo {max} {what}"))
}

/// Ventana fija por clave (IP o usuario); protege de fuerza bruta y del costo de argon2.
pub struct RateLimiter<K> {
    hits: Mutex<HashMap<K, (u32, Instant)>>,
}

impl<K> Default for RateLimiter<K> {
    fn default() -> Self { Self { hits: Mutex::new(HashMap::new()) } }
}

impl<K: Hash + Eq + Clone> RateLimiter<K> {
    /// Cuenta un intento; error 429 si se supera `max` dentro de `window`.
    pub fn hit(&self, key: &K, max: u32, window: Duration) -> Result<()> {
        let mut map = self.hits.lock().unwrap();
        let now = Instant::now();
        if map.len() > 10_000 {
            map.retain(|_, (_, t)| now.duration_since(*t) < window);
        }
        let e = map.entry(key.clone()).or_insert((0, now));
        if now.duration_since(e.1) >= window {
            *e = (0, now);
        }
        e.0 += 1;
        if e.0 > max { Err(too_fast()) } else { Ok(()) }
    }

    /// Solo consulta (no cuenta): ¿la clave ya está bloqueada?
    pub fn blocked(&self, key: &K, max: u32, window: Duration) -> bool {
        let map = self.hits.lock().unwrap();
        map.get(key).is_some_and(|(n, t)| *n >= max && t.elapsed() < window)
    }

    pub fn reset(&self, key: &K) {
        self.hits.lock().unwrap().remove(key);
    }
}

pub fn too_fast() -> AppError {
    AppError(axum::http::StatusCode::TOO_MANY_REQUESTS, "demasiados intentos, esperá unos minutos".into())
}

/// IP del cliente. Solo se confía en `X-Forwarded-For` si `TM_TRUSTED_PROXY=1`
/// (server detrás de un proxy inverso propio); se toma la última entrada, que es
/// la que agregó el proxy y no puede falsificar el cliente.
pub fn client_ip(headers: &HeaderMap, peer: SocketAddr, trust_proxy: bool) -> IpAddr {
    if trust_proxy {
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse().ok())
        {
            return ip;
        }
    }
    peer.ip()
}

/// Conexiones WebSocket activas por usuario.
#[derive(Default)]
pub struct WsCounter(Mutex<HashMap<i64, usize>>);

impl WsCounter {
    pub fn acquire(&self, user: i64) -> bool {
        let mut m = self.0.lock().unwrap();
        let n = m.entry(user).or_insert(0);
        if *n >= MAX_WS_PER_USER { return false; }
        *n += 1;
        true
    }

    pub fn release(&self, user: i64) {
        let mut m = self.0.lock().unwrap();
        if let Some(n) = m.get_mut(&user) {
            *n = n.saturating_sub(1);
            if *n == 0 { m.remove(&user); }
        }
    }
}

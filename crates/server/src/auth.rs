use crate::{error::AppError, AppState};
use argon2::{password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier}, Argon2};
use axum::{extract::FromRequestParts, http::request::Parts};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tm_core::Id;

const TOKEN_DAYS: i64 = 7;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: Id,
    name: String,
    /// Versión de sesión: "cerrar todas las sesiones" la incrementa e invalida los tokens viejos.
    ver: i64,
    exp: usize,
}

pub struct AuthUser {
    pub id: Id,
}

pub fn hash_password(pw: &str) -> Result<String, AppError> {
    Argon2::default()
        .hash_password(pw.as_bytes())
        .map(|h| h.to_string())
        .map_err(|_| AppError::bad("hash"))
}

pub fn verify_password(pw: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(pw.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Hash válido que no corresponde a ninguna cuenta; se verifica cuando el usuario no existe
/// para que el tiempo de respuesta no revele qué usuarios están registrados.
pub fn dummy_verify(pw: &str) {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let hash = DUMMY.get_or_init(|| hash_password("dummy-password-for-timing").unwrap_or_default());
    let _ = verify_password(pw, hash);
}

pub fn make_token(secret: &str, id: Id, name: &str, ver: i64) -> String {
    let exp = (chrono::Utc::now() + chrono::Duration::days(TOKEN_DAYS)).timestamp() as usize;
    let claims = Claims { sub: id, name: name.into(), ver, exp };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())).expect("jwt")
}

fn decode_token(secret: &str, token: &str) -> Option<Claims> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default()).ok().map(|d| d.claims)
}

/// Valida firma, vencimiento y que el usuario exista con la misma versión de sesión.
pub async fn authenticate(st: &AppState, token: &str) -> Option<AuthUser> {
    let c = decode_token(&st.secret, token)?;
    let ver: Option<i64> = sqlx::query_scalar("SELECT token_version FROM users WHERE id = ?")
        .bind(c.sub)
        .fetch_optional(&st.db)
        .await
        .ok()?;
    (ver? == c.ver).then_some(AuthUser { id: c.sub })
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::unauthorized("falta token"))?;
        authenticate(state, token).await.ok_or_else(|| AppError::unauthorized("token inválido"))
    }
}

/// Cadena hexadecimal aleatoria de `bytes` bytes (CSPRNG del sistema).
pub fn random_hex(bytes: usize) -> String {
    use std::io::Read;
    let mut buf = vec![0u8; bytes];
    std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)).expect("urandom");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(input.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

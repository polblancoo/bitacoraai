//! Licencia firmada: el servicio de licencias firma con Ed25519 un «token» con el plan, los límites y las funciones
//! habilitadas; el servidor lo verifica sin internet con la clave pública que lleva dentro. Sin token válido, o con la
//! clave pública vacía, las funciones del módulo «pro» no se habilitan y el resto de la app funciona igual.
//!
//! Formato: `<payload en hex>.<firma en hex>`; la firma es sobre los bytes del payload (un JSON).

use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;

/// Clave pública de verificación (32 bytes en hex). La privada vive solo como secreto del servicio de licencias.
/// Vacía = todavía no hay clave de producción: no se acepta ningún token.
pub const PUBLIC_KEY_HEX: &str = "";

/// Días de gracia después del vencimiento.
const GRACE_DAYS: i64 = 30;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Claims {
    pub app: String,
    pub plan: String,
    #[serde(default)]
    pub codigo: String,
    pub id_pc: String,
    pub hasta: DateTime<Utc>,
    #[serde(default)]
    pub funciones: Vec<String>,
    #[serde(default)]
    pub max_usuarios: Option<i64>,
    #[serde(default)]
    pub max_ias: Option<i64>,
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// Verifica firma, instalación (`id_pc`) y vigencia (con gracia). Devuelve los datos de la licencia.
pub fn verify_with(key_hex: &str, token: &str, id_pc: &str, now: DateTime<Utc>) -> Result<Claims, &'static str> {
    let key: [u8; 32] = unhex(key_hex).and_then(|b| b.try_into().ok()).ok_or("sin clave pública")?;
    let key = VerifyingKey::from_bytes(&key).map_err(|_| "clave pública inválida")?;
    let (payload, sig) = token.split_once('.').ok_or("formato inválido")?;
    let (payload, sig) = (unhex(payload).ok_or("formato inválido")?, unhex(sig).ok_or("formato inválido")?);
    let sig = Signature::from_slice(&sig).map_err(|_| "firma inválida")?;
    key.verify(&payload, &sig).map_err(|_| "firma inválida")?;
    let c: Claims = serde_json::from_slice(&payload).map_err(|_| "contenido inválido")?;
    if c.id_pc != id_pc {
        return Err("es de otra instalación");
    }
    if now > c.hasta + Duration::days(GRACE_DAYS) {
        return Err("vencido");
    }
    Ok(c)
}

pub fn verify(token: &str, id_pc: &str) -> Result<Claims, &'static str> {
    verify_with(PUBLIC_KEY_HEX, token, id_pc, Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
    fn key() -> (SigningKey, String) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = hex(sk.verifying_key().as_bytes());
        (sk, pk)
    }
    fn token(sk: &SigningKey, json: &str) -> String {
        format!("{}.{}", hex(json.as_bytes()), hex(&sk.sign(json.as_bytes()).to_bytes()))
    }
    const PC: &str = "aaaa";
    fn body(hasta: &str) -> String {
        format!(r#"{{"app":"rust-tui-manager","plan":"equipo","codigo":"TM-X","id_pc":"{PC}","hasta":"{hasta}","funciones":["revision","auditoria"],"max_usuarios":10,"max_ias":10}}"#)
    }
    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn token_valido() {
        let (sk, pk) = key();
        let c = verify_with(&pk, &token(&sk, &body("2027-01-01T00:00:00Z")), PC, at("2026-11-01T00:00:00Z")).unwrap();
        assert_eq!(c.funciones, ["revision", "auditoria"]);
        assert_eq!(c.max_usuarios, Some(10));
    }

    #[test]
    fn firma_adulterada_o_de_otra_clave() {
        let (sk, pk) = key();
        let t = token(&sk, &body("2027-01-01T00:00:00Z"));
        let cambiado = t.replacen("6571", "6572", 1);
        assert!(verify_with(&pk, &cambiado, PC, at("2026-11-01T00:00:00Z")).is_err());
        let otra = SigningKey::from_bytes(&[9u8; 32]);
        assert_eq!(verify_with(&pk, &token(&otra, &body("2027-01-01T00:00:00Z")), PC, at("2026-11-01T00:00:00Z")), Err("firma inválida"));
    }

    #[test]
    fn otra_instalacion_no_sirve() {
        let (sk, pk) = key();
        assert_eq!(verify_with(&pk, &token(&sk, &body("2027-01-01T00:00:00Z")), "bbbb", at("2026-11-01T00:00:00Z")), Err("es de otra instalación"));
    }

    #[test]
    fn vencimiento_con_gracia() {
        let (sk, pk) = key();
        let t = token(&sk, &body("2027-01-01T00:00:00Z"));
        assert!(verify_with(&pk, &t, PC, at("2027-01-20T00:00:00Z")).is_ok());
        assert_eq!(verify_with(&pk, &t, PC, at("2027-02-05T00:00:00Z")), Err("vencido"));
    }

    #[test]
    fn sin_clave_de_produccion_no_acepta_nada() {
        assert!(verify("00.00", PC).is_err());
        assert_eq!(PUBLIC_KEY_HEX, "");
    }

    /// Token generado con Node (misma firma Ed25519 que hace la Edge Function): el formato coincide de punta a punta.
    #[test]
    fn token_firmado_por_node() {
        let pk = "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c";
        let t = "7b22617070223a22727573742d7475692d6d616e61676572222c22706c616e223a2265717569706f222c22636f6469676f223a22544d2d58222c2269645f7063223a2261616161222c226861737461223a22323032372d30312d30315430303a30303a30305a222c2266756e63696f6e6573223a5b227265766973696f6e222c2261756469746f726961225d2c226d61785f7573756172696f73223a31302c226d61785f696173223a31307d.bf1ba2be516e3721b003e7f0491a5e33dd4b79b36770eb6b869f39603422d5e8a811f5f45f002c8b5d304ca2b1a6a8306f98d325936ec4b4a2ae04d989daef05";
        let c = verify_with(pk, t, "aaaa", at("2026-11-01T00:00:00Z")).unwrap();
        assert_eq!(c.plan, "equipo");
        assert_eq!(c.funciones, ["revision", "auditoria"]);
    }
}

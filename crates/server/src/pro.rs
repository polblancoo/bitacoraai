//! Punto de enganche del módulo cerrado «pro» (funciones de empresa, activadas con licencia firmada).
//!
//! El repositorio público compila siempre sin `pro`: acá solo está el contrato y una implementación vacía. Los binarios
//! oficiales se construyen con `--features pro`, que enlaza el crate privado `crewhelm_pro` (lo agrega la CI privada al
//! `Cargo.toml` antes de compilar) y le da el control de estas funciones. Sin licencia, o sin el módulo, no pasa nada:
//! las funciones pro simplemente no están, y el resto de la app funciona igual.

use crate::AppState;
use axum::Router;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Lo que el módulo cerrado le ofrece al servidor.
pub trait Pro: Send + Sync {
    /// Funciones que el módulo trae (para mostrarlas), estén o no habilitadas por la licencia.
    fn features(&self) -> Vec<&'static str>;
    /// Rutas propias (`/api/pro/...`). Cada una revisa que su función esté habilitada.
    fn router(&self) -> Router<AppState>;
}

/// Implementación del repositorio público: no trae nada.
struct NoPro;

impl Pro for NoPro {
    fn features(&self) -> Vec<&'static str> {
        Vec::new()
    }
    fn router(&self) -> Router<AppState> {
        Router::new()
    }
}

#[cfg(feature = "pro")]
fn build() -> Box<dyn Pro> {
    Box::new(crewhelm_pro::load())
}

#[cfg(not(feature = "pro"))]
fn build() -> Box<dyn Pro> {
    Box::new(NoPro)
}

fn get() -> &'static dyn Pro {
    static PRO: OnceLock<Box<dyn Pro>> = OnceLock::new();
    PRO.get_or_init(build).as_ref()
}

pub fn router() -> Router<AppState> {
    get().router()
}

/// Para el estado de la licencia: si el binario trae el módulo y qué funciones incluye.
pub fn info() -> Value {
    let f = get().features();
    json!({ "module": !f.is_empty(), "features": f })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_modulo_no_hay_funciones() {
        let i = info();
        assert_eq!(i["module"], false);
        assert_eq!(i["features"].as_array().map(Vec::len), Some(0));
    }
}

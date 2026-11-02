//! HTTPS automático: si el administrador no configura un certificado propio, el servidor crea uno
//! autofirmado la primera vez y lo sirve en un segundo puerto (el de HTTP + 1) junto al HTTP de siempre.
//! Los pedidos que entran por ese puerto llevan la marca interna `x-tm-https`, que el chat usa para
//! aceptar la conexión como segura; en el puerto HTTP la marca se borra, así nadie puede falsificarla.
use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    path::PathBuf,
    sync::OnceLock,
};

pub const HDR: &str = "x-tm-https";
static PORT: OnceLock<u16> = OnceLock::new();

/// Puerto HTTPS automático, si está activo (para armar el enlace que se le muestra a la gente).
pub fn port() -> Option<u16> {
    PORT.get().copied()
}

pub async fn mark(mut req: Request, next: Next) -> Response {
    req.headers_mut().insert(HDR, HeaderValue::from_static("1"));
    next.run(req).await
}

pub async fn strip(mut req: Request, next: Next) -> Response {
    req.headers_mut().remove(HDR);
    next.run(req).await
}

/// Dirección del HTTPS automático: TM_HTTPS=0 lo apaga, TM_HTTPS_PORT cambia el puerto (por defecto, el de HTTP + 1).
pub fn plan(http: SocketAddr) -> Option<SocketAddr> {
    if std::env::var("TM_HTTPS").is_ok_and(|v| v == "0") {
        return None;
    }
    let port = std::env::var("TM_HTTPS_PORT").ok().and_then(|p| p.parse().ok()).or_else(|| http.port().checked_add(1))?;
    let _ = PORT.set(port);
    Some(SocketAddr::new(http.ip(), port))
}

fn dir() -> PathBuf {
    if let Ok(d) = std::env::var("TM_TLS_DIR") {
        return d.into();
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config/tm-server/tls")
}

/// Nombres e IPs con los que la gente va a entrar: esta máquina, su IP de red y lo que se agregue en TM_TLS_NAMES.
fn names() -> Vec<String> {
    let mut v: Vec<String> = vec!["localhost".into(), "127.0.0.1".into()];
    if let Ok(h) = std::fs::read_to_string("/etc/hostname") {
        let h = h.trim();
        if !h.is_empty() {
            v.push(h.into());
        }
    }
    // Conectar un UDP no envía nada: solo deja ver con qué IP saldría a la red local
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
        if s.connect("192.0.2.1:9").is_ok() {
            if let Ok(a) = s.local_addr() {
                if !a.ip().is_unspecified() {
                    v.push(a.ip().to_string());
                }
            }
        }
    }
    if let Ok(extra) = std::env::var("TM_TLS_NAMES") {
        v.extend(extra.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
    }
    v.sort();
    v.dedup();
    v
}

/// Devuelve (certificado, clave); los crea si faltan o si cambió la IP o el nombre de la máquina.
pub fn ensure_cert() -> anyhow::Result<(PathBuf, PathBuf)> {
    use rcgen::{date_time_ymd, CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
    let d = dir();
    let (cert, key, list) = (d.join("auto-cert.pem"), d.join("auto-key.pem"), d.join("auto-names.txt"));
    let names = names();
    let joined = names.join("\n");
    if cert.exists() && key.exists() && std::fs::read_to_string(&list).is_ok_and(|s| s == joined) {
        return Ok((cert, key));
    }
    std::fs::create_dir_all(&d)?;
    let mut params = CertificateParams::default();
    params.subject_alt_names = names
        .iter()
        .map(|n| match n.parse::<IpAddr>() {
            Ok(ip) => Ok(SanType::IpAddress(ip)),
            Err(_) => Ok(SanType::DnsName(n.clone().try_into()?)),
        })
        .collect::<anyhow::Result<_>>()?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Task Manager (certificado propio)");
    params.distinguished_name = dn;
    params.not_before = date_time_ymd(2020, 1, 1);
    params.not_after = date_time_ymd(2034, 1, 1);
    let pair = KeyPair::generate()?;
    let c = params.self_signed(&pair)?;
    std::fs::write(&cert, c.pem())?;
    std::fs::write(&key, pair.serialize_pem())?;
    std::fs::write(&list, joined)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600));
    }
    tracing::info!("certificado HTTPS propio creado para: {}", names.join(", "));
    Ok((cert, key))
}

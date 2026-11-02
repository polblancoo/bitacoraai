//! Cifrado en reposo de las credenciales guardadas en la base (tokens de GitHub y de Google): AES-256-GCM.
//! La clave NO vive en la base: se lee de `TM_KEY_FILE` o de `~/.config/tm-server/credenciales.key` (se crea sola, permisos 600).
//! Así, quien se lleve solo la base (una copia de seguridad, por ejemplo) no puede leer los tokens.
//! Lo que ya estaba guardado en claro se cifra al arrancar; si algo no está cifrado, `open` lo devuelve tal cual.
use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce};
use sqlx::SqlitePool;

const PREFIX: &str = "enc1:";

pub struct Cred(Aes256Gcm);

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    (s.len() % 2 == 0).then(|| ()).and_then(|_| (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect())
}

fn key_path() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("TM_KEY_FILE") {
        if !p.trim().is_empty() {
            return Some(p.trim().into());
        }
    }
    std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join(".config/tm-server/credenciales.key"))
}

/// Lee (o crea) la clave y deja cifrado lo que estuviera en claro.
pub async fn init(db: &SqlitePool) -> anyhow::Result<Cred> {
    let path = key_path().ok_or_else(|| anyhow::anyhow!("no encuentro dónde guardar la clave de credenciales (definí TM_KEY_FILE)"))?;
    let key = match std::fs::read_to_string(&path).ok().and_then(|s| unhex(s.trim())).filter(|k| k.len() == 32) {
        Some(k) => k,
        None => {
            if path.exists() {
                anyhow::bail!("{} existe pero no es una clave válida: no la piso para no perder las credenciales", path.display());
            }
            let mut k = [0u8; 32];
            getrandom::fill(&mut k).map_err(|e| anyhow::anyhow!("sin aleatorios: {e}"))?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, hex(&k))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            }
            tracing::info!("credenciales: clave nueva en {}", path.display());
            k.to_vec()
        }
    };
    let c = Cred(Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("clave inválida: {e}"))?);
    for (select, update) in [
        ("SELECT user_id, token FROM github_links WHERE token NOT LIKE 'enc1:%'", "UPDATE github_links SET token = ? WHERE user_id = ?"),
        ("SELECT user_id, refresh_token FROM google_links WHERE refresh_token NOT LIKE 'enc1:%'", "UPDATE google_links SET refresh_token = ? WHERE user_id = ?"),
    ] {
        let rows: Vec<(i64, String)> = sqlx::query_as(select).fetch_all(db).await?;
        for (id, plain) in rows {
            sqlx::query(update).bind(c.seal(&plain)).bind(id).execute(db).await?;
        }
    }
    Ok(c)
}

impl Cred {
    pub fn seal(&self, plain: &str) -> String {
        let mut nonce = [0u8; 12];
        let _ = getrandom::fill(&mut nonce);
        match self.0.encrypt(Nonce::from_slice(&nonce), plain.as_bytes()) {
            Ok(ct) => format!("{PREFIX}{}{}", hex(&nonce), hex(&ct)),
            Err(_) => String::new(),
        }
    }

    /// Cifra bytes (archivos del chat): 12 bytes de nonce al principio y el texto cifrado después.
    pub fn seal_bytes(&self, plain: &[u8]) -> Vec<u8> {
        let mut nonce = [0u8; 12];
        let _ = getrandom::fill(&mut nonce);
        match self.0.encrypt(Nonce::from_slice(&nonce), plain) {
            Ok(ct) => [nonce.to_vec(), ct].concat(),
            Err(_) => vec![],
        }
    }

    pub fn open_bytes(&self, stored: &[u8]) -> Option<Vec<u8>> {
        if stored.len() < 13 {
            return None;
        }
        let (nonce, ct) = stored.split_at(12);
        self.0.decrypt(Nonce::from_slice(nonce), ct).ok()
    }

    pub fn open(&self, stored: &str) -> Option<String> {
        let Some(body) = stored.strip_prefix(PREFIX) else { return Some(stored.to_string()) };
        let raw = unhex(body)?;
        if raw.len() < 13 {
            return None;
        }
        let (nonce, ct) = raw.split_at(12);
        String::from_utf8(self.0.decrypt(Nonce::from_slice(nonce), ct).ok()?).ok()
    }
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub server: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub lang: Option<String>,
}

fn path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "rust-tui-manager").map(|d| d.config_dir().join("client.json"))
}

pub fn load() -> Config {
    path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(c: &Config) {
    let c = &Config { lang: Some(crate::i18n::code().into()), server: c.server.clone(), token: c.token.clone(), username: c.username.clone() };
    if let Some(p) = path() {
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        let _ = std::fs::write(&p, serde_json::to_string_pretty(c).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
        }
    }
}

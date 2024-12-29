use std::path::{Path, PathBuf};
use std::fs;

pub struct Config {
    pub base_dir: PathBuf,
    pub db_path: PathBuf,
    pub code_dirs: CodeDirs,
}

pub struct CodeDirs {
    pub rust: PathBuf,
    pub python: PathBuf,
    pub text: PathBuf,
}

impl Config {
    pub fn new() -> std::io::Result<Self> {
        let base_dir = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config/rust-tui-manager")
        } else {
            PathBuf::from(".")
        };

        // Crear directorio base
        fs::create_dir_all(&base_dir)?;

        // Crear directorios para código
        let code_base = base_dir.join("code");
        let code_dirs = CodeDirs {
            rust: create_code_dir(&code_base, "rust")?,
            python: create_code_dir(&code_base, "python")?,
            text: create_code_dir(&code_base, "text")?,
        };

        let db_path = base_dir.join("data.db");

        Ok(Self {
            base_dir,
            db_path,
            code_dirs,
        })
    }

    pub fn get_code_dir(&self, language: &crate::languages::Language) -> &Path {
        match language {
            crate::languages::Language::Rust => &self.code_dirs.rust,
            crate::languages::Language::Python => &self.code_dirs.python,
            crate::languages::Language::None => &self.code_dirs.text,
        }
    }
}

fn create_code_dir(base: &Path, name: &str) -> std::io::Result<PathBuf> {
    let dir = base.join(name);
    fs::create_dir_all(&dir)?;
    Ok(dir)
} 
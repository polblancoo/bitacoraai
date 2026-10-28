use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::Write;
use crate::languages::Language;
use crate::error::{Result, Error};
use chrono::Utc;
mod syntax;

pub const MAX_CODE_SIZE: usize = 50_000; // 50KB
pub const MAX_LINES: usize = 1000;

#[derive(Clone)]
pub struct CodeHandler {
    base_path: PathBuf,
    bat_available: bool,
    pygments_available: bool,
}

impl CodeHandler {
    pub fn new() -> Self {
        let base_path = if let Ok(home) = std::env::var("HOME") {
            let config_dir = PathBuf::from(home).join(".config/rust-tui-manager");
            // Crear directorios para cada lenguaje
            for lang in &["rust", "python", "text"] {
                let lang_dir = config_dir.join("code").join(lang);
                if let Err(e) = fs::create_dir_all(&lang_dir) {
                    eprintln!("Error creando directorio {}: {}", lang_dir.display(), e);
                }
            }
            config_dir
        } else {
            PathBuf::from(".")
        };

        Self {
            base_path,
            bat_available: Command::new("bat").arg("--version").output().is_ok(),
            pygments_available: Command::new("pygmentize").arg("-V").output().is_ok(),
        }
    }

    pub fn save_code(&self, code: &str, language: &Language) -> Result<String> {
        let lang_dir = match language {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::None => "text",
        };

        let extension = language.get_support().extension;
        let timestamp = Utc::now().timestamp();
        let filename = format!("code_{}.{}", timestamp, extension);
        let dir_path = self.base_path.join("code").join(lang_dir);
        let file_path = dir_path.join(&filename);
        
        // Verificar y crear directorio si no existe
        fs::create_dir_all(&dir_path)?;

        // Formatear código según el lenguaje
        let formatted_code = match language {
            Language::Rust => {
                if Command::new("rustfmt").arg("--version").output().is_ok() {
                    let mut child = Command::new("rustfmt")
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()?;

                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(code.as_bytes())?;
                    }

                    let output = child.wait_with_output()?;
                    if output.status.success() {
                        String::from_utf8_lossy(&output.stdout).to_string()
                    } else {
                        code.to_string()
                    }
                } else {
                    code.to_string()
                }
            },
            _ => code.to_string(),
        };

        // Guardar código
        fs::write(&file_path, formatted_code)?;
        
        Ok(file_path.to_string_lossy().to_string())
    }

    pub fn delete_code(&self, path: &str) -> Result<()> {
        if let Ok(path) = PathBuf::from(path).canonicalize() {
            if path.starts_with(&self.base_path) {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    fn format_code(&self, code: &str, language: &Language) -> Result<String> {
        match language {
            Language::Rust => {
                if Command::new("rustfmt").arg("--version").output().is_ok() {
                    let mut child = Command::new("rustfmt")
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()?;

                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(code.as_bytes())?;
                    }

                    let output = child.wait_with_output()?;
                    if output.status.success() {
                        Ok(String::from_utf8_lossy(&output.stdout).to_string())
                    } else {
                        Ok(code.to_string())
                    }
                } else {
                    Ok(code.to_string())
                }
            },
            _ => Ok(code.to_string()),
        }
    }

    pub fn get_highlighted_code(&self, path: &str, language: &Language) -> Result<String> {
        let code = fs::read_to_string(path)?;
        let lang_str = match language {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::None => "text",
        };
        
        Ok(syntax::highlight_code(&code, lang_str))
    }

    pub fn get_tools_status(&self) -> String {
        let mut status = Vec::new();
        if self.bat_available {
            status.push("bat ✅");
        } else {
            status.push("bat ❌");
        }
        if self.pygments_available {
            status.push("pygments ✅");
        } else {
            status.push("pygments ❌");
        }
        status.join(" | ")
    }

    pub fn validate_code(&self, code: &str) -> Result<(bool, String)> {
        let size = code.len();
        let lines = code.lines().count();
        let mut warnings = Vec::new();

        if size > MAX_CODE_SIZE {
            warnings.push(format!("⚠️ El código excede el límite de {}KB", MAX_CODE_SIZE/1000));
        }
        if lines > MAX_LINES {
            warnings.push(format!("⚠️ El código excede el límite de {} líneas", MAX_LINES));
        }

        let has_warnings = !warnings.is_empty();
        Ok((has_warnings, warnings.join("\n")))
    }

    pub fn truncate_if_needed(&self, code: &str) -> String {
        let mut result = code.to_string();
        if result.len() > MAX_CODE_SIZE {
            result.truncate(MAX_CODE_SIZE);
            result.push_str("\n... (código truncado)");
        }
        result
    }

    pub fn change_language(&self, old_path: &str, new_language: &Language) -> Result<String> {
        let old_path = PathBuf::from(old_path);
        if !old_path.exists() {
            return Err(Error::Application("Archivo no encontrado".to_string()));
        }

        let code = fs::read_to_string(&old_path)?;
        
        // Crear nuevo path con la extensión correcta
        let new_dir = match new_language {
            Language::Rust => self.base_path.join("code").join("rust"),
            Language::Python => self.base_path.join("code").join("python"),
            Language::None => self.base_path.join("code").join("text"),
        };

        let timestamp = Utc::now().timestamp();
        let extension = new_language.get_support().extension;
        let new_filename = format!("code_{}.{}", timestamp, extension);
        let new_path = new_dir.join(new_filename);

        // Asegurar que existe el directorio
        fs::create_dir_all(&new_dir)?;

        // Copiar el contenido al nuevo archivo
        fs::write(&new_path, code)?;

        // Eliminar el archivo antiguo
        fs::remove_file(old_path)?;

        Ok(new_path.to_string_lossy().to_string())
    }
} 
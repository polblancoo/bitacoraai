use crate::error::Result;
use crate::app::App;
use std::fs;
use std::path::PathBuf;
use std::env;

pub struct Storage {
    file_path: PathBuf,
}

impl Storage {
    pub fn new(file_name: &str) -> Self {
        let config_dir = if let Ok(home) = env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("rust-tui-manager");
            path
        } else {
            PathBuf::from(".") // Fallback al directorio actual si no hay HOME
        };

        // Crear el directorio si no existe
        fs::create_dir_all(&config_dir).unwrap_or_default();

        let file_path = config_dir.join(file_name);
        
        // Si el archivo no existe, crear el directorio y copiar el default
        if !file_path.exists() {
            let default_content = include_str!("data/default.json");
            fs::write(&file_path, default_content).unwrap_or_default();
        }

        Self { file_path }
    }

    pub fn save(&self, app: &App) -> Result<()> {
        let json = app.save_state()?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }

    pub fn load(&self) -> Result<String> {
        match fs::read_to_string(&self.file_path) {
            Ok(contents) => Ok(contents),
            Err(_) => {
                // Si hay error al leer, devolver el contenido por defecto
                let default_content = include_str!("data/default.json");
                Ok(default_content.to_string())
            }
        }
    }
} 
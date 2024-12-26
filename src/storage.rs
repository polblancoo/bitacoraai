use crate::error::Result;
use crate::app::App;
use std::fs;

pub struct Storage {
    file_path: String,
}

impl Storage {
    pub fn new(file_path: String) -> Self {
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
            Err(_) => Ok(String::from("{}"))  // Devolver un objeto JSON vacío si el archivo no existe
        }
    }
} 
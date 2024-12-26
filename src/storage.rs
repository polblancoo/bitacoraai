use crate::app::{Section, Detail};
use crate::error::Result;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::Local;

const DEFAULT_DATA: &str = include_str!("data/default.json");

#[derive(Deserialize, Serialize)]
struct Data {
    sections: Vec<Section>,
}

pub struct Storage {
    file_path: String,
}

impl Storage {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }

    fn validate_json(&self, json: &str) -> Result<()> {
        match serde_json::from_str::<Data>(json) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Error validando JSON: {}", e);
                println!("JSON problemático: {}", json);
                Err(crate::error::Error::Serialization(e))
            }
        }
    }

    pub fn load(&self) -> Result<Vec<Section>> {
        let data = if Path::new(&self.file_path).exists() {
            fs::read_to_string(&self.file_path)?
        } else {
            self.validate_json(DEFAULT_DATA)?;
            fs::write(&self.file_path, DEFAULT_DATA)?;
            DEFAULT_DATA.to_string()
        };

        let data: Data = serde_json::from_str(&data)
            .map_err(|e| crate::error::Error::Serialization(e))?;
        Ok(data.sections)
    }

    pub fn save(&self, sections: &[Section]) -> Result<()> {
        let data = Data {
            sections: sections.to_vec(),
        };
        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| crate::error::Error::Serialization(e))?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }

    fn debug_json(&self, json: &str) -> Result<()> {
        match serde_json::from_str::<Data>(json) {
            Ok(_) => println!("JSON válido"),
            Err(e) => println!("Error al parsear JSON: {}\nJSON: {}", e, json),
        }
        Ok(())
    }

    pub fn create_detail(&self, title: String, description: String) -> Detail {
        Detail {
            id: 0,
            title,
            description,
            created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
} 
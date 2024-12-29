use crate::error::Result;
use crate::app::App;
use crate::db::Database;
use std::path::PathBuf;
use std::env;

pub struct Storage {
    db: Database,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let config_dir = if let Ok(home) = env::var("HOME") {
            let path = PathBuf::from(home).join(".config/rust-tui-manager");
            std::fs::create_dir_all(&path)?;
            path
        } else {
            PathBuf::from(".")
        };

        let db_path = config_dir.join("data.db");
        let db = Database::new(db_path.to_str().unwrap())?;
        
        Ok(Self { db })
    }

    pub fn load(&self) -> Result<App> {
        let sections = self.db.load_sections()?;
        Ok(App::from_sections(sections))
    }

    pub fn save(&self, app: &App) -> Result<()> {
        for section in &app.sections {
            self.db.save_section(section)?;
        }
        Ok(())
    }
} 
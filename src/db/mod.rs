use rusqlite::{Connection, Result};
use crate::app::{Section, Detail};
use crate::languages::Language;
use std::str::FromStr;

pub struct Database {
    conn: Connection,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: Connection::open(self.conn.path().unwrap()).unwrap()
        }
    }
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        
        // Crear tablas si no existen
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sections (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS details (
                id INTEGER PRIMARY KEY,
                section_id INTEGER,
                title TEXT NOT NULL,
                description TEXT,
                code_path TEXT,
                language TEXT NOT NULL DEFAULT 'none',
                created_at TEXT NOT NULL,
                FOREIGN KEY(section_id) REFERENCES sections(id)
            )",
            [],
        )?;

        // Insertar datos por defecto si las tablas están vacías
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM sections", [], |row| row.get(0))?;
        
        if count == 0 {
            // Insertar secciones por defecto
            conn.execute(
                "INSERT INTO sections (id, title) VALUES (1, '📝 Notas')",
                [],
            )?;
            conn.execute(
                "INSERT INTO sections (id, title) VALUES (2, '🔧 Comandos')",
                [],
            )?;

            // Insertar detalles por defecto
            conn.execute(
                "INSERT INTO details (id, section_id, title, description, language, created_at)
                 VALUES (1, 1, 'Bienvenido', 'Bienvenido a Rust TUI Manager.\nUsa h para ver la ayuda.', 'none', datetime('now'))",
                [],
            )?;
            conn.execute(
                "INSERT INTO details (id, section_id, title, description, language, created_at)
                 VALUES (2, 1, 'Primeros pasos', 'Usa Tab para moverte entre paneles.\nUsa a para agregar, e para editar, d para eliminar.', 'none', datetime('now'))",
                [],
            )?;
            conn.execute(
                "INSERT INTO details (id, section_id, title, description, language, created_at)
                 VALUES (3, 2, 'Atajos básicos', 'Tab: cambiar panel\na: agregar\ne: editar\nd: eliminar', 'none', datetime('now'))",
                [],
            )?;
            conn.execute(
                "INSERT INTO details (id, section_id, title, description, language, created_at)
                 VALUES (4, 2, 'Atajos avanzados', 'Ctrl+S: guardar\nCtrl+L: cambiar lenguaje\nEsc: cancelar', 'none', datetime('now'))",
                [],
            )?;
        }

        Ok(Self { conn })
    }

    pub fn load_sections(&self) -> Result<Vec<Section>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title FROM sections ORDER BY id"
        )?;
        
        let sections = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            
            let mut detail_stmt = self.conn.prepare(
                "SELECT id, title, description, code_path, language, created_at 
                 FROM details 
                 WHERE section_id = ? 
                 ORDER BY id"
            )?;
            
            let details = detail_stmt.query_map([id], |row| {
                Ok(Detail {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    code_path: row.get(3)?,
                    language: Language::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(Language::None),
                    created_at: row.get(5)?,
                })
            })?.collect::<Result<Vec<_>>>()?;

            Ok(Section {
                id: id as usize,
                title,
                details,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(sections)
    }

    pub fn save_section(&self, section: &Section) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sections (id, title) VALUES (?1, ?2)",
            rusqlite::params![section.id as i64, section.title],
        )?;

        for detail in &section.details {
            self.conn.execute(
                "INSERT OR REPLACE INTO details 
                (id, section_id, title, description, code_path, language, created_at) 
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    detail.id as i64,
                    section.id as i64,
                    detail.title,
                    detail.description,
                    detail.code_path.as_deref().unwrap_or(""),
                    detail.language.to_string(),
                    detail.created_at,
                ],
            )?;
        }

        Ok(())
    }

    pub fn delete_section(&self, id: usize) -> Result<()> {
        self.conn.execute("DELETE FROM details WHERE section_id = ?", [id as i64])?;
        self.conn.execute("DELETE FROM sections WHERE id = ?", [id as i64])?;
        Ok(())
    }

    pub fn delete_detail(&self, section_id: usize, detail_id: usize) -> Result<()> {
        self.conn.execute(
            "DELETE FROM details WHERE section_id = ? AND id = ?",
            [section_id as i64, detail_id as i64],
        )?;
        Ok(())
    }

    pub fn search_local(&self, query: &str) -> Result<Vec<(Section, Detail)>> {
        let query = format!("%{}%", query.to_lowercase());
        
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.title, d.id, d.title, d.description, d.code_path, d.language, d.created_at 
             FROM sections s 
             LEFT JOIN details d ON s.id = d.section_id 
             WHERE LOWER(s.title) LIKE ?1 
                OR LOWER(d.title) LIKE ?1 
                OR LOWER(d.description) LIKE ?1 
             ORDER BY s.id, d.id"
        )?;

        let results = stmt.query_map([&query], |row| {
            let section = Section {
                id: row.get(0)?,
                title: row.get(1)?,
                details: vec![],
            };
            
            let detail = Detail {
                id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                code_path: row.get(5)?,
                language: Language::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or(Language::None),
                created_at: row.get(7)?,
            };

            Ok((section, detail))
        })?.collect::<Result<Vec<_>>>()?;

        Ok(results)
    }

    pub fn get_latest_entries(&self, limit: i64) -> Result<Vec<(Section, Detail)>> {
        let mut stmt = self.conn.prepare(crat
            "SELECT s.id, s.title, d.id, d.title, d.description, d.code_path, d.language, d.created_at 
             FROM sections s 
             JOIN details d ON s.id = d.section_id 
             ORDER BY d.created_at DESC 
             LIMIT ?"
        )?;

        let results = stmt.query_map([limit], |row| {
            let section = Section {
                id: row.get(0)?,
                title: row.get(1)?,
                details: vec![],
            };
            
            let detail = Detail {
                id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                code_path: row.get(5)?,
                language: Language::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or(Language::None),
                created_at: row.get(7)?,
            };

            Ok((section, detail))
        })?.collect::<Result<Vec<_>>>()?;

        Ok(results)
    }
} 
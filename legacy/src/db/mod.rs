use crate::app::{Detail, Section};
use crate::languages::Language;
use rusqlite::{Connection, Result, params};
use std::str::FromStr;

pub struct Database {
    conn: Connection,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: Connection::open(self.conn.path().unwrap()).unwrap(),
        }
    }
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Habilitar foreign keys antes de crear las tablas
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Crear tablas si no existen
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS details (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                section_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                code_path TEXT,
                language TEXT NOT NULL DEFAULT 'none',
                created_at TEXT NOT NULL,
                FOREIGN KEY(section_id) REFERENCES sections(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Insertar sección por defecto si no hay ninguna
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sections",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            conn.execute(
                "INSERT INTO sections (title) VALUES (?1)",
                params!["📁 Notas"],
            )?;
        }

        Ok(Self { conn })
    }

    pub fn save_section(&mut self, section: &Section) -> Result<()> {
        // Comenzar transacción
        let tx = self.conn.transaction()?;

        let section_id = if section.id == 0 {
            // Nueva sección
            tx.execute(
                "INSERT INTO sections (title) VALUES (?1)",
                params![&section.title],
            )?;
            tx.last_insert_rowid()
        } else {
            // Verificar que la sección existe
            let exists: bool = tx.query_row(
                "SELECT 1 FROM sections WHERE id = ?1",
                params![section.id as i64],
                |_| Ok(true),
            ).unwrap_or(false);

            if !exists {
                // Si la sección no existe, crearla con el ID especificado
                tx.execute(
                    "INSERT INTO sections (id, title) VALUES (?1, ?2)",
                    params![section.id as i64, &section.title],
                )?;
                section.id as i64
            } else {
                // Actualizar sección existente
                tx.execute(
                    "UPDATE sections SET title = ?1 WHERE id = ?2",
                    params![&section.title, section.id as i64],
                )?;
                section.id as i64
            }
        };

        // Eliminar detalles antiguos
        tx.execute(
            "DELETE FROM details WHERE section_id = ?1",
            params![section_id],
        )?;

        // Insertar nuevos detalles
        for detail in &section.details {
            tx.execute(
                "INSERT INTO details (section_id, title, description, code_path, language, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    section_id,
                    &detail.title,
                    &detail.description,
                    detail.code_path.as_deref().unwrap_or(""),
                    detail.language.to_string(),
                    &detail.created_at,
                ],
            )?;
        }

        // Confirmar transacción
        tx.commit()?;
        Ok(())
    }

    pub fn load_sections(&self) -> Result<Vec<Section>> {
        let mut sections = Vec::new();
        
        let mut stmt = self.conn.prepare(
            "SELECT id, title FROM sections ORDER BY id"
        )?;
        
        let section_iter = stmt.query_map([], |row| {
            Ok(Section {
                id: row.get::<_, i64>(0)? as usize,
                title: row.get(1)?,
                details: Vec::new(),
            })
        })?;

        for section_result in section_iter {
            sections.push(section_result?);
        }

        // Cargar detalles para cada sección
        for section in &mut sections {
            let mut stmt = self.conn.prepare(
                "SELECT id, title, description, code_path, language, created_at 
                 FROM details 
                 WHERE section_id = ?1
                 ORDER BY id"
            )?;

            let details = stmt.query_map(params![section.id as i64], |row| {
                Ok(Detail {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    code_path: row.get(3)?,
                    language: Language::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(Language::None),
                    created_at: row.get(5)?,
                })
            })?;

            section.details = details.collect::<Result<Vec<_>>>()?;
        }

        Ok(sections)
    }

    pub fn search_local(&self, query: &str) -> Result<Vec<(Section, Detail)>> {
        let query = format!("%{}%", query.to_lowercase());
        let mut results = Vec::new();

        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.title, d.id, d.title, d.description, d.code_path, d.language, d.created_at
             FROM sections s
             LEFT JOIN details d ON s.id = d.section_id
             WHERE LOWER(s.title) LIKE ?1 
                OR LOWER(d.title) LIKE ?1 
                OR LOWER(d.description) LIKE ?1
             ORDER BY s.id, d.id"
        )?;

        let rows = stmt.query_map([&query], |row| {
            let section = Section {
                id: row.get::<_, i64>(0)? as usize,
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
        })?;

        for row_result in rows {
            results.push(row_result?);
        }

        Ok(results)
    }

    pub fn delete_section(&mut self, id: usize) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sections WHERE id = ?1", 
            params![id as i64]
        )?;
        Ok(())
    }

    pub fn delete_detail(&mut self, section_id: usize, detail_id: usize) -> Result<()> {
        self.conn.execute(
            "DELETE FROM details WHERE section_id = ?1 AND id = ?2",
            params![section_id as i64, detail_id as i64],
        )?;
        Ok(())
    }
}


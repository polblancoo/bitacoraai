use crate::error::Result;
use crate::search::{SearchProvider, SearchResult};
use crate::search::types::SearchSource;

pub struct CratesIoSearch;

impl CratesIoSearch {
    pub fn new() -> Self {
        Self
    }
}

impl SearchProvider for CratesIoSearch {
    async fn search(&mut self, query: &str) -> Result<Vec<SearchResult>> {
        // Por ahora retornamos un resultado de ejemplo
        Ok(vec![
            SearchResult {
                title: format!("Crate: {}", query),
                description: "Ejemplo de resultado de Crates.io".to_string(),
                source: SearchSource::CratesIo,
            }
        ])
    }
} 
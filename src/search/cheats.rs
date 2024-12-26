use crate::error::Result;
use crate::search::{SearchProvider, SearchResult};
use crate::search::types::SearchSource;

pub struct CheatsRsSearch;

impl CheatsRsSearch {
    pub fn new() -> Self {
        Self
    }
}

impl SearchProvider for CheatsRsSearch {
    async fn search(&mut self, query: &str) -> Result<Vec<SearchResult>> {
        // Por ahora retornamos un resultado de ejemplo
        Ok(vec![
            SearchResult {
                title: format!("Cheat para: {}", query),
                description: "Ejemplo de resultado de Cheats.rs".to_string(),
                source: SearchSource::CheatsRs,
            }
        ])
    }
}

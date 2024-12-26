use crate::error::Result;
use crate::search::{SearchProvider, SearchResult, SearchSource};
use reqwest;

pub struct CheatsRsSearch {
    client: reqwest::Client,
}

impl CheatsRsSearch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl SearchProvider for CheatsRsSearch {
    async fn search(&mut self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://cheat.sh/{}", query);
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "Rust-TUI-Manager")
            .send()
            .await?
            .text()
            .await?;

        // Convertir la respuesta de texto en un SearchResult
        Ok(vec![SearchResult {
            title: query.to_string(),
            description: response,
            source: SearchSource::CheatsRs,
        }])
    }
}

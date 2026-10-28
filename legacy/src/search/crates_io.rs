use crate::error::Result;
use crate::search::{SearchProvider, SearchResult, SearchSource};
use reqwest;
use serde::Deserialize;

#[derive(Deserialize)]
struct CratesResponse {
    crates: Vec<CrateInfo>,
}

#[derive(Deserialize)]
struct CrateInfo {
    name: String,
    description: Option<String>,
}

pub struct CratesIoSearch {
    client: reqwest::Client,
}

impl CratesIoSearch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl SearchProvider for CratesIoSearch {
    async fn search(&mut self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://crates.io/api/v1/crates?q={}&per_page=10", query);
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "Rust-TUI-Manager")
            .send()
            .await?
            .json::<CratesResponse>()
            .await?;

        Ok(response.crates.into_iter().map(|crate_info| SearchResult {
            title: crate_info.name,
            description: crate_info.description.unwrap_or_default(),
            source: SearchSource::CratesIo,
        }).collect())
    }
} 
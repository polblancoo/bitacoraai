pub mod types;
pub mod cheats;
pub mod crates_io;

pub use types::{SearchResult, SearchSource};

pub trait SearchProvider {
    async fn search(&mut self, query: &str) -> crate::error::Result<Vec<SearchResult>>;
}


//! Code indexing for the knowledge graph.

mod extractor;
mod generic;
mod patterns;

pub use generic::GenericIndexer;
pub use patterns::{DEFAULT_EXTENSIONS, MAX_CHUNK_SIZE, CHUNK_OVERLAP};

use async_trait::async_trait;
use std::path::Path;

use super::error::KnowledgeError;
use super::models::IndexStats;

/// Trait for indexing code into the knowledge graph.
#[async_trait]
pub trait Indexer: Send + Sync {
    /// Index a directory recursively.
    async fn index_directory(&self, path: &Path) -> Result<IndexStats, KnowledgeError>;

    /// Index a single file.
    async fn index_file(&self, path: &str, content: &str) -> Result<(), KnowledgeError>;
}

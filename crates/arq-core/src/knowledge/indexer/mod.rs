//! Code indexing for the knowledge graph.

mod extractor;
mod generic;
mod patterns;

pub use generic::GenericIndexer;
pub use patterns::{CHUNK_OVERLAP, DEFAULT_EXTENSIONS, MAX_CHUNK_SIZE};

use async_trait::async_trait;
use std::path::Path;

use super::error::KnowledgeError;
use super::models::IndexStats;

/// Progress information during indexing.
#[derive(Debug, Clone)]
pub struct IndexProgress {
    /// Current file being indexed.
    pub current_file: String,
    /// Number of files processed so far.
    pub files_done: usize,
    /// Total number of files to process.
    pub files_total: usize,
}

/// Trait for indexing code into the knowledge graph.
#[async_trait]
pub trait Indexer: Send + Sync {
    /// Index a directory recursively.
    async fn index_directory(&self, path: &Path) -> Result<IndexStats, KnowledgeError>;

    /// Index a directory with progress reporting.
    async fn index_directory_with_progress<F>(
        &self,
        path: &Path,
        on_progress: F,
    ) -> Result<IndexStats, KnowledgeError>
    where
        F: Fn(IndexProgress) + Send + Sync;

    /// Count files that will be indexed (for progress bar setup).
    fn count_indexable_files(&self, path: &Path) -> usize;

    /// Index a single file.
    async fn index_file(&self, path: &str, content: &str) -> Result<(), KnowledgeError>;
}

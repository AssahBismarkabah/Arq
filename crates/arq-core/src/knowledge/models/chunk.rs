//! Code chunk with embedding for semantic search.

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// A code chunk with its embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    /// Unique identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    /// File path containing this chunk.
    pub file_path: String,
    /// Entity ID (struct or function) this chunk belongs to, if any.
    pub entity_id: Option<String>,
    /// Entity type ("struct", "function", or "file").
    pub entity_type: String,
    /// The actual code content.
    pub content: String,
    /// Start line number.
    pub start_line: u32,
    /// End line number.
    pub end_line: u32,
    /// Embedding vector (384 dimensions for BGESmallENV15).
    pub embedding: Vec<f32>,
}

impl CodeChunk {
    /// Create a new code chunk without embedding.
    pub fn new(
        file_path: impl Into<String>,
        content: impl Into<String>,
        start_line: u32,
        end_line: u32,
    ) -> Self {
        Self {
            id: None,
            file_path: file_path.into(),
            entity_id: None,
            entity_type: "file".to_string(),
            content: content.into(),
            start_line,
            end_line,
            embedding: Vec::new(),
        }
    }

    /// Set the entity this chunk belongs to.
    pub fn with_entity(mut self, entity_id: impl Into<String>, entity_type: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self.entity_type = entity_type.into();
        self
    }

    /// Set the embedding vector.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = embedding;
        self
    }
}

/// Result from a semantic search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// File path.
    pub path: String,
    /// Similarity score (0.0 to 1.0).
    pub score: f32,
    /// Start line number.
    pub start_line: u32,
    /// End line number.
    pub end_line: u32,
    /// Preview of the content.
    pub preview: Option<String>,
    /// Entity ID if this chunk belongs to a struct/function.
    pub entity_id: Option<String>,
    /// Entity type.
    pub entity_type: String,
}

/// Statistics about the knowledge graph index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Number of indexed files.
    pub files: usize,
    /// Number of indexed structs/classes.
    pub structs: usize,
    /// Number of indexed functions/methods.
    pub functions: usize,
    /// Number of code chunks.
    pub chunks: usize,
    /// Total size of indexed files in bytes.
    pub total_size: u64,
    /// Last update time.
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

//! Knowledge graph error types.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur in the knowledge graph.
#[derive(Debug, Error)]
pub enum KnowledgeError {
    /// Database connection or query error.
    #[error("Database error: {0}")]
    Database(String),

    /// Embedding generation error.
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// File parsing error.
    #[error("Parse error in {path}: {message}")]
    Parse { path: String, message: String },

    /// IO error.
    #[error("IO error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Knowledge graph not initialized.
    #[error("Knowledge graph not initialized. Run 'arq init' first.")]
    NotInitialized,

    /// Entity not found.
    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    /// Index corrupted.
    #[error("Index corrupted: {0}")]
    Corrupted(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<std::io::Error> for KnowledgeError {
    fn from(err: std::io::Error) -> Self {
        KnowledgeError::Io {
            path: PathBuf::new(),
            source: err,
        }
    }
}

impl From<surrealdb::Error> for KnowledgeError {
    fn from(err: surrealdb::Error) -> Self {
        KnowledgeError::Database(err.to_string())
    }
}

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Invalid task directory: {0}")]
    InvalidDirectory(PathBuf),
}

impl StorageError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        StorageError::Io {
            path: path.into(),
            source,
        }
    }
}

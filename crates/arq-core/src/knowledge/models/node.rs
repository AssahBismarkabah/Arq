//! Node types for the knowledge graph.

use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

/// A file node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// Unique identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    /// Relative path from project root.
    pub path: String,
    /// File name.
    pub name: String,
    /// File extension.
    pub extension: String,
    /// SHA256 hash of file contents (for change detection).
    pub hash: String,
    /// File size in bytes.
    pub size: u64,
    /// When the file was indexed.
    pub indexed_at: Datetime,
}

impl FileNode {
    /// Create a new file node.
    pub fn new(path: impl Into<String>, hash: impl Into<String>, size: u64) -> Self {
        let path = path.into();
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let extension = std::path::Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        Self {
            id: None,
            path,
            name,
            extension,
            hash: hash.into(),
            size,
            indexed_at: Datetime::default(),
        }
    }
}

/// A struct/class node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructNode {
    /// Unique identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    /// Struct/class name.
    pub name: String,
    /// File containing this struct.
    pub file_path: String,
    /// Start line number.
    pub start_line: u32,
    /// End line number.
    pub end_line: u32,
    /// Visibility (pub, private, etc.).
    pub visibility: String,
    /// Documentation comment.
    pub doc_comment: Option<String>,
}

/// A function/method node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionNode {
    /// Unique identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    /// Function name.
    pub name: String,
    /// File containing this function.
    pub file_path: String,
    /// Parent struct (if this is a method).
    pub parent_struct: Option<String>,
    /// Start line number.
    pub start_line: u32,
    /// End line number.
    pub end_line: u32,
    /// Visibility.
    pub visibility: String,
    /// Whether the function is async.
    pub is_async: bool,
    /// Function signature.
    pub signature: String,
    /// Documentation comment.
    pub doc_comment: Option<String>,
}

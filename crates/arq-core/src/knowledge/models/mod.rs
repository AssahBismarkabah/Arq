//! Data models for the knowledge graph.

mod chunk;
mod node;

pub use chunk::{CodeChunk, IndexStats, SearchResult};
pub use node::{FileNode, FunctionNode, StructNode};

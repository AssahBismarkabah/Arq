//! Knowledge graph for semantic code search and dependency analysis.
//!
//! This module provides intelligent codebase indexing with:
//! - **Semantic search** via local embeddings (fastembed BGE-Small)
//! - **Graph-based dependency tracking** via SurrealDB relations
//! - **Impact analysis** for understanding code changes
//!
//! # Components
//!
//! - [`KnowledgeGraph`] - Main facade implementing [`KnowledgeStore`]
//! - [`KnowledgeDb`] - SurrealDB embedded database with HNSW vector index
//! - [`Embedder`] - Local embedding generation using fastembed
//! - [`indexer::GenericIndexer`] - Code chunking and indexing
//!
//! # Storage
//!
//! Uses SurrealDB embedded with RocksDB persistence. Stores:
//! - **Nodes**: File, Struct, Function entities
//! - **Edges**: CONTAINS, CALLS relations
//! - **Vectors**: 384-dimension embeddings with HNSW index for similarity search
//!
//! # Example
//!
//! ```ignore
//! use arq_core::knowledge::{KnowledgeGraph, KnowledgeStore};
//!
//! let kg = KnowledgeGraph::open("./knowledge.db").await?;
//! kg.initialize().await?;
//! kg.index_directory(".").await?;
//!
//! let results = kg.search_code("authentication handler", 10).await?;
//! ```

mod db;
mod embedder;
mod error;
pub mod indexer;
pub mod models;

pub use db::KnowledgeDb;
pub use embedder::Embedder;
pub use error::KnowledgeError;
pub use models::{CodeChunk, FileNode, FunctionNode, IndexStats, SearchResult, StructNode};

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Main interface for the knowledge graph.
///
/// Provides semantic search and graph traversal capabilities
/// for codebase analysis.
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    /// Initialize the knowledge store (create tables, indexes).
    async fn initialize(&self) -> Result<(), KnowledgeError>;

    /// Check if the store has been initialized.
    async fn is_initialized(&self) -> Result<bool, KnowledgeError>;

    /// Index a directory recursively.
    async fn index_directory(&self, path: &Path) -> Result<IndexStats, KnowledgeError>;

    /// Index a single file.
    async fn index_file(&self, path: &str, content: &str) -> Result<(), KnowledgeError>;

    /// Remove a file from the index.
    async fn remove_file(&self, path: &str) -> Result<(), KnowledgeError>;

    /// Semantic search for code relevant to a query.
    async fn search_code(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, KnowledgeError>;

    /// Get all entities that the given entity depends on.
    async fn get_dependencies(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError>;

    /// Get all entities that depend on the given entity (reverse dependencies).
    async fn get_impact(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError>;

    /// Get statistics about the indexed codebase.
    async fn get_stats(&self) -> Result<IndexStats, KnowledgeError>;
}

/// The main knowledge graph implementation.
pub struct KnowledgeGraph {
    db: Arc<KnowledgeDb>,
    embedder: Arc<dyn Embedder>,
}

impl KnowledgeGraph {
    /// Create a new knowledge graph with the given database path.
    pub async fn new(db_path: &Path) -> Result<Self, KnowledgeError> {
        let db = KnowledgeDb::open(db_path).await?;
        let embedder = embedder::FastEmbedder::new()?;

        Ok(Self {
            db: Arc::new(db),
            embedder: Arc::new(embedder),
        })
    }

    /// Open an existing knowledge graph.
    pub async fn open(db_path: &Path) -> Result<Self, KnowledgeError> {
        Self::new(db_path).await
    }
}

#[async_trait]
impl KnowledgeStore for KnowledgeGraph {
    async fn initialize(&self) -> Result<(), KnowledgeError> {
        self.db.initialize_schema().await
    }

    async fn is_initialized(&self) -> Result<bool, KnowledgeError> {
        self.db.is_initialized().await
    }

    async fn index_directory(&self, path: &Path) -> Result<IndexStats, KnowledgeError> {
        use indexer::Indexer;

        let indexer = indexer::GenericIndexer::new(
            Arc::clone(&self.db),
            Arc::clone(&self.embedder),
        );

        indexer.index_directory(path).await
    }

    async fn index_file(&self, path: &str, content: &str) -> Result<(), KnowledgeError> {
        use indexer::Indexer;

        let indexer = indexer::GenericIndexer::new(
            Arc::clone(&self.db),
            Arc::clone(&self.embedder),
        );

        indexer.index_file(path, content).await
    }

    async fn remove_file(&self, path: &str) -> Result<(), KnowledgeError> {
        self.db.remove_file(path).await
    }

    async fn search_code(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, KnowledgeError> {
        // Generate embedding for query
        let query_embedding = self.embedder.embed(&[query.to_string()])?;

        // Search using vector similarity
        self.db.search_by_embedding(&query_embedding[0], limit).await
    }

    async fn get_dependencies(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError> {
        self.db.get_dependencies(entity_id).await
    }

    async fn get_impact(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError> {
        self.db.get_impact(entity_id).await
    }

    async fn get_stats(&self) -> Result<IndexStats, KnowledgeError> {
        self.db.get_stats().await
    }
}

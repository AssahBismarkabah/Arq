//! Generic code indexer using regex-based extraction.

use async_trait::async_trait;
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::Indexer;
use crate::knowledge::db::KnowledgeDb;
use crate::knowledge::embedder::Embedder;
use crate::knowledge::error::KnowledgeError;
use crate::knowledge::models::{CodeChunk, FileNode, IndexStats};

/// Default extensions to index.
const DEFAULT_EXTENSIONS: &[&str] = &[
    // Systems languages
    "rs",
    "c",
    "cpp",
    "h",
    "hpp",
    "go",
    // JVM languages
    "java",
    "kt",
    "scala",
    // .NET languages
    "cs",
    "fs",
    // Scripting languages
    "py",
    "rb",
    "php",
    // JavaScript ecosystem
    "js",
    "ts",
    "tsx",
    "jsx",
    "vue",
    "svelte",
    // Mobile
    "swift",
    // Functional languages
    "ml",
    "hs",
    "ex",
    "exs",
    "clj",
    // Web
    "html",
    "css",
    "scss",
    // Config/Data
    "yaml",
    "yml",
    "toml",
    "json",
    // Documentation
    "md",
    // Database
    "sql",
];

/// Maximum chunk size in characters.
const MAX_CHUNK_SIZE: usize = 1000;

/// Chunk overlap in characters.
const CHUNK_OVERLAP: usize = 100;

/// Generic indexer that works with any language.
pub struct GenericIndexer {
    db: Arc<KnowledgeDb>,
    embedder: Arc<dyn Embedder>,
    extensions: Vec<String>,
}

impl GenericIndexer {
    /// Create a new generic indexer.
    pub fn new(db: Arc<KnowledgeDb>, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            db,
            embedder,
            extensions: DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a new generic indexer with custom extensions.
    pub fn with_extensions(
        db: Arc<KnowledgeDb>,
        embedder: Arc<dyn Embedder>,
        extensions: Vec<String>,
    ) -> Self {
        Self {
            db,
            embedder,
            extensions,
        }
    }

    /// Compute SHA256 hash of content.
    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Split content into overlapping chunks.
    fn chunk_content(content: &str, file_path: &str) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return chunks;
        }

        let mut current_chunk = String::new();
        let mut chunk_start_line = 1u32;
        let mut current_line = 1u32;

        for line in &lines {
            current_chunk.push_str(line);
            current_chunk.push('\n');

            if current_chunk.len() >= MAX_CHUNK_SIZE {
                chunks.push(CodeChunk::new(
                    file_path,
                    current_chunk.trim(),
                    chunk_start_line,
                    current_line,
                ));

                // Start new chunk with overlap
                let overlap_start = current_line.saturating_sub((CHUNK_OVERLAP / 40) as u32);
                current_chunk = lines
                    .iter()
                    .skip(overlap_start as usize - 1)
                    .take((current_line - overlap_start + 1) as usize)
                    .map(|s| format!("{}\n", s))
                    .collect();
                chunk_start_line = overlap_start;
            }

            current_line += 1;
        }

        // Add remaining content as final chunk
        if !current_chunk.trim().is_empty() {
            chunks.push(CodeChunk::new(
                file_path,
                current_chunk.trim(),
                chunk_start_line,
                current_line - 1,
            ));
        }

        chunks
    }

    /// Check if file extension is in the allowed list.
    fn should_index(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.extensions.iter().any(|e| e == ext))
            .unwrap_or(false)
    }
}

#[async_trait]
impl Indexer for GenericIndexer {
    async fn index_directory(&self, path: &Path) -> Result<IndexStats, KnowledgeError> {
        let mut stats = IndexStats::default();

        let walker = WalkBuilder::new(path)
            .hidden(true)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            let file_path = entry.path();

            if !file_path.is_file() || !self.should_index(file_path) {
                continue;
            }

            let relative_path = file_path
                .strip_prefix(path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            match fs::read_to_string(file_path) {
                Ok(content) => {
                    if let Err(e) = self.index_file(&relative_path, &content).await {
                        eprintln!("Warning: Failed to index {}: {}", relative_path, e);
                        continue;
                    }
                    stats.files += 1;
                    stats.total_size += content.len() as u64;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read {}: {}", relative_path, e);
                }
            }
        }

        stats.last_updated = Some(chrono::Utc::now());

        // Get chunk count from DB
        let db_stats = self.db.get_stats().await?;
        stats.chunks = db_stats.chunks;

        Ok(stats)
    }

    async fn index_file(&self, path: &str, content: &str) -> Result<(), KnowledgeError> {
        // Compute hash for change detection
        let hash = Self::compute_hash(content);

        // Check if file already indexed with same hash
        if let Some(existing) = self.db.get_file(path).await? {
            if existing.hash == hash {
                return Ok(()); // File unchanged, skip
            }
        }

        // Remove old data for this file
        self.db.remove_file(path).await?;

        // Create file node
        let file_node = FileNode::new(path, &hash, content.len() as u64);
        self.db.upsert_file(&file_node).await?;

        // Split into chunks
        let mut chunks = Self::chunk_content(content, path);

        if chunks.is_empty() {
            return Ok(());
        }

        // Generate embeddings for all chunks
        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embedder.embed(&texts)?;

        // Store chunks with embeddings
        for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
            chunk.embedding = embedding;
            self.db.insert_chunk(chunk).await?;
        }

        Ok(())
    }
}

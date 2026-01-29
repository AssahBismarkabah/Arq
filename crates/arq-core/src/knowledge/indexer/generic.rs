//! Generic code indexer using regex-based extraction.

use async_trait::async_trait;
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::extractor::{extract_calls, extract_functions, extract_line_range, extract_structs};
use super::patterns::{DEFAULT_EXTENSIONS, MAX_CHUNK_SIZE, CHUNK_OVERLAP};
use super::Indexer;
use crate::knowledge::db::KnowledgeDb;
use crate::knowledge::embedder::Embedder;
use crate::knowledge::error::KnowledgeError;
use crate::knowledge::models::{CodeChunk, FileNode, IndexStats};

/// Generic indexer that works with any language.
pub struct GenericIndexer {
    db: Arc<KnowledgeDb>,
    embedder: Arc<dyn Embedder>,
    extensions: Vec<String>,
}

impl GenericIndexer {
    /// Create a new generic indexer with default extensions.
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
        Self { db, embedder, extensions }
    }

    /// Check if file extension is in the allowed list.
    fn should_index(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.extensions.iter().any(|e| e == ext))
            .unwrap_or(false)
    }

    /// Compute SHA256 hash of content for change detection.
    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Split content into overlapping chunks for embedding.
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
                let overlap_lines = (CHUNK_OVERLAP / 40) as u32;
                let overlap_start = current_line.saturating_sub(overlap_lines);
                current_chunk = lines
                    .iter()
                    .skip(overlap_start.saturating_sub(1) as usize)
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

    /// Index structs and functions, creating graph relations.
    async fn index_code_entities(&self, path: &str, content: &str) -> Result<(), KnowledgeError> {
        let structs = extract_structs(content, path);
        let functions = extract_functions(content, path);

        // Store structs and create contains relations
        for s in &structs {
            if let Ok(struct_id) = self.db.insert_struct(s).await {
                let _ = self.db.relate_contains(path, &struct_id).await;
            }
        }

        // Store functions and create relations
        for f in &functions {
            if let Ok(fn_id) = self.db.insert_function(f).await {
                let _ = self.db.relate_contains(path, &fn_id).await;

                // Link methods to parent struct
                if let Some(parent) = &f.parent_struct {
                    let _ = self.db.relate_has_method(parent, &fn_id).await;
                }

                // Extract and create call relations
                let fn_content = extract_line_range(content, f.start_line, f.end_line);
                for callee in extract_calls(&fn_content) {
                    let _ = self.db.relate_calls(&fn_id, &callee).await;
                }
            }
        }

        Ok(())
    }

    /// Generate and store embeddings for code chunks.
    async fn index_embeddings(&self, path: &str, content: &str) -> Result<(), KnowledgeError> {
        let mut chunks = Self::chunk_content(content, path);

        if chunks.is_empty() {
            return Ok(());
        }

        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embedder.embed(&texts)?;

        for (chunk, embedding) in chunks.iter_mut().zip(embeddings) {
            chunk.embedding = embedding;
            self.db.insert_chunk(chunk).await?;
        }

        Ok(())
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

        // Get counts from DB
        let db_stats = self.db.get_stats().await?;
        stats.chunks = db_stats.chunks;
        stats.structs = db_stats.structs;
        stats.functions = db_stats.functions;

        Ok(stats)
    }

    async fn index_file(&self, path: &str, content: &str) -> Result<(), KnowledgeError> {
        let hash = Self::compute_hash(content);

        // Skip if unchanged
        if let Some(existing) = self.db.get_file(path).await? {
            if existing.hash == hash {
                return Ok(());
            }
        }

        // Remove old data and create new file node
        self.db.remove_file(path).await?;
        let file_node = FileNode::new(path, &hash, content.len() as u64);
        self.db.upsert_file(&file_node).await?;

        // Index code entities (structs, functions, relations)
        self.index_code_entities(path, content).await?;

        // Index embeddings
        self.index_embeddings(path, content).await?;

        Ok(())
    }
}

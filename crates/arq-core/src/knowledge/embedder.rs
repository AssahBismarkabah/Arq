//! Embedding generation for semantic search.

use std::path::PathBuf;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use super::error::KnowledgeError;

/// Trait for embedding generation.
pub trait Embedder: Send + Sync {
    /// Generate embeddings for a batch of text.
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, KnowledgeError>;

    /// Get the embedding dimension.
    fn dimension(&self) -> usize;

    /// Get the model name.
    fn model_name(&self) -> &str;
}

/// FastEmbed-based embedder using BGE-Small model.
pub struct FastEmbedder {
    model: TextEmbedding,
    dimension: usize,
    model_name: String,
}

impl FastEmbedder {
    /// Create a new FastEmbed embedder with the default model.
    /// Uses `~/.arq/cache/` as the model cache directory.
    pub fn new() -> Result<Self, KnowledgeError> {
        let cache_dir = Self::default_cache_dir();
        Self::with_model_and_cache(EmbeddingModel::BGESmallENV15, cache_dir)
    }

    /// Create a new FastEmbed embedder with a specific model.
    /// Uses `~/.arq/cache/` as the model cache directory.
    #[allow(dead_code)]
    pub fn with_model(model: EmbeddingModel) -> Result<Self, KnowledgeError> {
        let cache_dir = Self::default_cache_dir();
        Self::with_model_and_cache(model, cache_dir)
    }

    /// Create a new FastEmbed embedder with a specific model and cache directory.
    pub fn with_model_and_cache(
        model: EmbeddingModel,
        cache_dir: PathBuf,
    ) -> Result<Self, KnowledgeError> {
        let model_name = format!("{:?}", model);

        // Ensure cache directory exists
        std::fs::create_dir_all(&cache_dir).map_err(|e| {
            KnowledgeError::Embedding(format!("Failed to create cache directory: {}", e))
        })?;

        let text_embedding = TextEmbedding::try_new(
            InitOptions::new(model)
                .with_cache_dir(cache_dir)
                .with_show_download_progress(true),
        )
        .map_err(|e| KnowledgeError::Embedding(e.to_string()))?;

        // Get dimension from a test embedding
        let test_result = text_embedding
            .embed(vec!["test"], None)
            .map_err(|e| KnowledgeError::Embedding(e.to_string()))?;

        let dimension = test_result.first().map(|v| v.len()).unwrap_or(384);

        Ok(Self {
            model: text_embedding,
            dimension,
            model_name,
        })
    }

    /// Get the default cache directory: `~/.arq/cache/`
    fn default_cache_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".arq")
            .join("cache")
    }
}

impl Embedder for FastEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, KnowledgeError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let texts_vec: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        self.model
            .embed(texts_vec, None)
            .map_err(|e| KnowledgeError::Embedding(e.to_string()))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedder_dimension() {
        // This test requires downloading the model, so we skip it in CI
        if std::env::var("CI").is_ok() {
            return;
        }

        let embedder = FastEmbedder::new().expect("Failed to create embedder");
        assert_eq!(embedder.dimension(), 384); // BGE-Small produces 384-dim vectors
    }
}

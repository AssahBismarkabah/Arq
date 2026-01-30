mod error;
mod claude;
mod openai;
mod provider;

pub use error::LLMError;
pub use claude::ClaudeClient;
pub use openai::OpenAIClient;
pub use provider::Provider;

use async_trait::async_trait;
use tokio::sync::mpsc;

/// A chunk of streamed response from an LLM.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// The text content of this chunk.
    pub text: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
}

impl StreamChunk {
    /// Create a new text chunk.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_final: false,
        }
    }

    /// Create a final (end of stream) chunk.
    pub fn done() -> Self {
        Self {
            text: String::new(),
            is_final: true,
        }
    }
}

/// Trait for Large Language Model providers.
///
/// This abstraction allows swapping between different LLM providers
/// without changing the rest of the code.
///
/// # Supported Providers
///
/// - **OpenAI-compatible** (default): Works with OpenAI, Azure, Ollama, vLLM, OpenRouter, etc.
/// - **Anthropic**: Claude models via Anthropic API
/// - **Ollama**: Local models via Ollama
///
/// # Example
///
/// ```ignore
/// use arq_core::llm::{Provider, LLM};
///
/// // Auto-detect from environment
/// let llm = Provider::from_env()?;
///
/// // Or configure explicitly
/// let llm = Provider::Ollama {
///     base_url: None,
///     model: "llama3".to_string(),
/// }.build()?;
///
/// let response = llm.complete("Hello!").await?;
/// ```
#[async_trait]
pub trait LLM: Send + Sync {
    /// Complete a prompt and return the response.
    async fn complete(&self, prompt: &str) -> Result<String, LLMError>;

    /// Complete a prompt with a system message.
    async fn complete_with_system(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<String, LLMError>;

    /// Stream a completion with a system message.
    ///
    /// Sends chunks through the provided channel as they arrive.
    /// The final chunk will have `is_final: true`.
    ///
    /// Default implementation falls back to non-streaming and sends
    /// the entire response as a single chunk.
    async fn stream_complete(
        &self,
        system: &str,
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), LLMError> {
        // Default: fall back to non-streaming
        let response = self.complete_with_system(system, prompt).await?;
        let _ = tx.send(StreamChunk::text(response));
        let _ = tx.send(StreamChunk::done());
        Ok(())
    }

    /// Returns true if this provider supports streaming.
    fn supports_streaming(&self) -> bool {
        false
    }
}

/// Blanket implementation for boxed trait objects.
#[async_trait]
impl LLM for Box<dyn LLM> {
    async fn complete(&self, prompt: &str) -> Result<String, LLMError> {
        (**self).complete(prompt).await
    }

    async fn complete_with_system(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<String, LLMError> {
        (**self).complete_with_system(system, prompt).await
    }

    async fn stream_complete(
        &self,
        system: &str,
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), LLMError> {
        (**self).stream_complete(system, prompt, tx).await
    }

    fn supports_streaming(&self) -> bool {
        (**self).supports_streaming()
    }
}

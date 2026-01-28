mod error;
mod claude;
mod openai;
mod provider;

pub use error::LLMError;
pub use claude::ClaudeClient;
pub use openai::OpenAIClient;
pub use provider::Provider;

use async_trait::async_trait;

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
}

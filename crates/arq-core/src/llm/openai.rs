use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{LLMError, LLM};

const DEFAULT_MAX_TOKENS: u32 = 4096;

/// OpenAI-compatible API client.
///
/// Works with any provider that implements the OpenAI chat completions API:
/// - OpenAI
/// - Azure OpenAI
/// - Ollama (http://localhost:11434/v1)
/// - vLLM
/// - llama.cpp
/// - OpenRouter
/// - Together AI
/// - Groq
/// - Mistral
/// - And many more
pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    model: String,
    max_tokens: u32,
    client: Client,
}

impl OpenAIClient {
    /// Creates a new OpenAI-compatible client.
    ///
    /// # Arguments
    /// * `base_url` - The API base URL (e.g., "https://api.openai.com/v1")
    /// * `api_key` - The API key (can be empty for local providers like Ollama)
    /// * `model` - The model name (e.g., "gpt-4", "llama3", "mistral")
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            model: model.into(),
            max_tokens: DEFAULT_MAX_TOKENS,
            client: Client::new(),
        }
    }

    /// Creates a client for OpenAI.
    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new("https://api.openai.com/v1", api_key, model)
    }

    /// Creates a client for OpenAI from environment variables.
    /// Uses OPENAI_API_KEY and optionally OPENAI_MODEL.
    pub fn openai_from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| LLMError::MissingApiKey)?;
        let model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4o".to_string());
        Ok(Self::openai(api_key, model))
    }

    /// Creates a client for Ollama (local).
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::new("http://localhost:11434/v1", "", model)
    }

    /// Creates a client for OpenRouter.
    pub fn openrouter(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new("https://openrouter.ai/api/v1", api_key, model)
    }

    /// Creates a client from environment variables.
    /// Uses ARQ_LLM_BASE_URL, ARQ_LLM_API_KEY, and ARQ_LLM_MODEL.
    pub fn from_env() -> Result<Self, LLMError> {
        let base_url = std::env::var("ARQ_LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let api_key = std::env::var("ARQ_LLM_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .unwrap_or_default();
        let model = std::env::var("ARQ_LLM_MODEL")
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .unwrap_or_else(|_| "gpt-4o".to_string());

        Ok(Self::new(base_url, api_key, model))
    }

    /// Sets the maximum tokens for responses.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    async fn send_request(&self, messages: Vec<ChatMessage>, system: Option<&str>) -> Result<String, LLMError> {
        let mut all_messages = Vec::new();

        // Add system message if provided
        if let Some(sys) = system {
            all_messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.to_string(),
            });
        }

        all_messages.extend(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: all_messages,
            max_tokens: Some(self.max_tokens),
        };

        let url = format!("{}/chat/completions", self.base_url);

        let mut req = self.client
            .post(&url)
            .header("content-type", "application/json");

        // Only add authorization if api_key is not empty
        if !self.api_key.is_empty() {
            req = req.header("authorization", format!("Bearer {}", self.api_key));
        }

        let response = req.json(&request).send().await?;

        let status = response.status();

        if status == 429 {
            return Err(LLMError::RateLimited);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;

        // Extract content from first choice
        let content = chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(content)
    }
}

#[async_trait]
impl LLM for OpenAIClient {
    async fn complete(&self, prompt: &str) -> Result<String, LLMError> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.send_request(messages, None).await
    }

    async fn complete_with_system(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<String, LLMError> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.send_request(messages, Some(system)).await
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OpenAIClient::new(
            "https://api.example.com/v1",
            "test-key",
            "gpt-4",
        );
        assert_eq!(client.base_url, "https://api.example.com/v1");
        assert_eq!(client.model, "gpt-4");
    }

    #[test]
    fn test_openai_client() {
        let client = OpenAIClient::openai("test-key", "gpt-4o");
        assert_eq!(client.base_url, "https://api.openai.com/v1");
        assert_eq!(client.model, "gpt-4o");
    }

    #[test]
    fn test_ollama_client() {
        let client = OpenAIClient::ollama("llama3");
        assert_eq!(client.base_url, "http://localhost:11434/v1");
        assert_eq!(client.model, "llama3");
        assert!(client.api_key.is_empty());
    }

    #[test]
    fn test_openrouter_client() {
        let client = OpenAIClient::openrouter("test-key", "anthropic/claude-3-opus");
        assert_eq!(client.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(client.model, "anthropic/claude-3-opus");
    }

    #[test]
    fn test_url_trailing_slash_removed() {
        let client = OpenAIClient::new(
            "https://api.example.com/v1/",
            "key",
            "model",
        );
        assert_eq!(client.base_url, "https://api.example.com/v1");
    }
}

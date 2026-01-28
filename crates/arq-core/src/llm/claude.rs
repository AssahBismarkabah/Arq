use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_ANTHROPIC_API_VERSION, DEFAULT_ANTHROPIC_MODEL,
    DEFAULT_ANTHROPIC_URL, DEFAULT_MAX_TOKENS,
};
use super::{LLMError, LLM};

/// Claude API client.
pub struct ClaudeClient {
    api_key: String,
    api_url: String,
    api_version: String,
    model: String,
    max_tokens: u32,
    client: Client,
}

impl ClaudeClient {
    /// Creates a new Claude client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_url: DEFAULT_ANTHROPIC_URL.to_string(),
            api_version: DEFAULT_ANTHROPIC_API_VERSION.to_string(),
            model: DEFAULT_ANTHROPIC_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            client: Client::new(),
        }
    }

    /// Creates a Claude client from the ANTHROPIC_API_KEY environment variable.
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key =
            std::env::var("ANTHROPIC_API_KEY").map_err(|_| LLMError::MissingApiKey)?;
        Ok(Self::new(api_key))
    }

    /// Sets the model to use.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets the maximum tokens for responses.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Sets the API URL (for proxies or enterprise deployments).
    pub fn with_api_url(mut self, url: impl Into<String>) -> Self {
        self.api_url = url.into();
        self
    }

    /// Sets the API version.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    async fn send_request(&self, request: &ClaudeRequest) -> Result<String, LLMError> {
        let response = self
            .client
            .post(&self.api_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await?;

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

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;

        // Extract text from the first content block
        let text = claude_response
            .content
            .into_iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    Some(block.text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }
}

#[async_trait]
impl LLM for ClaudeClient {
    async fn complete(&self, prompt: &str) -> Result<String, LLMError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: None,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        self.send_request(&request).await
    }

    async fn complete_with_system(
        &self,
        system: &str,
        prompt: &str,
    ) -> Result<String, LLMError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: Some(system.to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        self.send_request(&request).await
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ClaudeClient::new("test-key");
        assert_eq!(client.model, DEFAULT_ANTHROPIC_MODEL);
        assert_eq!(client.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(client.api_url, DEFAULT_ANTHROPIC_URL);
        assert_eq!(client.api_version, DEFAULT_ANTHROPIC_API_VERSION);
    }

    #[test]
    fn test_client_with_model() {
        let client = ClaudeClient::new("test-key").with_model("claude-3-opus");
        assert_eq!(client.model, "claude-3-opus");
    }

    #[test]
    fn test_client_with_api_url() {
        let client = ClaudeClient::new("test-key")
            .with_api_url("https://proxy.example.com/v1/messages");
        assert_eq!(client.api_url, "https://proxy.example.com/v1/messages");
    }

    #[test]
    fn test_from_env_missing() {
        // Temporarily unset the env var
        std::env::remove_var("ANTHROPIC_API_KEY");
        let result = ClaudeClient::from_env();
        assert!(matches!(result, Err(LLMError::MissingApiKey)));
    }
}

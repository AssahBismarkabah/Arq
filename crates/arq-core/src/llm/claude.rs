use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{LLMError, LLM};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Claude API client.
pub struct ClaudeClient {
    api_key: String,
    model: String,
    max_tokens: u32,
    client: Client,
}

impl ClaudeClient {
    /// Creates a new Claude client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
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

    async fn send_request(&self, request: &ClaudeRequest) -> Result<String, LLMError> {
        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
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
        assert_eq!(client.model, DEFAULT_MODEL);
        assert_eq!(client.max_tokens, DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_client_with_model() {
        let client = ClaudeClient::new("test-key").with_model("claude-3-opus");
        assert_eq!(client.model, "claude-3-opus");
    }

    #[test]
    fn test_from_env_missing() {
        // Temporarily unset the env var
        std::env::remove_var("ANTHROPIC_API_KEY");
        let result = ClaudeClient::from_env();
        assert!(matches!(result, Err(LLMError::MissingApiKey)));
    }
}

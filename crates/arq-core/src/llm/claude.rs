use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::{LLMError, StreamChunk, LLM};
use crate::config::{
    DEFAULT_ANTHROPIC_API_VERSION, DEFAULT_ANTHROPIC_MODEL, DEFAULT_ANTHROPIC_URL,
    DEFAULT_MAX_TOKENS,
};

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
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| LLMError::MissingApiKey)?;
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

    /// Send a streaming request and forward chunks through the channel.
    async fn send_streaming_request(
        &self,
        request: &ClaudeRequest,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), LLMError> {
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

        // Process SSE stream
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LLMError::Network(e.to_string()))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete SSE events from buffer
            while let Some(pos) = buffer.find("\n\n") {
                let event_data = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                // Parse SSE event
                if let Some(text) = parse_claude_sse_event(&event_data) {
                    let _ = tx.send(StreamChunk::text(text));
                }
            }
        }

        // Send final chunk
        let _ = tx.send(StreamChunk::done());
        Ok(())
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
            stream: None,
        };

        self.send_request(&request).await
    }

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String, LLMError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: Some(system.to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: None,
        };

        self.send_request(&request).await
    }

    async fn stream_complete(
        &self,
        system: &str,
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), LLMError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: Some(system.to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: Some(true),
        };

        self.send_streaming_request(&request, tx).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
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

/// Parse a Claude SSE event and extract text from content_block_delta events.
///
/// Claude streaming format:
/// ```text
/// event: content_block_delta
/// data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}
/// ```
fn parse_claude_sse_event(event_data: &str) -> Option<String> {
    let mut event_type = None;
    let mut data_line = None;

    for line in event_data.lines() {
        if let Some(stripped) = line.strip_prefix("event: ") {
            event_type = Some(stripped.trim());
        } else if let Some(stripped) = line.strip_prefix("data: ") {
            data_line = Some(stripped.trim());
        }
    }

    // Only process content_block_delta events
    if event_type != Some("content_block_delta") {
        return None;
    }

    let data = data_line?;

    // Parse the JSON data
    #[derive(Deserialize)]
    struct DeltaEvent {
        delta: Delta,
    }

    #[derive(Deserialize)]
    struct Delta {
        #[serde(default)]
        text: String,
    }

    let parsed: DeltaEvent = serde_json::from_str(data).ok()?;

    if parsed.delta.text.is_empty() {
        None
    } else {
        Some(parsed.delta.text)
    }
}

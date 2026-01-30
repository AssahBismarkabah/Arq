use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::config::{
    DEFAULT_MAX_TOKENS, DEFAULT_OLLAMA_URL, DEFAULT_OPENAI_MODEL,
    DEFAULT_OPENAI_URL, DEFAULT_OPENROUTER_URL,
};
use super::{LLMError, LLM, StreamChunk};

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
        Self::new(DEFAULT_OPENAI_URL, api_key, model)
    }

    /// Creates a client for OpenAI from environment variables.
    /// Uses OPENAI_API_KEY and optionally OPENAI_MODEL.
    pub fn openai_from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| LLMError::MissingApiKey)?;
        let model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| DEFAULT_OPENAI_MODEL.to_string());
        Ok(Self::openai(api_key, model))
    }

    /// Creates a client for Ollama (local).
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::new(DEFAULT_OLLAMA_URL, "", model)
    }

    /// Creates a client for OpenRouter.
    pub fn openrouter(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(DEFAULT_OPENROUTER_URL, api_key, model)
    }

    /// Creates a client from environment variables.
    /// Uses ARQ_LLM_BASE_URL, ARQ_LLM_API_KEY, and ARQ_LLM_MODEL.
    pub fn from_env() -> Result<Self, LLMError> {
        let base_url = std::env::var("ARQ_LLM_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_OPENAI_URL.to_string());
        let api_key = std::env::var("ARQ_LLM_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .unwrap_or_default();
        let model = std::env::var("ARQ_LLM_MODEL")
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .unwrap_or_else(|_| DEFAULT_OPENAI_MODEL.to_string());

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
                ..Default::default()
            });
        }

        all_messages.extend(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: all_messages,
            max_tokens: Some(self.max_tokens),
            stream: None,
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

        // Get response as text first for better error messages
        let response_text = response.text().await
            .map_err(|e| LLMError::Network(format!("Failed to read response body: {}", e)))?;

        // Try to parse as JSON
        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| LLMError::ParseError(format!(
                "Failed to parse response: {}. Response: {}",
                e,
                &response_text[..response_text.len().min(500)]
            )))?;

        // Extract content from first choice
        let content = chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(content)
    }

    /// Send a streaming request and forward chunks through the channel.
    async fn send_streaming_request(
        &self,
        messages: Vec<ChatMessage>,
        system: Option<&str>,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), LLMError> {
        let mut all_messages = Vec::new();

        // Add system message if provided
        if let Some(sys) = system {
            all_messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.to_string(),
                ..Default::default()
            });
        }

        all_messages.extend(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: all_messages,
            max_tokens: Some(self.max_tokens),
            stream: Some(true),
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

        // Process SSE stream
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LLMError::Network(e.to_string()))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete SSE lines from buffer
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                // Parse SSE data line
                if let Some(text) = parse_openai_sse_line(&line) {
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
impl LLM for OpenAIClient {
    async fn complete(&self, prompt: &str) -> Result<String, LLMError> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            ..Default::default()
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
            ..Default::default()
        }];

        self.send_request(messages, Some(system)).await
    }

    async fn stream_complete(
        &self,
        system: &str,
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), LLMError> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            ..Default::default()
        }];

        self.send_streaming_request(messages, Some(system), tx).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ChatMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
    // Some providers include extra fields like thinking_blocks
    #[serde(flatten, default)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
    // Allow extra fields like usage, model, etc.
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
    // Allow extra fields like index, finish_reason
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Parse an OpenAI SSE line and extract text from delta content.
///
/// OpenAI streaming format:
/// ```text
/// data: {"id":"...","choices":[{"delta":{"content":"Hello"},"index":0}]}
/// data: [DONE]
/// ```
fn parse_openai_sse_line(line: &str) -> Option<String> {
    // Skip empty lines and non-data lines
    let data = line.strip_prefix("data: ")?;

    // Check for end of stream
    if data == "[DONE]" {
        return None;
    }

    // Parse the JSON data
    #[derive(Deserialize)]
    struct StreamResponse {
        choices: Vec<StreamChoice>,
    }

    #[derive(Deserialize)]
    struct StreamChoice {
        delta: Delta,
    }

    #[derive(Deserialize)]
    struct Delta {
        #[serde(default)]
        content: Option<String>,
    }

    let parsed: StreamResponse = serde_json::from_str(data).ok()?;

    parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.delta.content)
        .filter(|s| !s.is_empty())
}


use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::{
    tool_call::{LLMToolResponse, LLMWithTools, Message, MessageContent, MessageRole, ToolCall},
    LLMError, StreamChunk, LLM,
};
use crate::config::{
    DEFAULT_MAX_TOKENS, DEFAULT_OLLAMA_URL, DEFAULT_OPENAI_MODEL, DEFAULT_OPENAI_URL,
    DEFAULT_OPENROUTER_URL,
};

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
    /// * `base_url` - The API base URL (e.g., `https://api.openai.com/v1`)
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
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| LLMError::MissingApiKey)?;
        let model =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_OPENAI_MODEL.to_string());
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
        let base_url =
            std::env::var("ARQ_LLM_BASE_URL").unwrap_or_else(|_| DEFAULT_OPENAI_URL.to_string());
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

    async fn send_request(
        &self,
        messages: Vec<ChatMessage>,
        system: Option<&str>,
    ) -> Result<String, LLMError> {
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

        let mut req = self
            .client
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
        let response_text = response
            .text()
            .await
            .map_err(|e| LLMError::Network(format!("Failed to read response body: {}", e)))?;

        // Try to parse as JSON
        let chat_response: ChatResponse = serde_json::from_str(&response_text).map_err(|e| {
            LLMError::ParseError(format!(
                "Failed to parse response: {}. Response: {}",
                e,
                &response_text[..response_text.len().min(500)]
            ))
        })?;

        // Extract content from first choice
        let content = chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.get_content());

        match content {
            Some(text) if !text.is_empty() => Ok(text),
            Some(_) => Err(LLMError::ParseError(format!(
                "LLM returned empty content. Raw response: {}",
                &response_text[..response_text.len().min(500)]
            ))),
            None => Err(LLMError::ParseError(format!(
                "LLM response had no choices. Raw response: {}",
                &response_text[..response_text.len().min(500)]
            ))),
        }
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

        let mut req = self
            .client
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

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String, LLMError> {
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

        self.send_streaming_request(messages, Some(system), tx)
            .await
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
    /// Some providers (like Gemini) put content in reasoning_content
    #[serde(default)]
    reasoning_content: Option<String>,
    // Some providers include extra fields like thinking_blocks
    #[serde(flatten, default)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

impl ChatMessage {
    /// Get the actual content, trying content first, then reasoning_content
    fn get_content(&self) -> String {
        if !self.content.is_empty() {
            self.content.clone()
        } else if let Some(ref reasoning) = self.reasoning_content {
            reasoning.clone()
        } else {
            String::new()
        }
    }
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

// ============================================================================
// Tool Calling Support
// ============================================================================

/// Chat request with tools support.
#[derive(Debug, Serialize)]
struct ChatRequestWithTools {
    model: String,
    messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// Response structure that includes tool calls.
#[derive(Debug, Deserialize)]
struct ChatResponseWithTools {
    choices: Vec<ChoiceWithTools>,
}

#[derive(Debug, Deserialize)]
struct ChoiceWithTools {
    message: MessageWithTools,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageWithTools {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    _type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Deserialize)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

impl OpenAIClient {
    /// Convert our Message type to OpenAI format.
    fn message_to_openai(&self, msg: &Message) -> serde_json::Value {
        match &msg.content {
            MessageContent::Text(text) => {
                let mut obj = serde_json::json!({
                    "role": match msg.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "tool",
                    },
                    "content": text,
                });

                // Add tool_call_id for tool messages
                if let Some(ref tool_call_id) = msg.tool_call_id {
                    obj["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
                }

                obj
            }
            MessageContent::ToolCalls(calls) => {
                let tool_calls: Vec<serde_json::Value> = calls
                    .iter()
                    .map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments.to_string()
                            }
                        })
                    })
                    .collect();

                serde_json::json!({
                    "role": "assistant",
                    "tool_calls": tool_calls
                })
            }
        }
    }

    /// Send a request with tools support.
    async fn send_request_with_tools(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<LLMToolResponse, LLMError> {
        // Build messages array
        let mut all_messages = Vec::new();

        // Add system message
        all_messages.push(serde_json::json!({
            "role": "system",
            "content": system
        }));

        // Add conversation messages
        for msg in messages {
            all_messages.push(self.message_to_openai(msg));
        }

        let request = ChatRequestWithTools {
            model: self.model.clone(),
            messages: all_messages,
            max_tokens: Some(self.max_tokens),
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.to_vec())
            },
            tool_choice: if tools.is_empty() {
                None
            } else {
                Some("auto".to_string())
            },
        };

        let url = format!("{}/chat/completions", self.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json");

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

        let response_text = response
            .text()
            .await
            .map_err(|e| LLMError::Network(format!("Failed to read response body: {}", e)))?;

        let chat_response: ChatResponseWithTools =
            serde_json::from_str(&response_text).map_err(|e| {
                LLMError::ParseError(format!(
                    "Failed to parse response: {}. Response: {}",
                    e,
                    &response_text[..response_text.len().min(500)]
                ))
            })?;

        // Extract from first choice
        let choice = chat_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LLMError::ParseError("No choices in response".to_string()))?;

        let content = choice.message.content.unwrap_or_default();
        let finish_reason = choice.finish_reason;

        // Parse tool calls if present
        let tool_calls: Vec<ToolCall> = choice
            .message
            .tool_calls
            .map(|calls| {
                calls
                    .into_iter()
                    .filter_map(|tc| {
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments).ok()?;
                        Some(ToolCall::new(tc.id, tc.function.name, args))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let is_final = tool_calls.is_empty()
            && finish_reason.as_deref() != Some("tool_calls")
            && finish_reason.as_deref() != Some("function_call");

        Ok(LLMToolResponse {
            content,
            tool_calls,
            is_final,
            finish_reason,
        })
    }
}

#[async_trait]
impl LLMWithTools for OpenAIClient {
    async fn complete_with_tools(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<LLMToolResponse, LLMError> {
        self.send_request_with_tools(system, messages, tools).await
    }

    async fn stream_complete_with_tools(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[serde_json::Value],
        chunk_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<LLMToolResponse, LLMError> {
        // Build messages array
        let mut all_messages = Vec::new();

        // Add system message
        all_messages.push(serde_json::json!({
            "role": "system",
            "content": system
        }));

        // Add conversation messages
        for msg in messages {
            all_messages.push(self.message_to_openai(msg));
        }

        let request = serde_json::json!({
            "model": self.model,
            "messages": all_messages,
            "max_tokens": self.max_tokens,
            "stream": true,
            "tools": if tools.is_empty() { serde_json::Value::Null } else { serde_json::json!(tools) },
            "tool_choice": if tools.is_empty() { serde_json::Value::Null } else { serde_json::json!("auto") }
        });

        let url = format!("{}/chat/completions", self.base_url);

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json");

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
        let mut accumulated_content = String::new();
        let mut tool_calls: Vec<StreamingToolCall> = Vec::new();
        let mut finish_reason: Option<String> = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LLMError::Network(e.to_string()))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            // Process complete SSE lines from buffer
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                // Parse SSE data line
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    // Parse the streaming response
                    if let Ok(parsed) = serde_json::from_str::<StreamingResponse>(data) {
                        if let Some(choice) = parsed.choices.into_iter().next() {
                            // Handle content delta
                            if let Some(content) = choice.delta.content {
                                accumulated_content.push_str(&content);
                                let _ = chunk_tx.send(StreamChunk::text(content));
                            }

                            // Handle tool call deltas
                            if let Some(tc_deltas) = choice.delta.tool_calls {
                                for tc_delta in tc_deltas {
                                    let idx = tc_delta.index;

                                    // Ensure we have enough entries
                                    while tool_calls.len() <= idx {
                                        tool_calls.push(StreamingToolCall::default());
                                    }

                                    // Update tool call
                                    if let Some(id) = tc_delta.id {
                                        tool_calls[idx].id = id;
                                    }
                                    if let Some(func) = tc_delta.function {
                                        if let Some(name) = func.name {
                                            tool_calls[idx].name = name;
                                        }
                                        if let Some(args) = func.arguments {
                                            tool_calls[idx].arguments.push_str(&args);
                                        }
                                    }
                                }
                            }

                            // Capture finish reason
                            if let Some(reason) = choice.finish_reason {
                                finish_reason = Some(reason);
                            }
                        }
                    }
                }
            }
        }

        // Send done marker
        let _ = chunk_tx.send(StreamChunk::done());

        // Convert accumulated tool calls to final format
        let final_tool_calls: Vec<ToolCall> = tool_calls
            .into_iter()
            .filter(|tc| !tc.id.is_empty())
            .filter_map(|tc| {
                let args: serde_json::Value = serde_json::from_str(&tc.arguments).ok()?;
                Some(ToolCall::new(tc.id, tc.name, args))
            })
            .collect();

        let is_final = final_tool_calls.is_empty()
            && finish_reason.as_deref() != Some("tool_calls")
            && finish_reason.as_deref() != Some("function_call");

        Ok(LLMToolResponse {
            content: accumulated_content,
            tool_calls: final_tool_calls,
            is_final,
            finish_reason,
        })
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}

/// Streaming response delta structure.
#[derive(Debug, Deserialize)]
struct StreamingResponse {
    choices: Vec<StreamingChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamingChoice {
    delta: StreamingDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamingDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<StreamingToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct StreamingToolCallDelta {
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<StreamingFunctionDelta>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamingFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

/// Accumulator for streaming tool calls.
#[derive(Debug, Default)]
struct StreamingToolCall {
    id: String,
    name: String,
    arguments: String,
}

//! Tool calling support for LLMs.
//!
//! Provides types and traits for LLM function/tool calling capabilities.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::{LLMError, StreamChunk};

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender.
    pub role: MessageRole,
    /// Content of the message.
    pub content: MessageContent,
    /// Tool call ID (for tool result messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Name (for tool messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            tool_call_id: None,
            name: None,
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            tool_call_id: None,
            name: None,
        }
    }

    /// Create an assistant message with tool calls.
    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::ToolCalls(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    /// Create a tool result message.
    pub fn tool_result(tool_call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::Text(output.into()),
            tool_call_id: Some(tool_call_id.into()),
            name: None,
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (instructions).
    System,
    /// User message.
    User,
    /// Assistant (LLM) message.
    Assistant,
    /// Tool result message.
    Tool,
}

/// Content of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text content.
    Text(String),
    /// Tool calls from assistant.
    ToolCalls(Vec<ToolCall>),
}

impl MessageContent {
    /// Get text content if this is a text message.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(s) => Some(s),
            MessageContent::ToolCalls(_) => None,
        }
    }

    /// Get tool calls if this is a tool calls message.
    pub fn as_tool_calls(&self) -> Option<&[ToolCall]> {
        match self {
            MessageContent::Text(_) => None,
            MessageContent::ToolCalls(calls) => Some(calls),
        }
    }
}

/// A tool call request from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this call (used to match with results).
    pub id: String,
    /// Name of the tool to call.
    pub name: String,
    /// Arguments as JSON.
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }
}

/// Response from an LLM that may include tool calls.
#[derive(Debug, Clone)]
pub struct LLMToolResponse {
    /// Text content (may be empty if only tool calls).
    pub content: String,
    /// Tool calls requested by LLM.
    pub tool_calls: Vec<ToolCall>,
    /// Whether this is a final response (no more tool calls needed).
    pub is_final: bool,
    /// Finish reason from the API.
    pub finish_reason: Option<String>,
}

impl LLMToolResponse {
    /// Create a text-only response.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_calls: Vec::new(),
            is_final: true,
            finish_reason: Some("stop".to_string()),
        }
    }

    /// Create a response with tool calls.
    pub fn with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content: String::new(),
            tool_calls,
            is_final: false,
            finish_reason: Some("tool_calls".to_string()),
        }
    }

    /// Check if the response has tool calls.
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Extended LLM trait with tool calling support.
#[async_trait]
pub trait LLMWithTools: Send + Sync {
    /// Complete with tools available.
    ///
    /// # Arguments
    /// * `system` - System prompt
    /// * `messages` - Conversation history
    /// * `tools` - Available tools in provider format (OpenAI or Anthropic)
    ///
    /// # Returns
    /// Response that may contain text and/or tool calls.
    async fn complete_with_tools(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<LLMToolResponse, LLMError>;

    /// Check if this LLM supports tool calling.
    fn supports_tools(&self) -> bool {
        true
    }

    /// Complete with tools and stream text chunks as they arrive.
    ///
    /// This method streams text content as it's generated while still
    /// properly handling tool calls. The complete response is returned
    /// at the end, while text chunks are sent through the channel.
    ///
    /// # Arguments
    /// * `system` - System prompt
    /// * `messages` - Conversation history
    /// * `tools` - Available tools in provider format
    /// * `chunk_tx` - Channel to send text chunks as they arrive
    ///
    /// # Returns
    /// Complete response with all tool calls after streaming finishes.
    async fn stream_complete_with_tools(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[serde_json::Value],
        chunk_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<LLMToolResponse, LLMError> {
        // Default implementation: fall back to non-streaming
        let response = self.complete_with_tools(system, messages, tools).await?;

        // Send the complete text as a single chunk
        if !response.content.is_empty() {
            let _ = chunk_tx.send(StreamChunk::text(response.content.clone()));
        }
        let _ = chunk_tx.send(StreamChunk::done());

        Ok(response)
    }

    /// Check if this LLM supports streaming with tools.
    fn supports_streaming(&self) -> bool {
        false
    }
}

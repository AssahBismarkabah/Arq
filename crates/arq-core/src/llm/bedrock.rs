//! AWS Bedrock client for Claude models.
//!
//! Uses the Bedrock Converse API for unified tool calling support.

use async_trait::async_trait;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, InferenceConfiguration, Message as BedrockMessage, StopReason,
    SystemContentBlock, Tool, ToolConfiguration, ToolInputSchema, ToolResultBlock,
    ToolResultContentBlock, ToolResultStatus, ToolSpecification, ToolUseBlock,
};
use aws_sdk_bedrockruntime::Client;
use tokio::sync::mpsc;

use super::{
    tool_call::{LLMToolResponse, LLMWithTools, Message, MessageContent, MessageRole, ToolCall},
    LLMError, StreamChunk, LLM,
};
use crate::config::DEFAULT_MAX_TOKENS;

/// Default Bedrock model (Claude 3.5 Sonnet v2).
pub const DEFAULT_BEDROCK_MODEL: &str = "anthropic.claude-3-5-sonnet-20241022-v2:0";

/// AWS Bedrock client for Claude models.
pub struct BedrockClient {
    client: Client,
    model_id: String,
    max_tokens: i32,
}

impl BedrockClient {
    /// Creates a new Bedrock client.
    ///
    /// Uses the standard AWS credential chain:
    /// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// 2. AWS credentials file (~/.aws/credentials)
    /// 3. AWS config file (~/.aws/config)
    /// 4. IAM instance roles (EC2, ECS, Lambda)
    pub async fn new(region: Option<String>, model_id: String) -> Result<Self, LLMError> {
        // Load AWS configuration
        let config = if let Some(ref r) = region {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(r.clone()))
                .load()
                .await
        } else {
            aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
        };

        // Create the Bedrock client
        let client = Client::new(&config);

        Ok(Self {
            client,
            model_id,
            max_tokens: DEFAULT_MAX_TOKENS as i32,
        })
    }

    /// Creates a Bedrock client from environment variables.
    pub async fn from_env() -> Result<Self, LLMError> {
        let model_id = std::env::var("AWS_BEDROCK_MODEL")
            .unwrap_or_else(|_| DEFAULT_BEDROCK_MODEL.to_string());
        Self::new(None, model_id).await
    }

    /// Sets the maximum tokens for responses.
    pub fn with_max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Sets the model ID.
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }

    /// Convert a serde_json::Value to AWS Document.
    fn json_to_document(value: &serde_json::Value) -> aws_smithy_types::Document {
        match value {
            serde_json::Value::Null => aws_smithy_types::Document::Null,
            serde_json::Value::Bool(b) => aws_smithy_types::Document::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    aws_smithy_types::Document::Number(aws_smithy_types::Number::NegInt(i))
                } else if let Some(u) = n.as_u64() {
                    aws_smithy_types::Document::Number(aws_smithy_types::Number::PosInt(u))
                } else if let Some(f) = n.as_f64() {
                    aws_smithy_types::Document::Number(aws_smithy_types::Number::Float(f))
                } else {
                    aws_smithy_types::Document::Null
                }
            }
            serde_json::Value::String(s) => aws_smithy_types::Document::String(s.clone()),
            serde_json::Value::Array(arr) => {
                aws_smithy_types::Document::Array(arr.iter().map(Self::json_to_document).collect())
            }
            serde_json::Value::Object(obj) => aws_smithy_types::Document::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), Self::json_to_document(v)))
                    .collect(),
            ),
        }
    }

    /// Convert AWS Document to serde_json::Value.
    fn document_to_json(doc: &aws_smithy_types::Document) -> serde_json::Value {
        match doc {
            aws_smithy_types::Document::Null => serde_json::Value::Null,
            aws_smithy_types::Document::Bool(b) => serde_json::Value::Bool(*b),
            aws_smithy_types::Document::Number(n) => match n {
                aws_smithy_types::Number::PosInt(i) => serde_json::json!(*i),
                aws_smithy_types::Number::NegInt(i) => serde_json::json!(*i),
                aws_smithy_types::Number::Float(f) => serde_json::json!(*f),
            },
            aws_smithy_types::Document::String(s) => serde_json::Value::String(s.clone()),
            aws_smithy_types::Document::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::document_to_json).collect())
            }
            aws_smithy_types::Document::Object(obj) => {
                let map: serde_json::Map<String, serde_json::Value> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::document_to_json(v)))
                    .collect();
                serde_json::Value::Object(map)
            }
        }
    }

    /// Convert our Message to Bedrock format.
    fn message_to_bedrock(&self, msg: &Message) -> Option<BedrockMessage> {
        match &msg.content {
            MessageContent::Text(text) => {
                let role = match msg.role {
                    MessageRole::User => ConversationRole::User,
                    MessageRole::Assistant => ConversationRole::Assistant,
                    MessageRole::Tool => {
                        // Tool results go to user role in Bedrock
                        ConversationRole::User
                    }
                    MessageRole::System => {
                        // System messages are handled separately
                        return None;
                    }
                };

                // For tool results, wrap in ToolResultBlock
                if msg.role == MessageRole::Tool {
                    if let Some(ref tool_call_id) = msg.tool_call_id {
                        let tool_result = ToolResultBlock::builder()
                            .tool_use_id(tool_call_id)
                            .status(ToolResultStatus::Success)
                            .content(ToolResultContentBlock::Text(text.clone()))
                            .build()
                            .ok()?;

                        return BedrockMessage::builder()
                            .role(role)
                            .content(ContentBlock::ToolResult(tool_result))
                            .build()
                            .ok();
                    }
                }

                BedrockMessage::builder()
                    .role(role)
                    .content(ContentBlock::Text(text.clone()))
                    .build()
                    .ok()
            }
            MessageContent::ToolCalls(calls) => {
                // Convert tool calls to Bedrock format
                let mut builder = BedrockMessage::builder().role(ConversationRole::Assistant);

                for tc in calls {
                    let doc = Self::json_to_document(&tc.arguments);
                    let tool_use = ToolUseBlock::builder()
                        .tool_use_id(&tc.id)
                        .name(&tc.name)
                        .input(doc)
                        .build()
                        .ok()?;

                    builder = builder.content(ContentBlock::ToolUse(tool_use));
                }

                builder.build().ok()
            }
        }
    }

    /// Convert OpenAI-format tools to Bedrock format.
    fn convert_tools_to_bedrock(&self, tools: &[serde_json::Value]) -> Vec<Tool> {
        tools
            .iter()
            .filter_map(|tool| {
                let function = tool.get("function")?;
                let name = function.get("name")?.as_str()?;
                let description = function.get("description")?.as_str().unwrap_or("");
                let parameters = function
                    .get("parameters")
                    .cloned()
                    .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));

                let doc = Self::json_to_document(&parameters);
                let input_schema = ToolInputSchema::Json(doc);

                let spec = ToolSpecification::builder()
                    .name(name)
                    .description(description)
                    .input_schema(input_schema)
                    .build()
                    .ok()?;

                Some(Tool::ToolSpec(spec))
            })
            .collect()
    }

    /// Extract content and tool calls from Bedrock response.
    fn extract_response(
        &self,
        output: &aws_sdk_bedrockruntime::types::ConverseOutput,
    ) -> (String, Vec<ToolCall>) {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        if let aws_sdk_bedrockruntime::types::ConverseOutput::Message(msg) = output {
            for block in msg.content() {
                match block {
                    ContentBlock::Text(text) => {
                        content.push_str(text);
                    }
                    ContentBlock::ToolUse(tool_use) => {
                        let args = Self::document_to_json(tool_use.input());

                        tool_calls.push(ToolCall::new(
                            tool_use.tool_use_id(),
                            tool_use.name(),
                            args,
                        ));
                    }
                    _ => {}
                }
            }
        }

        (content, tool_calls)
    }
}

#[async_trait]
impl LLM for BedrockClient {
    async fn complete(&self, prompt: &str) -> Result<String, LLMError> {
        self.complete_with_system("You are a helpful assistant.", prompt)
            .await
    }

    async fn complete_with_system(&self, system: &str, prompt: &str) -> Result<String, LLMError> {
        let message = BedrockMessage::builder()
            .role(ConversationRole::User)
            .content(ContentBlock::Text(prompt.to_string()))
            .build()
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;

        let inference_config = InferenceConfiguration::builder()
            .max_tokens(self.max_tokens)
            .build();

        let result = self
            .client
            .converse()
            .model_id(&self.model_id)
            .messages(message)
            .system(SystemContentBlock::Text(system.to_string()))
            .inference_config(inference_config)
            .send()
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("AccessDenied") || err_msg.contains("credentials") {
                    LLMError::MissingApiKey
                } else if err_msg.contains("ThrottlingException") {
                    LLMError::RateLimited
                } else {
                    LLMError::RequestFailed(err_msg)
                }
            })?;

        let output = result
            .output()
            .ok_or_else(|| LLMError::ParseError("No output in response".to_string()))?;

        let (content, _) = self.extract_response(output);

        Ok(content)
    }

    async fn stream_complete(
        &self,
        system: &str,
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), LLMError> {
        let message = BedrockMessage::builder()
            .role(ConversationRole::User)
            .content(ContentBlock::Text(prompt.to_string()))
            .build()
            .map_err(|e| LLMError::RequestFailed(e.to_string()))?;

        let inference_config = InferenceConfiguration::builder()
            .max_tokens(self.max_tokens)
            .build();

        let mut stream = self
            .client
            .converse_stream()
            .model_id(&self.model_id)
            .messages(message)
            .system(SystemContentBlock::Text(system.to_string()))
            .inference_config(inference_config)
            .send()
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("AccessDenied") || err_msg.contains("credentials") {
                    LLMError::MissingApiKey
                } else if err_msg.contains("ThrottlingException") {
                    LLMError::RateLimited
                } else {
                    LLMError::RequestFailed(err_msg)
                }
            })?;

        while let Some(event) = stream
            .stream
            .recv()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?
        {
            use aws_sdk_bedrockruntime::types::ConverseStreamOutput;
            if let ConverseStreamOutput::ContentBlockDelta(delta) = event {
                if let Some(aws_sdk_bedrockruntime::types::ContentBlockDelta::Text(text)) =
                    delta.delta()
                {
                    let _ = tx.send(StreamChunk::text(text));
                }
            }
        }

        let _ = tx.send(StreamChunk::done());
        Ok(())
    }

    fn supports_streaming(&self) -> bool {
        // Disable streaming for now due to AWS SDK builder issues
        // The non-streaming converse API works reliably
        false
    }
}

#[async_trait]
impl LLMWithTools for BedrockClient {
    async fn complete_with_tools(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<LLMToolResponse, LLMError> {
        // Convert messages to Bedrock format
        let bedrock_messages: Vec<BedrockMessage> = messages
            .iter()
            .filter_map(|m| self.message_to_bedrock(m))
            .collect();

        // Bedrock requires at least one message
        if bedrock_messages.is_empty() {
            return Err(LLMError::RequestFailed(
                "No valid messages to send to Bedrock".to_string(),
            ));
        }

        let inference_config = InferenceConfiguration::builder()
            .max_tokens(self.max_tokens)
            .build();

        let mut request = self
            .client
            .converse()
            .model_id(&self.model_id)
            .system(SystemContentBlock::Text(system.to_string()))
            .inference_config(inference_config);

        // Add messages
        for msg in bedrock_messages {
            request = request.messages(msg);
        }

        // Add tools if provided
        if !tools.is_empty() {
            let bedrock_tools = self.convert_tools_to_bedrock(tools);
            if !bedrock_tools.is_empty() {
                let tool_config = ToolConfiguration::builder()
                    .set_tools(Some(bedrock_tools))
                    .build()
                    .map_err(|e| LLMError::RequestFailed(format!("Tool config error: {}", e)))?;
                request = request.tool_config(tool_config);
            }
        }

        let result = request.send().await.map_err(|e| {
            let err_msg = format!("{:?}", e);
            if err_msg.contains("AccessDenied") || err_msg.contains("credentials") {
                LLMError::MissingApiKey
            } else if err_msg.contains("ThrottlingException") {
                LLMError::RateLimited
            } else {
                LLMError::RequestFailed(format!("Bedrock API error: {}", err_msg))
            }
        })?;

        let output = result
            .output()
            .ok_or_else(|| LLMError::ParseError("No output in response".to_string()))?;

        let (content, tool_calls) = self.extract_response(output);

        let stop_reason = result.stop_reason();
        let is_tool_use = *stop_reason == StopReason::ToolUse;
        let is_final = tool_calls.is_empty() && !is_tool_use;

        Ok(LLMToolResponse {
            content,
            tool_calls,
            is_final,
            finish_reason: Some(format!("{:?}", stop_reason)),
        })
    }

    async fn stream_complete_with_tools(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[serde_json::Value],
        chunk_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<LLMToolResponse, LLMError> {
        // Convert messages to Bedrock format
        let bedrock_messages: Vec<BedrockMessage> = messages
            .iter()
            .filter_map(|m| self.message_to_bedrock(m))
            .collect();

        let inference_config = InferenceConfiguration::builder()
            .max_tokens(self.max_tokens)
            .build();

        let mut request = self
            .client
            .converse_stream()
            .model_id(&self.model_id)
            .system(SystemContentBlock::Text(system.to_string()))
            .inference_config(inference_config);

        // Add messages
        for msg in bedrock_messages {
            request = request.messages(msg);
        }

        // Add tools if provided
        if !tools.is_empty() {
            let bedrock_tools = self.convert_tools_to_bedrock(tools);
            if !bedrock_tools.is_empty() {
                let tool_config = ToolConfiguration::builder()
                    .set_tools(Some(bedrock_tools))
                    .build()
                    .map_err(|e| LLMError::RequestFailed(e.to_string()))?;
                request = request.tool_config(tool_config);
            }
        }

        let mut stream = request.send().await.map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("AccessDenied") || err_msg.contains("credentials") {
                LLMError::MissingApiKey
            } else if err_msg.contains("ThrottlingException") {
                LLMError::RateLimited
            } else {
                LLMError::RequestFailed(err_msg)
            }
        })?;

        let mut accumulated_content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_input = String::new();
        let mut stop_reason: Option<String> = None;

        while let Some(event) = stream
            .stream
            .recv()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?
        {
            use aws_sdk_bedrockruntime::types::ConverseStreamOutput;

            match event {
                ConverseStreamOutput::ContentBlockDelta(delta) => {
                    if let Some(d) = delta.delta() {
                        match d {
                            aws_sdk_bedrockruntime::types::ContentBlockDelta::Text(text) => {
                                accumulated_content.push_str(text);
                                let _ = chunk_tx.send(StreamChunk::text(text));
                            }
                            aws_sdk_bedrockruntime::types::ContentBlockDelta::ToolUse(
                                tool_delta,
                            ) => {
                                current_tool_input.push_str(tool_delta.input());
                            }
                            _ => {}
                        }
                    }
                }
                ConverseStreamOutput::ContentBlockStart(start) => {
                    if let Some(aws_sdk_bedrockruntime::types::ContentBlockStart::ToolUse(
                        tool_start,
                    )) = start.start()
                    {
                        current_tool_id = tool_start.tool_use_id().to_string();
                        current_tool_name = tool_start.name().to_string();
                        current_tool_input.clear();
                    }
                }
                ConverseStreamOutput::ContentBlockStop(_) => {
                    // If we were building a tool call, save it
                    if !current_tool_id.is_empty() {
                        let args: serde_json::Value = serde_json::from_str(&current_tool_input)
                            .unwrap_or(serde_json::json!({}));
                        tool_calls.push(ToolCall::new(
                            current_tool_id.clone(),
                            current_tool_name.clone(),
                            args,
                        ));
                        current_tool_id.clear();
                        current_tool_name.clear();
                        current_tool_input.clear();
                    }
                }
                ConverseStreamOutput::MessageStop(stop) => {
                    let r = stop.stop_reason();
                    stop_reason = Some(format!("{:?}", r));
                }
                _ => {}
            }
        }

        let _ = chunk_tx.send(StreamChunk::done());

        let is_final = tool_calls.is_empty() && stop_reason.as_deref() != Some("ToolUse");

        Ok(LLMToolResponse {
            content: accumulated_content,
            tool_calls,
            is_final,
            finish_reason: stop_reason,
        })
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        // Disable streaming for now due to AWS SDK builder issues
        // The non-streaming converse API works reliably
        false
    }
}

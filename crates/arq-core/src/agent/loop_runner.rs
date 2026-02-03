//! Agentic loop runner.
//!
//! Implements the main agent loop that takes a plan and executes it
//! by iteratively calling tools until the task is complete.

use std::path::PathBuf;
use std::time::Duration;

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::llm::{LLMError, LLMWithTools, Message, ToolCall};
use crate::planning::Plan;
use crate::storage::Storage;

use super::error::AgentError;
use super::tools::{rollback_all_files, ToolContext, ToolRegistry, ToolResult};

/// Configuration for the agentic loop.
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Maximum iterations before stopping.
    pub max_iterations: usize,
    /// Whether to auto-confirm tool calls (dangerous!).
    pub auto_confirm: bool,
    /// Dry run mode - don't actually modify files.
    pub dry_run: bool,
    /// Maximum retries for transient failures.
    pub max_retries: usize,
    /// Initial retry delay in milliseconds.
    pub retry_delay_ms: u64,
    /// Maximum context messages before truncation.
    pub max_context_messages: usize,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            auto_confirm: false,
            dry_run: false,
            max_retries: 3,
            retry_delay_ms: 1000,
            max_context_messages: 50,
        }
    }
}

/// State of the agentic loop.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentLoopState {
    /// Current iteration count.
    pub iteration: usize,
    /// Files modified during this session.
    pub modified_files: Vec<String>,
    /// Files backed up (original path -> backup path).
    pub backed_up_files: Vec<(String, String)>,
    /// Whether task is complete.
    pub is_complete: bool,
    /// Completion summary.
    pub summary: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Total retries used.
    pub total_retries: usize,
}

/// Progress events from the agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentLoopProgress {
    /// Starting the loop.
    Started { plan_summary: String },
    /// LLM is thinking.
    Thinking { iteration: usize },
    /// Retrying after failure.
    Retrying {
        attempt: usize,
        max_attempts: usize,
        reason: String,
    },
    /// LLM wants to call a tool.
    ToolRequested {
        tool_name: String,
        args_preview: String,
    },
    /// Awaiting user confirmation for tool.
    AwaitingConfirmation {
        tool_name: String,
        args: serde_json::Value,
        request_id: String,
    },
    /// Tool was executed.
    ToolExecuted {
        tool_name: String,
        success: bool,
        output_preview: String,
    },
    /// File was backed up before modification.
    FileBackedUp { original: String, backup: String },
    /// LLM provided text response (streaming chunk).
    TextChunk { content: String },
    /// LLM provided text response (complete).
    TextResponse { content: String },
    /// Task marked complete.
    Complete {
        summary: String,
        files_changed: Vec<String>,
    },
    /// Error occurred.
    Error { message: String },
    /// Context was truncated due to length.
    ContextTruncated { removed_messages: usize },
    /// Files were rolled back from backups.
    FilesRolledBack { count: usize, files: Vec<String> },
    /// Session was resumed from checkpoint.
    SessionResumed {
        iteration: usize,
        message_count: usize,
    },
    /// Checkpoint saved.
    CheckpointSaved { iteration: usize },
}

/// Confirmation response for a tool call.
#[derive(Debug, Clone)]
pub struct ToolConfirmation {
    /// Request ID (matches AwaitingConfirmation.request_id).
    pub request_id: String,
    /// Whether the user confirmed.
    pub confirmed: bool,
}

/// The main agentic loop runner.
pub struct AgentLoopRunner<S: Storage = crate::storage::FileStorage> {
    /// LLM client with tool support.
    llm: Box<dyn LLMWithTools>,
    /// Tool registry.
    tools: ToolRegistry,
    /// Tool execution context.
    context: ToolContext,
    /// Configuration.
    config: AgentLoopConfig,
    /// Storage for checkpointing (optional).
    storage: Option<S>,
    /// Task ID for checkpointing (required if storage is set).
    task_id: Option<String>,
}

impl<S: Storage> AgentLoopRunner<S> {
    /// Create a new agent loop runner.
    pub fn new(llm: Box<dyn LLMWithTools>, root: impl Into<PathBuf>) -> Self {
        Self {
            llm,
            tools: ToolRegistry::with_default_tools(),
            context: ToolContext::new(root),
            config: AgentLoopConfig::default(),
            storage: None,
            task_id: None,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        llm: Box<dyn LLMWithTools>,
        root: impl Into<PathBuf>,
        config: AgentLoopConfig,
    ) -> Self {
        let root = root.into();
        Self {
            llm,
            tools: ToolRegistry::with_default_tools(),
            context: ToolContext::new(&root).with_dry_run(config.dry_run),
            config,
            storage: None,
            task_id: None,
        }
    }

    /// Set storage for checkpointing.
    pub fn with_storage(mut self, storage: S, task_id: String) -> Self {
        self.storage = Some(storage);
        self.task_id = Some(task_id);
        self
    }

    /// Check if there's a resumable session.
    pub fn has_resumable_session(&self) -> bool {
        match (&self.storage, &self.task_id) {
            (Some(storage), Some(task_id)) => storage.has_agent_session(task_id).unwrap_or(false),
            _ => false,
        }
    }

    /// Get resumable session info (iteration count) if available.
    pub fn get_resumable_session_info(&self) -> Option<(usize, usize)> {
        match (&self.storage, &self.task_id) {
            (Some(storage), Some(task_id)) => storage
                .load_agent_session(task_id)
                .ok()
                .flatten()
                .map(|(state, messages)| (state.iteration, messages.len())),
            _ => None,
        }
    }

    /// Clear any saved session for a fresh start.
    pub fn clear_session(&self) -> Result<(), AgentError> {
        if let (Some(storage), Some(task_id)) = (&self.storage, &self.task_id) {
            storage
                .clear_agent_session(task_id)
                .map_err(|e| AgentError::ExecutionFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// Save a checkpoint of the current session state.
    fn save_checkpoint(
        &self,
        state: &AgentLoopState,
        messages: &[Message],
        progress_tx: &mpsc::UnboundedSender<AgentLoopProgress>,
    ) {
        if let (Some(storage), Some(task_id)) = (&self.storage, &self.task_id) {
            if storage.save_agent_session(task_id, state, messages).is_ok() {
                let _ = progress_tx.send(AgentLoopProgress::CheckpointSaved {
                    iteration: state.iteration,
                });
            }
        }
    }

    /// Run the agentic loop, resuming from checkpoint if available.
    ///
    /// If `resume` is true and a checkpoint exists, resumes from the saved state.
    /// Otherwise starts fresh.
    pub async fn run_or_resume(
        &mut self,
        plan: &Plan,
        resume: bool,
        progress_tx: mpsc::UnboundedSender<AgentLoopProgress>,
        confirmation_rx: mpsc::UnboundedReceiver<ToolConfirmation>,
    ) -> Result<AgentLoopState, AgentError> {
        // Try to resume if requested
        if resume {
            if let (Some(storage), Some(task_id)) = (&self.storage, &self.task_id) {
                if let Ok(Some((saved_state, saved_messages))) = storage.load_agent_session(task_id)
                {
                    let _ = progress_tx.send(AgentLoopProgress::SessionResumed {
                        iteration: saved_state.iteration,
                        message_count: saved_messages.len(),
                    });

                    return self
                        .run_from_state(
                            plan,
                            saved_state,
                            saved_messages,
                            progress_tx,
                            confirmation_rx,
                        )
                        .await;
                }
            }
        }

        // Start fresh
        self.run(plan, progress_tx, confirmation_rx).await
    }

    /// Run the agentic loop from a given state and messages.
    async fn run_from_state(
        &mut self,
        plan: &Plan,
        mut state: AgentLoopState,
        mut messages: Vec<Message>,
        progress_tx: mpsc::UnboundedSender<AgentLoopProgress>,
        mut confirmation_rx: mpsc::UnboundedReceiver<ToolConfirmation>,
    ) -> Result<AgentLoopState, AgentError> {
        // Build system prompt
        let system_prompt = self.build_system_prompt(plan);

        let _ = progress_tx.send(AgentLoopProgress::Started {
            plan_summary: format!(
                "Resumed at iteration {}: {} files to create, {} files to modify",
                state.iteration,
                plan.files_to_create.len(),
                plan.files_to_modify.len()
            ),
        });

        // Main loop - continues from where we left off
        while state.iteration < self.config.max_iterations && !state.is_complete {
            state.iteration += 1;

            let _ = progress_tx.send(AgentLoopProgress::Thinking {
                iteration: state.iteration,
            });

            // Manage context window - truncate if too many messages
            if messages.len() > self.config.max_context_messages {
                let to_remove = messages.len() - self.config.max_context_messages;
                if to_remove > 0 && messages.len() > 2 {
                    let remove_count = to_remove.min(messages.len() - 2);
                    messages.drain(1..1 + remove_count);
                    let _ = progress_tx.send(AgentLoopProgress::ContextTruncated {
                        removed_messages: remove_count,
                    });
                }
            }

            // Call LLM with retry logic
            let response = match self
                .call_llm_with_retry(&system_prompt, &messages, &mut state, &progress_tx)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    state.error = Some(e.to_string());
                    let _ = progress_tx.send(AgentLoopProgress::Error {
                        message: e.to_string(),
                    });
                    return Err(AgentError::LLMError(e));
                }
            };

            // Handle text content
            if !response.content.is_empty() {
                let _ = progress_tx.send(AgentLoopProgress::TextResponse {
                    content: response.content.clone(),
                });
                messages.push(Message::assistant(response.content));
            }

            // Handle tool calls
            let had_tool_calls = !response.tool_calls.is_empty();
            if had_tool_calls {
                messages.push(Message::assistant_tool_calls(response.tool_calls.clone()));

                // Separate read-only and write tools for parallel execution
                let (read_only_calls, write_calls): (Vec<_>, Vec<_>) = response
                    .tool_calls
                    .into_iter()
                    .partition(|tc| self.is_tool_read_only(&tc.name));

                // Execute read-only tools in parallel
                if !read_only_calls.is_empty() {
                    let progress_tx_clone = progress_tx.clone();
                    let read_futures = read_only_calls
                        .iter()
                        .map(|tc| self.execute_readonly_tool(tc, &progress_tx_clone));
                    let read_results = join_all(read_futures).await;

                    // Add results to messages in order
                    for (tool_call, result) in read_only_calls.iter().zip(read_results.into_iter())
                    {
                        messages.push(Message::tool_result(&tool_call.id, &result.output));

                        if tool_call.name == "task_complete" && result.success {
                            state.is_complete = true;
                            state.summary = Some(result.output.clone());
                        }
                    }

                    // Save checkpoint after parallel read-only tools execution
                    self.save_checkpoint(&state, &messages, &progress_tx);
                }

                // Execute write tools sequentially (need confirmation and backups)
                for tool_call in write_calls {
                    let result = self
                        .execute_tool_call(
                            &tool_call,
                            &progress_tx,
                            &mut confirmation_rx,
                            &mut state,
                        )
                        .await;

                    messages.push(Message::tool_result(&tool_call.id, &result.output));

                    for file in &result.modified_files {
                        if !state.modified_files.contains(file) {
                            state.modified_files.push(file.clone());
                        }
                    }

                    // Save checkpoint after tool execution
                    self.save_checkpoint(&state, &messages, &progress_tx);

                    if tool_call.name == "task_complete" && result.success {
                        state.is_complete = true;
                        state.summary = Some(result.output.clone());
                    }
                }
            }

            if response.is_final && !had_tool_calls && !state.is_complete {
                let nudge = self.build_nudge_message(&state, plan);
                messages.push(Message::user(nudge));
            }
        }

        // Final state update
        if state.is_complete {
            // Clear session on successful completion
            let _ = self.clear_session();
            let _ = progress_tx.send(AgentLoopProgress::Complete {
                summary: state.summary.clone().unwrap_or_default(),
                files_changed: state.modified_files.clone(),
            });
        } else if state.iteration >= self.config.max_iterations {
            let msg = format!(
                "Max iterations ({}) reached. {} files were modified.",
                self.config.max_iterations,
                state.modified_files.len()
            );
            state.error = Some(msg.clone());
            let _ = progress_tx.send(AgentLoopProgress::Error { message: msg });
        }

        Ok(state)
    }

    /// Run the agentic loop until completion or max iterations.
    ///
    /// # Arguments
    /// * `plan` - The approved plan to implement
    /// * `progress_tx` - Channel to send progress updates
    /// * `confirmation_rx` - Channel to receive tool confirmations
    pub async fn run(
        &mut self,
        plan: &Plan,
        progress_tx: mpsc::UnboundedSender<AgentLoopProgress>,
        mut confirmation_rx: mpsc::UnboundedReceiver<ToolConfirmation>,
    ) -> Result<AgentLoopState, AgentError> {
        let mut state = AgentLoopState::default();
        let mut messages: Vec<Message> = Vec::new();

        // Build system prompt
        let system_prompt = self.build_system_prompt(plan);

        // Add initial message with plan
        let initial_message = self.build_initial_message(plan);
        messages.push(Message::user(initial_message));

        let _ = progress_tx.send(AgentLoopProgress::Started {
            plan_summary: format!(
                "{} files to create, {} files to modify",
                plan.files_to_create.len(),
                plan.files_to_modify.len()
            ),
        });

        // Main loop
        while state.iteration < self.config.max_iterations && !state.is_complete {
            state.iteration += 1;

            let _ = progress_tx.send(AgentLoopProgress::Thinking {
                iteration: state.iteration,
            });

            // Manage context window - truncate if too many messages
            if messages.len() > self.config.max_context_messages {
                let to_remove = messages.len() - self.config.max_context_messages;
                // Keep the first message (initial plan) and remove old middle messages
                if to_remove > 0 && messages.len() > 2 {
                    let remove_count = to_remove.min(messages.len() - 2);
                    messages.drain(1..1 + remove_count);
                    let _ = progress_tx.send(AgentLoopProgress::ContextTruncated {
                        removed_messages: remove_count,
                    });
                }
            }

            // Call LLM with retry logic
            let response = match self
                .call_llm_with_retry(&system_prompt, &messages, &mut state, &progress_tx)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    state.error = Some(e.to_string());
                    let _ = progress_tx.send(AgentLoopProgress::Error {
                        message: e.to_string(),
                    });
                    return Err(AgentError::LLMError(e));
                }
            };

            // Handle text content
            if !response.content.is_empty() {
                let _ = progress_tx.send(AgentLoopProgress::TextResponse {
                    content: response.content.clone(),
                });

                messages.push(Message::assistant(response.content));
            }

            // Handle tool calls
            let had_tool_calls = !response.tool_calls.is_empty();
            if had_tool_calls {
                // Add assistant message with tool calls
                messages.push(Message::assistant_tool_calls(response.tool_calls.clone()));

                // Separate read-only and write tools for parallel execution
                let (read_only_calls, write_calls): (Vec<_>, Vec<_>) = response
                    .tool_calls
                    .into_iter()
                    .partition(|tc| self.is_tool_read_only(&tc.name));

                // Execute read-only tools in parallel
                if !read_only_calls.is_empty() {
                    let progress_tx_clone = progress_tx.clone();
                    let read_futures = read_only_calls
                        .iter()
                        .map(|tc| self.execute_readonly_tool(tc, &progress_tx_clone));
                    let read_results = join_all(read_futures).await;

                    // Add results to messages in order
                    for (tool_call, result) in read_only_calls.iter().zip(read_results.into_iter())
                    {
                        messages.push(Message::tool_result(&tool_call.id, &result.output));

                        if tool_call.name == "task_complete" && result.success {
                            state.is_complete = true;
                            state.summary = Some(result.output.clone());
                        }
                    }

                    // Save checkpoint after parallel read-only tools execution
                    self.save_checkpoint(&state, &messages, &progress_tx);
                }

                // Execute write tools sequentially (need confirmation and backups)
                for tool_call in write_calls {
                    let result = self
                        .execute_tool_call(
                            &tool_call,
                            &progress_tx,
                            &mut confirmation_rx,
                            &mut state,
                        )
                        .await;

                    // Add tool result to conversation
                    messages.push(Message::tool_result(&tool_call.id, &result.output));

                    // Track modified files
                    for file in &result.modified_files {
                        if !state.modified_files.contains(file) {
                            state.modified_files.push(file.clone());
                        }
                    }

                    // Save checkpoint after tool execution
                    self.save_checkpoint(&state, &messages, &progress_tx);

                    // Check for task completion
                    if tool_call.name == "task_complete" && result.success {
                        state.is_complete = true;
                        state.summary = Some(result.output.clone());
                    }
                }
            }

            // If no tool calls and response is final, provide guidance
            if response.is_final && !had_tool_calls && !state.is_complete {
                let nudge = self.build_nudge_message(&state, plan);
                messages.push(Message::user(nudge));
            }
        }

        // Final state update
        if state.is_complete {
            // Clear session on successful completion
            let _ = self.clear_session();
            let _ = progress_tx.send(AgentLoopProgress::Complete {
                summary: state.summary.clone().unwrap_or_default(),
                files_changed: state.modified_files.clone(),
            });
        } else if state.iteration >= self.config.max_iterations {
            let msg = format!(
                "Max iterations ({}) reached. {} files were modified.",
                self.config.max_iterations,
                state.modified_files.len()
            );
            state.error = Some(msg.clone());
            let _ = progress_tx.send(AgentLoopProgress::Error { message: msg });
        }

        Ok(state)
    }

    /// Call LLM with retry logic and exponential backoff.
    async fn call_llm_with_retry(
        &self,
        system_prompt: &str,
        messages: &[Message],
        state: &mut AgentLoopState,
        progress_tx: &mpsc::UnboundedSender<AgentLoopProgress>,
    ) -> Result<crate::llm::LLMToolResponse, LLMError> {
        let mut last_error = None;
        let mut delay = Duration::from_millis(self.config.retry_delay_ms);

        for attempt in 1..=self.config.max_retries + 1 {
            match self
                .llm
                .complete_with_tools(system_prompt, messages, &self.tools.to_openai_format())
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);

                    // Don't retry on the last attempt
                    if attempt <= self.config.max_retries {
                        state.total_retries += 1;

                        let reason = last_error
                            .as_ref()
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "Unknown error".to_string());

                        let _ = progress_tx.send(AgentLoopProgress::Retrying {
                            attempt,
                            max_attempts: self.config.max_retries + 1,
                            reason,
                        });

                        // Exponential backoff
                        sleep(delay).await;
                        delay = delay.saturating_mul(2);

                        // Cap at 30 seconds
                        if delay > Duration::from_secs(30) {
                            delay = Duration::from_secs(30);
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LLMError::RequestFailed("Max retries exceeded".into())))
    }

    /// Execute a single tool call with confirmation if needed.
    /// Execute a read-only tool without confirmation.
    /// This is used for parallel execution of read-only tools.
    async fn execute_readonly_tool(
        &self,
        tool_call: &ToolCall,
        progress_tx: &mpsc::UnboundedSender<AgentLoopProgress>,
    ) -> ToolResult {
        let _ = progress_tx.send(AgentLoopProgress::ToolRequested {
            tool_name: tool_call.name.clone(),
            args_preview: truncate_string(
                &serde_json::to_string_pretty(&tool_call.arguments).unwrap_or_default(),
                150,
            ),
        });

        // Get tool
        let tool = match self.tools.get(&tool_call.name) {
            Some(t) => t,
            None => {
                let error = format!(
                    "Unknown tool: '{}'. Available tools: {}",
                    tool_call.name,
                    self.tools
                        .list()
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let _ = progress_tx.send(AgentLoopProgress::ToolExecuted {
                    tool_name: tool_call.name.clone(),
                    success: false,
                    output_preview: truncate_string(&error, 200),
                });
                return ToolResult::failure(error);
            }
        };

        // Execute tool directly (no confirmation needed for read-only)
        let result = tool
            .execute(tool_call.arguments.clone(), &self.context)
            .await;

        let _ = progress_tx.send(AgentLoopProgress::ToolExecuted {
            tool_name: tool_call.name.clone(),
            success: result.success,
            output_preview: truncate_string(&result.output, 300),
        });

        result
    }

    /// Check if a tool is read-only.
    fn is_tool_read_only(&self, tool_name: &str) -> bool {
        self.tools
            .get(tool_name)
            .map(|t| t.is_read_only())
            .unwrap_or(false)
    }

    async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
        progress_tx: &mpsc::UnboundedSender<AgentLoopProgress>,
        confirmation_rx: &mut mpsc::UnboundedReceiver<ToolConfirmation>,
        state: &mut AgentLoopState,
    ) -> ToolResult {
        let _ = progress_tx.send(AgentLoopProgress::ToolRequested {
            tool_name: tool_call.name.clone(),
            args_preview: truncate_string(
                &serde_json::to_string_pretty(&tool_call.arguments).unwrap_or_default(),
                150,
            ),
        });

        // Get tool
        let tool = match self.tools.get(&tool_call.name) {
            Some(t) => t,
            None => {
                let error = format!(
                    "Unknown tool: '{}'. Available tools: {}",
                    tool_call.name,
                    self.tools
                        .list()
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let _ = progress_tx.send(AgentLoopProgress::ToolExecuted {
                    tool_name: tool_call.name.clone(),
                    success: false,
                    output_preview: truncate_string(&error, 200),
                });
                return ToolResult::failure(error);
            }
        };

        // Check if confirmation needed
        if tool.requires_confirmation() && !self.config.auto_confirm {
            let request_id = format!("{}_{}", tool_call.name, state.iteration);

            let _ = progress_tx.send(AgentLoopProgress::AwaitingConfirmation {
                tool_name: tool_call.name.clone(),
                args: tool_call.arguments.clone(),
                request_id: request_id.clone(),
            });

            // Wait for user confirmation with timeout
            let confirmed = loop {
                match confirmation_rx.recv().await {
                    Some(conf) if conf.request_id == request_id => break conf.confirmed,
                    Some(_) => continue, // Wrong request, keep waiting
                    None => break false, // Channel closed
                }
            };

            if !confirmed {
                let output = "User declined to execute this tool call. \
                             Please try a different approach or ask for clarification."
                    .to_string();
                let _ = progress_tx.send(AgentLoopProgress::ToolExecuted {
                    tool_name: tool_call.name.clone(),
                    success: false,
                    output_preview: output.clone(),
                });
                return ToolResult::failure(output);
            }
        }

        // Create backup for file-modifying tools
        if matches!(tool_call.name.as_str(), "write_file" | "edit_file") {
            if let Some(path) = tool_call.arguments.get("path").and_then(|v| v.as_str()) {
                if let Some(backup_path) = self.backup_file(path) {
                    state
                        .backed_up_files
                        .push((path.to_string(), backup_path.clone()));
                    let _ = progress_tx.send(AgentLoopProgress::FileBackedUp {
                        original: path.to_string(),
                        backup: backup_path,
                    });
                }
            }
        }

        // Execute tool
        let result = tool
            .execute(tool_call.arguments.clone(), &self.context)
            .await;

        let _ = progress_tx.send(AgentLoopProgress::ToolExecuted {
            tool_name: tool_call.name.clone(),
            success: result.success,
            output_preview: truncate_string(&result.output, 300),
        });

        result
    }

    /// Backup a file before modification.
    fn backup_file(&self, path: &str) -> Option<String> {
        let full_path = self.context.root.join(path);
        if !full_path.exists() {
            return None; // No backup needed for new files
        }

        // Store backups in .arq/backups/ (inside the gitignored .arq directory)
        let backup_dir = self.context.root.join(".arq").join("backups");
        if std::fs::create_dir_all(&backup_dir).is_err() {
            return None;
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let file_name = full_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let backup_name = format!("{}_{}", timestamp, file_name);
        let backup_path = backup_dir.join(&backup_name);

        match std::fs::copy(&full_path, &backup_path) {
            Ok(_) => Some(backup_path.to_string_lossy().to_string()),
            Err(_) => None,
        }
    }

    /// Rollback all changes made during the current session.
    /// Restores all backed up files to their original state.
    pub fn rollback_all_changes(
        &self,
        state: &AgentLoopState,
    ) -> Result<(usize, Vec<String>), AgentError> {
        if state.backed_up_files.is_empty() {
            return Ok((0, Vec::new()));
        }

        rollback_all_files(&self.context.root, &state.backed_up_files)
            .map_err(AgentError::ExecutionFailed)
    }

    /// Build a nudge message when the LLM seems stuck.
    fn build_nudge_message(&self, state: &AgentLoopState, plan: &Plan) -> String {
        let files_done = state.modified_files.len();
        let files_total = plan.files_to_create.len() + plan.files_to_modify.len();

        if files_done == 0 {
            "You haven't made any file changes yet. \
             Please start implementing the plan by reading existing files and making the necessary changes. \
             Use read_file to understand the codebase, then write_file or edit_file to make changes."
                .to_string()
        } else if files_done < files_total {
            format!(
                "You've modified {} out of {} files. \
                 Please continue with the remaining items in the plan. \
                 When all changes are complete and verified, call task_complete with a summary.",
                files_done, files_total
            )
        } else {
            "You've made changes to all planned files. \
             Please verify the changes work correctly by running appropriate commands \
             (e.g., cargo check, npm test). \
             If everything passes, call task_complete with a summary of what was done."
                .to_string()
        }
    }

    /// Build the system prompt for the agent.
    fn build_system_prompt(&self, plan: &Plan) -> String {
        let mut prompt = format!(
            r#"You are an expert software engineer implementing a code change. Your task is to implement the plan exactly as specified.

## Your Role

You are a meticulous, senior engineer who:
- Reads and understands code before modifying it
- Makes precise, minimal changes that accomplish the goal
- Verifies changes work before marking the task complete
- Asks for clarification when something is ambiguous

## Available Tools

You have access to these tools:

### Reading & Exploration
- `read_file`: Read file contents (use line_start/line_end for large files)
- `list_files`: List files matching a glob pattern
- `search_files`: Search for patterns in code (regex supported)

### Modification
- `write_file`: Create new files or completely overwrite existing files
- `edit_file`: Make targeted search/replace edits (preferred for existing files)

### Execution
- `run_command`: Execute shell commands (build, test, lint, etc.)

### Git Operations
- `git_status`: Show repository status (staged, unstaged, untracked files)
- `git_diff`: Show file differences (unstaged, staged, or against a commit)
- `git_log`: Show commit history
- `git_add`: Stage files for commit
- `git_commit`: Create a commit with staged changes

### Recovery
- `rollback_file`: Restore a file to its previous state from backup (if a change breaks something)

### Completion
- `task_complete`: Signal when you're done (REQUIRED at the end!)

## Workflow

1. **Explore First**: Read existing files to understand the codebase structure
2. **Plan Your Changes**: Think through what needs to change before editing
3. **Make Changes**: Use edit_file for existing files, write_file for new files
4. **Verify**: Run build/test commands to ensure changes work
5. **Fix Issues**: If tests fail, read the errors and fix them
6. **Complete**: Call task_complete with a summary when done

## Best Practices

- **Read before writing**: Always read a file before editing it
- **Use edit_file**: Prefer edit_file over write_file for existing files
- **Small edits**: Make focused, surgical changes
- **Verify incrementally**: Run checks after significant changes
- **Handle errors**: If a command fails, read the output and fix the issue

## Important Rules

- Implement EXACTLY what the plan specifies - no more, no less
- Do NOT add features, comments, or code not in the plan
- Do NOT add logging, error handling, or other "improvements" unless specified
- Always verify your changes compile/pass tests before completing
- If stuck or confused, explain the issue in your response

## Task Information

**Task Name**: {}
**Approach**: {}
**Complexity**: {}
"#,
            plan.task_name,
            plan.approach,
            plan.complexity.as_str()
        );

        // Add project memory if available
        if let Some(storage) = &self.storage {
            if let Ok(Some(memory)) = storage.load_project_memory() {
                prompt.push_str("\n## Project Context\n\n");
                prompt.push_str(
                    "The following is persistent project memory containing important context:\n\n",
                );
                prompt.push_str(&memory);
                prompt.push('\n');
            }
        }

        prompt
    }

    /// Build the initial user message with the plan details.
    fn build_initial_message(&self, plan: &Plan) -> String {
        let mut msg = String::from("Please implement the following plan:\n\n");

        if !plan.files_to_create.is_empty() {
            msg.push_str("## Files to Create\n\n");
            for file in &plan.files_to_create {
                msg.push_str(&format!("### {}\n", file.path));
                msg.push_str(&format!("{}\n\n", file.description));

                if !file.exports.is_empty() {
                    msg.push_str("**Exports:**\n");
                    for export in &file.exports {
                        msg.push_str(&format!("- `{}`: `{}`\n", export.name, export.signature));
                        if !export.behavior.is_empty() {
                            for behavior in &export.behavior {
                                msg.push_str(&format!("  - {}\n", behavior));
                            }
                        }
                    }
                    msg.push('\n');
                }
            }
        }

        if !plan.files_to_modify.is_empty() {
            msg.push_str("## Files to Modify\n\n");
            for file in &plan.files_to_modify {
                msg.push_str(&format!("### {}\n", file.path));
                msg.push_str(&format!("{}\n", file.description));

                if let Some(line) = file.line {
                    msg.push_str(&format!("Around line: {}\n", line));
                }

                if !file.additions.is_empty() {
                    msg.push_str("**Additions:**\n");
                    for addition in &file.additions {
                        msg.push_str(&format!("- {}\n", addition));
                    }
                }

                if !file.removals.is_empty() {
                    msg.push_str("**Removals:**\n");
                    for removal in &file.removals {
                        msg.push_str(&format!("- {}\n", removal));
                    }
                }
                msg.push('\n');
            }
        }

        if !plan.dependencies_to_add.is_empty() {
            msg.push_str("## Dependencies to Add\n\n");
            for dep in &plan.dependencies_to_add {
                msg.push_str(&format!("- {}\n", dep));
            }
            msg.push('\n');
        }

        msg.push_str(
            "\nStart by exploring the project structure and reading relevant files to understand the context. \
             Then proceed to implement each item in the plan.",
        );

        msg
    }
}

/// Truncate a string to max length with ellipsis.
fn truncate_string(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = AgentLoopConfig::default();
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.max_retries, 3);
        assert!(!config.auto_confirm);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_state_defaults() {
        let state = AgentLoopState::default();
        assert_eq!(state.iteration, 0);
        assert!(state.modified_files.is_empty());
        assert!(!state.is_complete);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 5), "hello...");
    }
}

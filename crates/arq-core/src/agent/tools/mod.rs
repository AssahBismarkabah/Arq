//! Tool system for the agentic loop.
//!
//! Provides a set of tools that the agent can use to interact with the project:
//! - read_file: Read file contents
//! - write_file: Create or overwrite files
//! - edit_file: Search/replace edits
//! - run_command: Execute shell commands
//! - list_files: List files matching patterns
//! - search_files: Search for patterns in files
//! - rollback_file: Restore files from backups
//! - task_complete: Signal task completion
//! - git_status: Show git repository status
//! - git_diff: Show file differences
//! - git_log: Show commit history
//! - git_add: Stage files for commit
//! - git_commit: Create commits

mod edit_file;
mod git_add;
mod git_commit;
mod git_diff;
mod git_log;
mod git_status;
mod list_files;
mod read_file;
mod registry;
mod rollback_file;
mod run_command;
mod search_files;
mod task_complete;
mod write_file;

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use edit_file::EditFileTool;
pub use git_add::GitAddTool;
pub use git_commit::GitCommitTool;
pub use git_diff::GitDiffTool;
pub use git_log::GitLogTool;
pub use git_status::GitStatusTool;
pub use list_files::ListFilesTool;
pub use read_file::ReadFileTool;
pub use registry::ToolRegistry;
pub use rollback_file::{rollback_all_files, RollbackFileTool};
pub use run_command::RunCommandTool;
pub use search_files::SearchFilesTool;
pub use task_complete::TaskCompleteTool;
pub use write_file::WriteFileTool;

/// Result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool execution succeeded.
    pub success: bool,
    /// Output/result from the tool.
    pub output: String,
    /// Error message if failed.
    pub error: Option<String>,
    /// Files that were modified (for tracking).
    pub modified_files: Vec<String>,
}

impl ToolResult {
    /// Create a successful result.
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            modified_files: Vec::new(),
        }
    }

    /// Create a failure result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            modified_files: Vec::new(),
        }
    }

    /// Add modified files to the result.
    pub fn with_modified_files(mut self, files: Vec<String>) -> Self {
        self.modified_files = files;
        self
    }
}

/// Definition of a tool for LLM consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (e.g., "read_file").
    pub name: String,
    /// Description for LLM.
    pub description: String,
    /// JSON schema for parameters.
    pub parameters: serde_json::Value,
}

/// Context provided to tools during execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Project root directory.
    pub root: PathBuf,
    /// Whether running in dry-run mode.
    pub dry_run: bool,
    /// Blocked command patterns (for sandboxing).
    pub blocked_commands: Vec<String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            dry_run: false,
            blocked_commands: vec![
                "rm -rf /".to_string(),
                "sudo".to_string(),
                "> /dev/".to_string(),
                "mkfs".to_string(),
                "dd if=".to_string(),
            ],
        }
    }
}

impl ToolContext {
    /// Create a new tool context with the given root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ..Default::default()
        }
    }

    /// Set dry run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Add blocked command patterns.
    pub fn with_blocked_commands(mut self, commands: Vec<String>) -> Self {
        self.blocked_commands.extend(commands);
        self
    }
}

/// A tool that can be called by the agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool definition for LLM.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with given arguments.
    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult;

    /// Whether this tool requires user confirmation before executing.
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// Whether this tool only reads data (no side effects).
    /// Used for parallel execution optimization.
    fn is_read_only(&self) -> bool {
        false
    }
}

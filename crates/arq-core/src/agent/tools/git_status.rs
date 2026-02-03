//! GitStatus tool - show git repository status.

use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for showing git repository status.
pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "git_status".to_string(),
            description: "Show git repository status including staged, unstaged, and untracked files. \
                         Use this to check the current state of the repository before making commits."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "short": {
                        "type": "boolean",
                        "description": "Use short format output (default: true)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let short = args.get("short").and_then(|v| v.as_bool()).unwrap_or(true);

        let mut cmd = Command::new("git");
        cmd.arg("status");

        if short {
            cmd.arg("--short");
        }

        cmd.current_dir(&context.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    let status_output = if stdout.trim().is_empty() {
                        "Working tree clean - no changes to commit".to_string()
                    } else {
                        stdout.to_string()
                    };
                    ToolResult::success(status_output)
                } else {
                    ToolResult::failure(format!("git status failed: {}", stderr))
                }
            }
            Err(e) => ToolResult::failure(format!("Failed to execute git: {}", e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        false // Reading status doesn't need confirmation
    }

    fn is_read_only(&self) -> bool {
        true // Only reads git status, no side effects
    }
}

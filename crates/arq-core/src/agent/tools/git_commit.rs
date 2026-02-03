//! GitCommit tool - create commits.

use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for creating git commits.
pub struct GitCommitTool;

#[async_trait]
impl Tool for GitCommitTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "git_commit".to_string(),
            description: "Create a git commit with staged changes. Always use git_add first to stage \
                         the files you want to include, then git_status to verify what will be committed."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Commit message describing the changes"
                    }
                },
                "required": ["message"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let message = match args.get("message").and_then(|v| v.as_str()) {
            Some(m) if !m.trim().is_empty() => m,
            _ => return ToolResult::failure("Missing required parameter: message"),
        };

        // First check if there are staged changes
        let status_cmd = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&context.root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match status_cmd {
            Ok(status) if status.success() => {
                // Exit code 0 means no staged changes
                return ToolResult::failure(
                    "No staged changes to commit. Use git_add first to stage files.",
                );
            }
            Err(e) => {
                return ToolResult::failure(format!("Failed to check git status: {}", e));
            }
            _ => {} // Exit code 1 means there are staged changes
        }

        let mut cmd = Command::new("git");
        cmd.args(["commit", "-m", message]);

        cmd.current_dir(&context.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    ToolResult::success(format!("Commit created:\n{}", stdout))
                } else {
                    ToolResult::failure(format!("git commit failed: {}{}", stderr, stdout))
                }
            }
            Err(e) => ToolResult::failure(format!("Failed to execute git: {}", e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // Creating commits needs confirmation
    }
}

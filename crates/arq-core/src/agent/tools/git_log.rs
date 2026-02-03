//! GitLog tool - show commit history.

use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for showing git commit history.
pub struct GitLogTool;

#[async_trait]
impl Tool for GitLogTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "git_log".to_string(),
            description: "Show commit history. Can limit the number of commits shown \
                         and optionally filter by file path."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "count": {
                        "type": "integer",
                        "description": "Number of commits to show (default: 10)"
                    },
                    "oneline": {
                        "type": "boolean",
                        "description": "Show one line per commit (default: true)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional: filter commits that modified this file/directory"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let oneline = args
            .get("oneline")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let path = args.get("path").and_then(|v| v.as_str());

        let mut cmd = Command::new("git");
        cmd.arg("log");
        cmd.arg(format!("-{}", count));

        if oneline {
            cmd.arg("--oneline");
        }

        if let Some(file_path) = path {
            cmd.arg("--");
            cmd.arg(file_path);
        }

        cmd.current_dir(&context.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match cmd.output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    let log_output = if stdout.trim().is_empty() {
                        "No commits found".to_string()
                    } else {
                        stdout.to_string()
                    };
                    ToolResult::success(log_output)
                } else {
                    ToolResult::failure(format!("git log failed: {}", stderr))
                }
            }
            Err(e) => ToolResult::failure(format!("Failed to execute git: {}", e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        false // Reading history doesn't need confirmation
    }

    fn is_read_only(&self) -> bool {
        true // Only reads history, no side effects
    }
}

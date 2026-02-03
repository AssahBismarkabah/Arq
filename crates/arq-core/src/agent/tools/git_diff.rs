//! GitDiff tool - show file differences.

use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for showing git diffs.
pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "git_diff".to_string(),
            description: "Show differences between file versions. Can show unstaged changes, \
                         staged changes (--cached), or compare against a specific commit."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional: specific file or directory to diff"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "Show staged changes (--cached). Default: false (shows unstaged)"
                    },
                    "commit": {
                        "type": "string",
                        "description": "Optional: compare against specific commit hash or ref"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let path = args.get("path").and_then(|v| v.as_str());
        let staged = args
            .get("staged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let commit = args.get("commit").and_then(|v| v.as_str());

        let mut cmd = Command::new("git");
        cmd.arg("diff");

        if staged {
            cmd.arg("--cached");
        }

        if let Some(commit_ref) = commit {
            cmd.arg(commit_ref);
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
                    let diff_output = if stdout.trim().is_empty() {
                        "No differences found".to_string()
                    } else {
                        stdout.to_string()
                    };
                    ToolResult::success(diff_output)
                } else {
                    ToolResult::failure(format!("git diff failed: {}", stderr))
                }
            }
            Err(e) => ToolResult::failure(format!("Failed to execute git: {}", e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        false // Reading diffs doesn't need confirmation
    }

    fn is_read_only(&self) -> bool {
        true // Only reads diffs, no side effects
    }
}

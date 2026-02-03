//! GitAdd tool - stage files for commit.

use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for staging files in git.
pub struct GitAddTool;

#[async_trait]
impl Tool for GitAddTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "git_add".to_string(),
            description:
                "Stage files for the next commit. Can stage specific files or all changes. \
                         Always use git_status first to see what changes are available."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of file paths to stage"
                    },
                    "all": {
                        "type": "boolean",
                        "description": "Stage all changes including untracked files (-A)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let paths = args
            .get("paths")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());
        let all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(false);

        // Validate: must have either paths or all flag
        if paths.as_ref().map(|p| p.is_empty()).unwrap_or(true) && !all {
            return ToolResult::failure(
                "Must specify either 'paths' array or 'all: true' to stage files",
            );
        }

        let mut cmd = Command::new("git");
        cmd.arg("add");

        if all {
            cmd.arg("-A");
        } else if let Some(file_paths) = &paths {
            for path in file_paths {
                cmd.arg(path);
            }
        }

        cmd.current_dir(&context.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match cmd.output().await {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    let message = if all {
                        "Staged all changes".to_string()
                    } else {
                        format!(
                            "Staged {} file(s): {}",
                            paths.as_ref().map(|p| p.len()).unwrap_or(0),
                            paths.as_ref().map(|p| p.join(", ")).unwrap_or_default()
                        )
                    };
                    ToolResult::success(message)
                } else {
                    ToolResult::failure(format!("git add failed: {}", stderr))
                }
            }
            Err(e) => ToolResult::failure(format!("Failed to execute git: {}", e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // Staging files modifies the index
    }
}

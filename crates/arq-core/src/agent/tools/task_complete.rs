//! TaskComplete tool - signals task completion.

use async_trait::async_trait;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for signaling task completion.
pub struct TaskCompleteTool;

#[async_trait]
impl Tool for TaskCompleteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "task_complete".to_string(),
            description: "Signal that the task is complete. Call this when you have \
                         finished implementing all items in the plan and verified the code works. \
                         Provide a summary of what was done and list the files that were changed."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "Summary of what was implemented and verified"
                    },
                    "files_changed": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of files that were created or modified"
                    }
                },
                "required": ["summary"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let summary = args
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("Task complete");

        let files: Vec<String> = args
            .get("files_changed")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let files_str = if files.is_empty() {
            String::from("(no files listed)")
        } else {
            files
                .iter()
                .map(|f| format!("  - {}", f))
                .collect::<Vec<_>>()
                .join("\n")
        };

        ToolResult {
            success: true,
            output: format!(
                "=== TASK COMPLETE ===\n\n{}\n\nFiles changed:\n{}",
                summary, files_str
            ),
            error: None,
            modified_files: files,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false // Completing task doesn't need confirmation
    }

    fn is_read_only(&self) -> bool {
        true // Only signals completion, no file system side effects
    }
}

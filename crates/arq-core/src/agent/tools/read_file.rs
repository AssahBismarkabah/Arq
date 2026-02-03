//! ReadFile tool - reads file contents.

use async_trait::async_trait;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for reading file contents.
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file. Use this to examine existing code, \
                         configuration files, or any text file in the project. \
                         You can optionally specify line ranges to read specific portions."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file relative to project root"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Optional: start reading from this line (1-indexed)"
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Optional: stop reading at this line (inclusive)"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure("Missing required parameter: path"),
        };

        let full_path = context.root.join(path);

        // Check if file exists
        if !full_path.exists() {
            return ToolResult::failure(format!("File not found: {}", path));
        }

        // Read file content
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => return ToolResult::failure(format!("Failed to read {}: {}", path, e)),
        };

        // Handle line range if specified
        let start_line = args
            .get("start_line")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize);
        let end_line = args
            .get("end_line")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize);

        let output = match (start_line, end_line) {
            (Some(start), Some(end)) => {
                // Validate range
                if start == 0 {
                    return ToolResult::failure("start_line must be >= 1");
                }
                if end < start {
                    return ToolResult::failure("end_line must be >= start_line");
                }

                content
                    .lines()
                    .enumerate()
                    .filter(|(i, _)| *i + 1 >= start && *i < end)
                    .map(|(i, line)| format!("{:4} | {}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            (Some(start), None) => {
                if start == 0 {
                    return ToolResult::failure("start_line must be >= 1");
                }

                content
                    .lines()
                    .enumerate()
                    .filter(|(i, _)| *i + 1 >= start)
                    .map(|(i, line)| format!("{:4} | {}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            (None, Some(end)) => content
                .lines()
                .enumerate()
                .filter(|(i, _)| *i < end)
                .map(|(i, line)| format!("{:4} | {}", i + 1, line))
                .collect::<Vec<_>>()
                .join("\n"),
            (None, None) => {
                // Return with line numbers
                content
                    .lines()
                    .enumerate()
                    .map(|(i, line)| format!("{:4} | {}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        };

        let line_count = content.lines().count();
        ToolResult::success(format!(
            "File: {} ({} lines)\n\n{}",
            path, line_count, output
        ))
    }

    fn requires_confirmation(&self) -> bool {
        false // Reading files doesn't need confirmation
    }

    fn is_read_only(&self) -> bool {
        true // Only reads files, no side effects
    }
}

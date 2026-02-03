//! WriteFile tool - creates or overwrites files.

use async_trait::async_trait;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for creating or overwriting files.
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Create a new file or completely overwrite an existing file. \
                         Use this for creating new files or when you need to replace \
                         the entire contents of a file. For partial edits, use edit_file instead."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file relative to project root"
                    },
                    "content": {
                        "type": "string",
                        "description": "Complete content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure("Missing required parameter: path"),
        };

        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::failure("Missing required parameter: content"),
        };

        let full_path = context.root.join(path);
        let is_new = !full_path.exists();

        if context.dry_run {
            return ToolResult::success(format!(
                "[DRY RUN] Would {} {} ({} bytes)",
                if is_new { "create" } else { "overwrite" },
                path,
                content.len()
            ))
            .with_modified_files(vec![path.to_string()]);
        }

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::failure(format!("Failed to create directories: {}", e));
            }
        }

        // Write the file
        match std::fs::write(&full_path, content) {
            Ok(_) => ToolResult::success(format!(
                "{} {} ({} bytes, {} lines)",
                if is_new { "Created" } else { "Overwrote" },
                path,
                content.len(),
                content.lines().count()
            ))
            .with_modified_files(vec![path.to_string()]),
            Err(e) => ToolResult::failure(format!("Failed to write {}: {}", path, e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // Writing files needs confirmation
    }
}

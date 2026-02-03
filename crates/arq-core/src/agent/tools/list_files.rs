//! ListFiles tool - lists files matching patterns.

use async_trait::async_trait;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for listing files matching glob patterns.
pub struct ListFilesTool;

/// Default maximum results.
const DEFAULT_MAX_RESULTS: usize = 100;

#[async_trait]
impl Tool for ListFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_files".to_string(),
            description: "List files in the project matching a glob pattern. \
                         Use this to explore project structure and find files. \
                         Examples: '*.rs', 'src/**/*.ts', '**/*.json'"
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., 'src/**/*.rs', '*.json')"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum files to return (default: 100)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::failure("Missing required parameter: pattern"),
        };

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|m| m as usize)
            .unwrap_or(DEFAULT_MAX_RESULTS);

        // Build full glob pattern
        let full_pattern = context.root.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        // Execute glob
        let paths = match glob::glob(&pattern_str) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(format!("Invalid glob pattern: {}", e)),
        };

        // Collect results
        let mut files: Vec<String> = Vec::new();
        let mut total_found = 0;

        for entry in paths {
            match entry {
                Ok(path) => {
                    total_found += 1;
                    if files.len() < max_results {
                        // Get relative path
                        if let Ok(rel_path) = path.strip_prefix(&context.root) {
                            let rel_str = rel_path.to_string_lossy().to_string();
                            // Add indicator for directories
                            if path.is_dir() {
                                files.push(format!("{}/ (dir)", rel_str));
                            } else {
                                files.push(rel_str);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Skip files we can't access
                }
            }
        }

        if files.is_empty() {
            ToolResult::success(format!("No files matching pattern: {}", pattern))
        } else {
            let truncated_msg = if total_found > max_results {
                format!(" (showing {} of {})", max_results, total_found)
            } else {
                String::new()
            };

            ToolResult::success(format!(
                "Found {} files{}:\n\n{}",
                files.len(),
                truncated_msg,
                files.join("\n")
            ))
        }
    }

    fn requires_confirmation(&self) -> bool {
        false // Listing files doesn't need confirmation
    }

    fn is_read_only(&self) -> bool {
        true // Only lists files, no side effects
    }
}

//! SearchFiles tool - searches for patterns in files.

use async_trait::async_trait;
use regex::Regex;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for searching patterns in files.
pub struct SearchFilesTool;

/// Default maximum results.
const DEFAULT_MAX_RESULTS: usize = 50;

#[async_trait]
impl Tool for SearchFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search_files".to_string(),
            description: "Search for text patterns in files (like grep). \
                         Returns matching lines with file paths and line numbers. \
                         Supports regular expressions."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Text or regex pattern to search for"
                    },
                    "file_pattern": {
                        "type": "string",
                        "description": "Glob pattern for files to search (default: '**/*')"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum matches to return (default: 50)"
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

        let file_pattern = args
            .get("file_pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("**/*");

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|m| m as usize)
            .unwrap_or(DEFAULT_MAX_RESULTS);

        // Compile regex
        let regex = match Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => return ToolResult::failure(format!("Invalid regex pattern: {}", e)),
        };

        // Build glob pattern
        let full_pattern = context.root.join(file_pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let paths = match glob::glob(&pattern_str) {
            Ok(p) => p,
            Err(e) => return ToolResult::failure(format!("Invalid file pattern: {}", e)),
        };

        // Search through files
        let mut matches: Vec<String> = Vec::new();
        let mut files_searched = 0;
        let mut files_with_matches = 0;

        for entry in paths {
            if matches.len() >= max_results {
                break;
            }

            let path: std::path::PathBuf = match entry {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Skip directories and non-text files
            if !path.is_file() {
                continue;
            }

            // Skip binary files (simple heuristic)
            if is_likely_binary(&path) {
                continue;
            }

            files_searched += 1;

            // Read and search file
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue, // Skip files we can't read
            };

            let rel_path = path
                .strip_prefix(&context.root)
                .map(|p: &std::path::Path| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());

            let mut file_has_match = false;

            for (line_num, line) in content.lines().enumerate() {
                if matches.len() >= max_results {
                    break;
                }

                if regex.is_match(line) {
                    file_has_match = true;
                    let trimmed = line.trim();
                    let display_line = if trimmed.len() > 150 {
                        format!("{}...", &trimmed[..150])
                    } else {
                        trimmed.to_string()
                    };
                    matches.push(format!("{}:{}: {}", rel_path, line_num + 1, display_line));
                }
            }

            if file_has_match {
                files_with_matches += 1;
            }
        }

        if matches.is_empty() {
            ToolResult::success(format!(
                "No matches for pattern '{}' in {} files searched",
                pattern, files_searched
            ))
        } else {
            let truncated_msg = if matches.len() >= max_results {
                " (truncated)"
            } else {
                ""
            };

            ToolResult::success(format!(
                "Found {} matches in {} files (searched {} files){}:\n\n{}",
                matches.len(),
                files_with_matches,
                files_searched,
                truncated_msg,
                matches.join("\n")
            ))
        }
    }

    fn requires_confirmation(&self) -> bool {
        false // Searching files doesn't need confirmation
    }

    fn is_read_only(&self) -> bool {
        true // Only searches files, no side effects
    }
}

/// Simple heuristic to detect binary files.
fn is_likely_binary(path: &std::path::Path) -> bool {
    // Check extension
    let binary_extensions = [
        "exe", "dll", "so", "dylib", "bin", "obj", "o", "a", "lib", "png", "jpg", "jpeg", "gif",
        "bmp", "ico", "pdf", "zip", "tar", "gz", "7z", "rar", "mp3", "mp4", "avi", "mov", "wav",
        "wasm", "ttf", "otf", "woff", "woff2", "eot",
    ];

    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        if binary_extensions.contains(&ext_lower.as_str()) {
            return true;
        }
    }

    // Check file size (skip very large files)
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() > 1_000_000 {
            // > 1MB
            return true;
        }
    }

    false
}

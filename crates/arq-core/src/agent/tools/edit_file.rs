//! EditFile tool - makes targeted edits to files.

use async_trait::async_trait;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for making search/replace edits to files.
pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Edit a file using search and replace. Supports single or multiple edits. \
                         The search text must match exactly (including whitespace and indentation). \
                         Use this for modifying existing files. For new files, use write_file."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit"
                    },
                    "search": {
                        "type": "string",
                        "description": "Exact text to find (for single edit)"
                    },
                    "replace": {
                        "type": "string",
                        "description": "Text to replace with (for single edit)"
                    },
                    "edits": {
                        "type": "array",
                        "description": "Array of edits for multiple changes: [{search: string, replace: string}, ...]",
                        "items": {
                            "type": "object",
                            "properties": {
                                "search": { "type": "string" },
                                "replace": { "type": "string" }
                            },
                            "required": ["search", "replace"]
                        }
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences instead of just the first (default: false)"
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

        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Collect edits - either from single search/replace or from edits array
        let edits = match collect_edits(&args) {
            Ok(e) => e,
            Err(err) => return err,
        };

        if edits.is_empty() {
            return ToolResult::failure(
                "No edits specified. Provide either 'search'/'replace' or 'edits' array.",
            );
        }

        let full_path = context.root.join(path);

        // Check if file exists
        if !full_path.exists() {
            return ToolResult::failure(format!(
                "File not found: {}. Use write_file to create new files.",
                path
            ));
        }

        // Read current content
        let mut content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => return ToolResult::failure(format!("Failed to read {}: {}", path, e)),
        };

        // Apply each edit
        let mut applied_edits = Vec::new();
        let mut failed_edits = Vec::new();
        let original_content = content.clone();

        for (idx, edit) in edits.iter().enumerate() {
            if !content.contains(&edit.search) {
                let hint = generate_hint(&content, &edit.search);
                failed_edits.push(format!("Edit {}: Search text not found{}", idx + 1, hint));
                continue;
            }

            let occurrence_count = content.matches(&edit.search).count();

            if replace_all {
                content = content.replace(&edit.search, &edit.replace);
                applied_edits.push(format!(
                    "Edit {}: Replaced {} occurrence(s)",
                    idx + 1,
                    occurrence_count
                ));
            } else {
                content = content.replacen(&edit.search, &edit.replace, 1);
                applied_edits.push(format!(
                    "Edit {}: Replaced 1 of {} occurrence(s)",
                    idx + 1,
                    occurrence_count
                ));
            }
        }

        // Check if any edits were applied
        if applied_edits.is_empty() {
            let error_msg = format!(
                "No edits could be applied to {}:\n{}",
                path,
                failed_edits.join("\n")
            );
            return ToolResult::failure(error_msg);
        }

        // Generate diff
        let diff = create_unified_diff(&original_content, &content, path);

        if context.dry_run {
            let mut result = format!("[DRY RUN] Would edit {}:\n", path);
            for edit in &applied_edits {
                result.push_str(&format!("  - {}\n", edit));
            }
            if !failed_edits.is_empty() {
                result.push_str("\nFailed edits:\n");
                for fail in &failed_edits {
                    result.push_str(&format!("  - {}\n", fail));
                }
            }
            result.push_str(&format!("\n{}", diff));
            return ToolResult::success(result).with_modified_files(vec![path.to_string()]);
        }

        // Write the file
        match std::fs::write(&full_path, &content) {
            Ok(_) => {
                let mut result = format!("Edited {}:\n", path);
                for edit in &applied_edits {
                    result.push_str(&format!("  - {}\n", edit));
                }
                if !failed_edits.is_empty() {
                    result.push_str("\nSome edits failed:\n");
                    for fail in &failed_edits {
                        result.push_str(&format!("  - {}\n", fail));
                    }
                }
                result.push_str(&format!("\n{}", diff));
                ToolResult::success(result).with_modified_files(vec![path.to_string()])
            }
            Err(e) => ToolResult::failure(format!("Failed to write {}: {}", path, e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // Editing files needs confirmation
    }
}

/// A single edit operation.
struct Edit {
    search: String,
    replace: String,
}

/// Collect edits from the arguments.
fn collect_edits(args: &serde_json::Value) -> Result<Vec<Edit>, ToolResult> {
    let mut edits = Vec::new();

    // Check for single edit (search/replace)
    if let (Some(search), Some(replace)) = (
        args.get("search").and_then(|v| v.as_str()),
        args.get("replace").and_then(|v| v.as_str()),
    ) {
        edits.push(Edit {
            search: search.to_string(),
            replace: replace.to_string(),
        });
    }

    // Check for multiple edits
    if let Some(edits_array) = args.get("edits").and_then(|v| v.as_array()) {
        for (idx, edit) in edits_array.iter().enumerate() {
            let search = edit.get("search").and_then(|v| v.as_str()).ok_or_else(|| {
                ToolResult::failure(format!("edits[{}]: missing 'search' field", idx))
            })?;
            let replace = edit
                .get("replace")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolResult::failure(format!("edits[{}]: missing 'replace' field", idx))
                })?;

            edits.push(Edit {
                search: search.to_string(),
                replace: replace.to_string(),
            });
        }
    }

    Ok(edits)
}

/// Generate a helpful hint when search text is not found.
fn generate_hint(content: &str, search: &str) -> String {
    // Check for common issues
    let trimmed_search = search.trim();

    // Check if trimmed version exists
    if content.contains(trimmed_search) && trimmed_search != search {
        return format!("\n  Hint: The trimmed text was found. Check leading/trailing whitespace.");
    }

    // Check for line ending issues
    let normalized_search = search.replace("\r\n", "\n");
    if content.contains(&normalized_search) && normalized_search != search {
        return format!("\n  Hint: Try using Unix-style line endings (\\n instead of \\r\\n).");
    }

    // Try to find similar text
    if let Some(similar) = find_similar_text(content, search) {
        let preview: String = similar.chars().take(100).collect();
        return format!(
            "\n  Hint: Similar text found:\n    {}{}",
            preview,
            if similar.len() > 100 { "..." } else { "" }
        );
    }

    String::new()
}

/// Try to find similar text in content (for helpful error messages).
fn find_similar_text(content: &str, search: &str) -> Option<String> {
    let search_lines: Vec<&str> = search.lines().collect();
    if search_lines.is_empty() {
        return None;
    }

    let first_line = search_lines[0].trim();
    if first_line.is_empty() || first_line.len() < 5 {
        return None;
    }

    // Look for lines containing similar content
    let content_lines: Vec<&str> = content.lines().collect();
    for (idx, line) in content_lines.iter().enumerate() {
        if line.contains(first_line) || similarity(line, first_line) > 0.6 {
            // Return a few lines of context
            let start = idx;
            let end = (idx + search_lines.len()).min(content_lines.len());
            return Some(content_lines[start..end].join("\n"));
        }
    }

    None
}

/// Simple similarity score between two strings (0.0 to 1.0).
fn similarity(a: &str, b: &str) -> f64 {
    let a = a.trim().to_lowercase();
    let b = b.trim().to_lowercase();

    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();

    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

/// Create a unified diff representation.
fn create_unified_diff(old: &str, new: &str, path: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut diff = format!("--- a/{}\n+++ b/{}\n", path, path);

    // Simple line-by-line diff
    let mut old_idx = 0;
    let mut new_idx = 0;
    let mut hunk_lines: Vec<String> = Vec::new();
    let mut hunk_start_old = 1;
    let mut hunk_start_new = 1;
    let mut in_hunk = false;
    let context_lines = 3;
    let mut context_buffer: Vec<String> = Vec::new();

    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        let old_line = old_lines.get(old_idx).copied();
        let new_line = new_lines.get(new_idx).copied();

        match (old_line, new_line) {
            (Some(o), Some(n)) if o == n => {
                // Lines match
                if in_hunk {
                    hunk_lines.push(format!(" {}", o));
                    context_buffer.clear();
                } else {
                    context_buffer.push(format!(" {}", o));
                    if context_buffer.len() > context_lines {
                        context_buffer.remove(0);
                        hunk_start_old += 1;
                        hunk_start_new += 1;
                    }
                }
                old_idx += 1;
                new_idx += 1;
            }
            (Some(o), Some(n)) => {
                // Lines differ
                if !in_hunk {
                    in_hunk = true;
                    hunk_lines.extend(context_buffer.drain(..));
                }
                hunk_lines.push(format!("-{}", o));
                hunk_lines.push(format!("+{}", n));
                old_idx += 1;
                new_idx += 1;
            }
            (Some(o), None) => {
                // Old line deleted
                if !in_hunk {
                    in_hunk = true;
                    hunk_lines.extend(context_buffer.drain(..));
                }
                hunk_lines.push(format!("-{}", o));
                old_idx += 1;
            }
            (None, Some(n)) => {
                // New line added
                if !in_hunk {
                    in_hunk = true;
                    hunk_lines.extend(context_buffer.drain(..));
                }
                hunk_lines.push(format!("+{}", n));
                new_idx += 1;
            }
            (None, None) => break,
        }
    }

    if !hunk_lines.is_empty() {
        let old_count = hunk_lines.iter().filter(|l| !l.starts_with('+')).count();
        let new_count = hunk_lines.iter().filter(|l| !l.starts_with('-')).count();
        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk_start_old, old_count, hunk_start_new, new_count
        ));
        for line in hunk_lines {
            diff.push_str(&line);
            diff.push('\n');
        }
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_single_edit() {
        let args = serde_json::json!({
            "path": "test.txt",
            "search": "foo",
            "replace": "bar"
        });
        let edits = collect_edits(&args).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].search, "foo");
        assert_eq!(edits[0].replace, "bar");
    }

    #[test]
    fn test_collect_multiple_edits() {
        let args = serde_json::json!({
            "path": "test.txt",
            "edits": [
                { "search": "foo", "replace": "bar" },
                { "search": "baz", "replace": "qux" }
            ]
        });
        let edits = collect_edits(&args).unwrap();
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_similarity() {
        assert!(similarity("hello world", "hello world") > 0.9);
        assert!(similarity("hello world", "hello there") > 0.3);
        assert!(similarity("abc", "xyz") < 0.1);
    }

    #[test]
    fn test_generate_hint_whitespace() {
        let content = "hello world";
        let search = "  hello world  ";
        let hint = generate_hint(content, search);
        assert!(hint.contains("whitespace"));
    }
}

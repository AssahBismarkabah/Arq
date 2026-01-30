//! Code extraction logic for structs, functions, and calls.

use regex::Regex;

use super::patterns::{
    CALL_KEYWORDS, CALL_PATTERN, FUNCTION_KEYWORDS, FUNCTION_PATTERNS, NON_FUNCTION_KEYWORDS,
    STRUCT_KEYWORDS, STRUCT_PATTERNS,
};
use crate::knowledge::models::{FunctionNode, StructNode};

/// Extract struct/class definitions from source code.
pub fn extract_structs(content: &str, file_path: &str) -> Vec<StructNode> {
    let mut structs = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for pattern in STRUCT_PATTERNS {
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };

        for cap in re.captures_iter(content) {
            let name = extract_identifier(&cap, STRUCT_KEYWORDS);
            if name.is_empty() {
                continue;
            }

            let match_start = cap.get(0).map(|m| m.start()).unwrap_or(0);
            let start_line = count_lines_before(content, match_start);
            let end_line = find_block_end(&lines, start_line.saturating_sub(1));
            let visibility = extract_visibility(&cap);

            structs.push(StructNode {
                id: None,
                name,
                file_path: file_path.to_string(),
                start_line: start_line as u32,
                end_line: end_line as u32,
                visibility,
                doc_comment: None,
            });
        }
    }

    structs
}

/// Extract function/method definitions from source code.
pub fn extract_functions(content: &str, file_path: &str) -> Vec<FunctionNode> {
    let mut functions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (pattern, is_simple) in FUNCTION_PATTERNS {
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };

        for cap in re.captures_iter(content) {
            let full_match = cap.get(0).map(|m| m.as_str()).unwrap_or("");

            let name = if *is_simple {
                extract_identifier(&cap, FUNCTION_KEYWORDS)
            } else {
                extract_last_identifier(&cap)
            };

            if name.is_empty() || NON_FUNCTION_KEYWORDS.contains(&name.as_str()) {
                continue;
            }

            let match_start = cap.get(0).map(|m| m.start()).unwrap_or(0);
            let start_line = count_lines_before(content, match_start);
            let end_line = find_block_end(&lines, start_line.saturating_sub(1));

            functions.push(FunctionNode {
                id: None,
                name,
                file_path: file_path.to_string(),
                parent_struct: None,
                start_line: start_line as u32,
                end_line: end_line as u32,
                visibility: extract_visibility(&cap),
                is_async: full_match.contains("async"),
                signature: full_match.trim().to_string(),
                doc_comment: None,
            });
        }
    }

    functions
}

/// Extract function calls from source code.
pub fn extract_calls(content: &str) -> Vec<String> {
    let mut calls = Vec::new();

    let re = match Regex::new(CALL_PATTERN) {
        Ok(r) => r,
        Err(_) => return calls,
    };

    for cap in re.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            let call_name = name.as_str();
            if !CALL_KEYWORDS.contains(&call_name) {
                calls.push(call_name.to_string());
            }
        }
    }

    calls
}

/// Extract content for a specific line range.
pub fn extract_line_range(content: &str, start: u32, end: u32) -> String {
    let start_idx = start.saturating_sub(1) as usize;
    let end_idx = end as usize;

    content
        .lines()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .collect::<Vec<_>>()
        .join("\n")
}

// --- Helper functions ---

/// Extract an identifier from regex captures, filtering out keywords.
fn extract_identifier(cap: &regex::Captures, keywords: &[&str]) -> String {
    cap.iter()
        .flatten()
        .filter(|m| {
            let s = m.as_str().trim();
            !s.is_empty()
                && s.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !keywords.contains(&s)
        })
        .last()
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

/// Extract the last word-like identifier from regex captures.
fn extract_last_identifier(cap: &regex::Captures) -> String {
    cap.iter()
        .flatten()
        .filter(|m| {
            let s = m.as_str().trim();
            !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
        })
        .last()
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

/// Extract visibility from regex captures.
fn extract_visibility(cap: &regex::Captures) -> String {
    let full = cap.get(0).map(|m| m.as_str()).unwrap_or("");
    if full.contains("pub") || full.contains("export") || full.contains("public") {
        "public".to_string()
    } else {
        "private".to_string()
    }
}

/// Count newlines before a byte offset to get line number.
fn count_lines_before(content: &str, offset: usize) -> usize {
    content[..offset].matches('\n').count() + 1
}

/// Find the end of a code block using brace counting and indentation.
fn find_block_end(lines: &[&str], start_idx: usize) -> usize {
    if start_idx >= lines.len() {
        return start_idx + 1;
    }

    let start_line = lines[start_idx];
    let base_indent = start_line.len() - start_line.trim_start().len();
    let mut brace_count = 0;
    let mut found_open = false;

    for (i, line) in lines.iter().enumerate().skip(start_idx) {
        for ch in line.chars() {
            if ch == '{' {
                brace_count += 1;
                found_open = true;
            }
            if ch == '}' {
                brace_count -= 1;
            }
        }

        // Python-style block end (indentation)
        if line.trim().ends_with(':') && !found_open {
            found_open = true;
        }

        if found_open && i > start_idx {
            let current_indent = line.len() - line.trim_start().len();

            // Check indentation-based end (Python)
            if !line.trim().is_empty() && current_indent <= base_indent && brace_count <= 0 {
                return i;
            }

            // Check brace-based end
            if brace_count <= 0 {
                return i + 1;
            }
        }
    }

    // Default: 50 lines or end of file
    (start_idx + 50).min(lines.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_struct() {
        let code = r#"
pub struct Config {
    name: String,
}
"#;
        let structs = extract_structs(code, "test.rs");
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Config");
        assert_eq!(structs[0].visibility, "public");
    }

    #[test]
    fn test_extract_rust_function() {
        let code = r#"
pub async fn process_data(input: &str) -> Result<()> {
    Ok(())
}
"#;
        let functions = extract_functions(code, "test.rs");
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "process_data");
        assert!(functions[0].is_async);
    }

    #[test]
    fn test_extract_calls() {
        let code = r#"
fn main() {
    let x = calculate(10);
    process(x);
    if condition() {
        return;
    }
}
"#;
        let calls = extract_calls(code);
        assert!(calls.contains(&"calculate".to_string()));
        assert!(calls.contains(&"process".to_string()));
        assert!(calls.contains(&"condition".to_string()));
        // "if" should be filtered out
        assert!(!calls.contains(&"if".to_string()));
    }
}

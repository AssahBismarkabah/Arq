//! Tests for agent tools.

use arq_core::agent::tools::{
    EditFileTool, ListFilesTool, ReadFileTool, SearchFilesTool, Tool, ToolContext, WriteFileTool,
};
use std::fs;
use tempfile::TempDir;

fn create_test_context() -> (ToolContext, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let context = ToolContext::new(temp_dir.path());
    (context, temp_dir)
}

// ============================================================================
// ReadFileTool Tests
// ============================================================================

#[tokio::test]
async fn test_read_file_basic() {
    let (context, temp_dir) = create_test_context();

    // Create test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "line 1\nline 2\nline 3").unwrap();

    let tool = ReadFileTool;
    let args = serde_json::json!({ "path": "test.txt" });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    assert!(result.output.contains("line 1"));
    assert!(result.output.contains("line 2"));
    assert!(result.output.contains("line 3"));
    assert!(result.output.contains("3 lines"));
}

#[tokio::test]
async fn test_read_file_with_line_range() {
    let (context, temp_dir) = create_test_context();

    // Create test file with 5 lines
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "line 1\nline 2\nline 3\nline 4\nline 5").unwrap();

    let tool = ReadFileTool;
    let args = serde_json::json!({
        "path": "test.txt",
        "start_line": 2,
        "end_line": 4
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    assert!(result.output.contains("line 2"));
    assert!(result.output.contains("line 3"));
    assert!(result.output.contains("line 4"));
    // Should not contain lines outside range
    assert!(!result.output.contains("| line 1\n"));
    assert!(!result.output.contains("| line 5"));
}

#[tokio::test]
async fn test_read_file_not_found() {
    let (context, _temp_dir) = create_test_context();

    let tool = ReadFileTool;
    let args = serde_json::json!({ "path": "nonexistent.txt" });

    let result = tool.execute(args, &context).await;

    assert!(!result.success);
    assert!(result.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_read_file_missing_path() {
    let (context, _temp_dir) = create_test_context();

    let tool = ReadFileTool;
    let args = serde_json::json!({});

    let result = tool.execute(args, &context).await;

    assert!(!result.success);
    assert!(result.error.unwrap().contains("Missing required parameter"));
}

// ============================================================================
// WriteFileTool Tests
// ============================================================================

#[tokio::test]
async fn test_write_file_basic() {
    let (context, temp_dir) = create_test_context();

    let tool = WriteFileTool;
    let args = serde_json::json!({
        "path": "new_file.txt",
        "content": "Hello, World!"
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    assert_eq!(result.modified_files, vec!["new_file.txt"]);

    // Verify file was created
    let content = fs::read_to_string(temp_dir.path().join("new_file.txt")).unwrap();
    assert_eq!(content, "Hello, World!");
}

#[tokio::test]
async fn test_write_file_creates_directories() {
    let (context, temp_dir) = create_test_context();

    let tool = WriteFileTool;
    let args = serde_json::json!({
        "path": "subdir/nested/file.txt",
        "content": "Nested content"
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success);

    // Verify directory structure was created
    let file_path = temp_dir.path().join("subdir/nested/file.txt");
    assert!(file_path.exists());
    assert_eq!(fs::read_to_string(file_path).unwrap(), "Nested content");
}

#[tokio::test]
async fn test_write_file_requires_confirmation() {
    let tool = WriteFileTool;
    assert!(tool.requires_confirmation());
}

// ============================================================================
// EditFileTool Tests
// ============================================================================

#[tokio::test]
async fn test_edit_file_search_replace() {
    let (context, temp_dir) = create_test_context();

    // Create initial file
    let test_file = temp_dir.path().join("edit_test.txt");
    fs::write(&test_file, "Hello, World!\nGoodbye, World!").unwrap();

    let tool = EditFileTool;
    let args = serde_json::json!({
        "path": "edit_test.txt",
        "search": "World",
        "replace": "Universe"
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success, "Edit failed: {:?}", result.error);

    // Verify first occurrence was replaced
    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("Hello, Universe!"));
}

#[tokio::test]
async fn test_edit_file_replace_all() {
    let (context, temp_dir) = create_test_context();

    // Create initial file
    let test_file = temp_dir.path().join("edit_test.txt");
    fs::write(&test_file, "foo bar foo baz foo").unwrap();

    let tool = EditFileTool;
    let args = serde_json::json!({
        "path": "edit_test.txt",
        "search": "foo",
        "replace": "qux",
        "replace_all": true
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success, "Edit failed: {:?}", result.error);

    // Verify all occurrences were replaced
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "qux bar qux baz qux");
}

#[tokio::test]
async fn test_edit_file_multiple_edits() {
    let (context, temp_dir) = create_test_context();

    // Create initial file
    let test_file = temp_dir.path().join("multi_edit.txt");
    fs::write(&test_file, "apple banana cherry").unwrap();

    let tool = EditFileTool;
    let args = serde_json::json!({
        "path": "multi_edit.txt",
        "edits": [
            { "search": "apple", "replace": "APPLE" },
            { "search": "banana", "replace": "BANANA" }
        ]
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success, "Edit failed: {:?}", result.error);

    let content = fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("APPLE"));
    assert!(content.contains("BANANA"));
}

#[tokio::test]
async fn test_edit_file_not_found() {
    let (context, _temp_dir) = create_test_context();

    let tool = EditFileTool;
    let args = serde_json::json!({
        "path": "nonexistent.txt",
        "search": "foo",
        "replace": "bar"
    });

    let result = tool.execute(args, &context).await;

    assert!(!result.success);
    assert!(result.error.unwrap().contains("not found"));
}

// ============================================================================
// ListFilesTool Tests
// ============================================================================

#[tokio::test]
async fn test_list_files_basic() {
    let (context, temp_dir) = create_test_context();

    // Create some files
    fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
    fs::create_dir(temp_dir.path().join("subdir")).unwrap();
    fs::write(temp_dir.path().join("subdir/file3.txt"), "").unwrap();

    let tool = ListFilesTool;
    let args = serde_json::json!({ "pattern": "**/*.txt" });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    assert!(result.output.contains("file1.txt"));
    assert!(result.output.contains("file2.txt"));
    assert!(result.output.contains("file3.txt"));
}

#[tokio::test]
async fn test_list_files_with_max_results() {
    let (context, temp_dir) = create_test_context();

    // Create many files
    for i in 0..10 {
        fs::write(temp_dir.path().join(format!("file{}.txt", i)), "").unwrap();
    }

    let tool = ListFilesTool;
    let args = serde_json::json!({
        "pattern": "*.txt",
        "max_results": 5
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    // Count the number of files listed (should be limited)
    let file_count = result.output.lines().filter(|l| l.contains(".txt")).count();
    assert!(file_count <= 5);
}

// ============================================================================
// SearchFilesTool Tests
// ============================================================================

#[tokio::test]
async fn test_search_files_basic() {
    let (context, temp_dir) = create_test_context();

    // Create files with searchable content
    fs::write(temp_dir.path().join("file1.txt"), "Hello World").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "Goodbye World").unwrap();
    fs::write(temp_dir.path().join("file3.txt"), "No match here").unwrap();

    let tool = SearchFilesTool;
    let args = serde_json::json!({ "pattern": "World" });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    assert!(result.output.contains("file1.txt"));
    assert!(result.output.contains("file2.txt"));
}

#[tokio::test]
async fn test_search_files_with_file_pattern() {
    let (context, temp_dir) = create_test_context();

    // Create files
    fs::write(temp_dir.path().join("code.rs"), "fn main() {}").unwrap();
    fs::write(temp_dir.path().join("code.txt"), "fn main() {}").unwrap();

    let tool = SearchFilesTool;
    let args = serde_json::json!({
        "pattern": "fn main",
        "file_pattern": "*.rs"
    });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    assert!(result.output.contains("code.rs"));
    // Should not include .txt file
    assert!(!result.output.contains("code.txt"));
}

#[tokio::test]
async fn test_search_files_no_matches() {
    let (context, temp_dir) = create_test_context();

    fs::write(temp_dir.path().join("file.txt"), "Hello World").unwrap();

    let tool = SearchFilesTool;
    let args = serde_json::json!({ "pattern": "xyz123notfound" });

    let result = tool.execute(args, &context).await;

    assert!(result.success);
    assert!(result.output.contains("No matches"));
}

// ============================================================================
// Tool Trait Tests
// ============================================================================

#[test]
fn test_tool_definitions_have_required_fields() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ReadFileTool),
        Box::new(WriteFileTool),
        Box::new(EditFileTool),
        Box::new(ListFilesTool),
        Box::new(SearchFilesTool),
    ];

    for tool in tools {
        let def = tool.definition();
        assert!(!def.name.is_empty(), "Tool name should not be empty");
        assert!(
            !def.description.is_empty(),
            "Tool description should not be empty"
        );
        assert!(
            def.parameters.is_object(),
            "Tool parameters should be a JSON object"
        );
    }
}

#[test]
fn test_confirmation_requirements() {
    // Tools that modify files should require confirmation
    assert!(WriteFileTool.requires_confirmation());
    assert!(EditFileTool.requires_confirmation());

    // Read-only tools should not require confirmation
    assert!(!ReadFileTool.requires_confirmation());
    assert!(!ListFilesTool.requires_confirmation());
    assert!(!SearchFilesTool.requires_confirmation());
}

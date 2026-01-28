use arq_core::ContextBuilder;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_gather_files() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a test file
    let mut file = File::create(root.join("test.rs")).unwrap();
    writeln!(file, "fn main() {{}}").unwrap();

    let builder = ContextBuilder::new(root);
    let context = builder.gather().unwrap();

    assert!(!context.files.is_empty());
    assert!(context.files.iter().any(|f| f.path == "test.rs"));
}

#[test]
fn test_excluded_directories() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create node_modules directory with a file
    fs::create_dir(root.join("node_modules")).unwrap();
    let mut file = File::create(root.join("node_modules/test.js")).unwrap();
    writeln!(file, "console.log('test')").unwrap();

    // Create a regular file
    let mut file = File::create(root.join("index.js")).unwrap();
    writeln!(file, "console.log('main')").unwrap();

    let builder = ContextBuilder::new(root);
    let context = builder.gather().unwrap();

    // Should include index.js but not node_modules/test.js
    assert!(context.files.iter().any(|f| f.path == "index.js"));
    assert!(!context.files.iter().any(|f| f.path.contains("node_modules")));
}

#[test]
fn test_custom_config() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create test files
    let mut file = File::create(root.join("test.rs")).unwrap();
    writeln!(file, "fn main() {{}}").unwrap();

    let mut file = File::create(root.join("test.custom")).unwrap();
    writeln!(file, "custom content").unwrap();

    // Default config should not include .custom files
    let builder = ContextBuilder::new(root);
    let context = builder.gather().unwrap();
    assert!(!context.files.iter().any(|f| f.path.ends_with(".custom")));

    // Custom config with .custom extension should include it
    let builder = ContextBuilder::new(root).include_extension("custom");
    let context = builder.gather().unwrap();
    assert!(context.files.iter().any(|f| f.path.ends_with(".custom")));
}

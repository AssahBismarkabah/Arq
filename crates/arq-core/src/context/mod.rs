use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use ignore::WalkBuilder;

use crate::config::ContextConfig;

/// Builds context from a codebase for LLM analysis.
pub struct ContextBuilder {
    root_path: PathBuf,
    config: ContextConfig,
}

impl ContextBuilder {
    /// Creates a new context builder rooted at the given path with default config.
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            config: ContextConfig::default(),
        }
    }

    /// Creates a new context builder with custom configuration.
    pub fn with_config(root_path: impl Into<PathBuf>, config: ContextConfig) -> Self {
        Self {
            root_path: root_path.into(),
            config,
        }
    }

    /// Sets the maximum file size.
    pub fn max_file_size(mut self, size: u64) -> Self {
        self.config.max_file_size = size;
        self
    }

    /// Sets the maximum total context size.
    pub fn max_total_size(mut self, size: u64) -> Self {
        self.config.max_total_size = size;
        self
    }

    /// Adds an extension to include.
    pub fn include_extension(mut self, ext: impl Into<String>) -> Self {
        self.config.include_extensions.push(ext.into());
        self
    }

    /// Adds a directory to exclude.
    pub fn exclude_dir(mut self, dir: impl Into<String>) -> Self {
        self.config.exclude_dirs.push(dir.into());
        self
    }

    /// Gathers context from the codebase.
    pub fn gather(&self) -> Result<Context, ContextError> {
        let structure = self.build_tree()?;
        let files = self.gather_files()?;

        Ok(Context { structure, files })
    }

    /// Builds a directory tree string.
    fn build_tree(&self) -> Result<String, ContextError> {
        let mut tree = String::new();
        self.build_tree_recursive(&self.root_path, "", &mut tree)?;
        Ok(tree)
    }

    fn build_tree_recursive(
        &self,
        path: &Path,
        prefix: &str,
        tree: &mut String,
    ) -> Result<(), ContextError> {
        let exclude_dirs = &self.config.exclude_dirs;

        let entries: Vec<_> = fs::read_dir(path)
            .map_err(|e| ContextError::IoError(path.to_path_buf(), e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                !name.starts_with('.') && !exclude_dirs.contains(&name)
            })
            .collect();

        let mut sorted_entries = entries;
        sorted_entries.sort_by_key(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (!is_dir, e.file_name())
        });

        for (i, entry) in sorted_entries.iter().enumerate() {
            let is_last = i == sorted_entries.len() - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let name = entry.file_name().to_string_lossy().to_string();

            tree.push_str(prefix);
            tree.push_str(connector);
            tree.push_str(&name);

            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                tree.push('/');
            }
            tree.push('\n');

            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };
                self.build_tree_recursive(&entry.path(), &new_prefix, tree)?;
            }
        }

        Ok(())
    }

    /// Gathers relevant files from the codebase.
    fn gather_files(&self) -> Result<Vec<FileContent>, ContextError> {
        let mut files = Vec::new();
        let mut total_size: u64 = 0;

        let walker = WalkBuilder::new(&self.root_path)
            .hidden(true)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Check extension
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            if !self.config.include_extensions.iter().any(|e| e == extension) {
                continue;
            }

            // Check if in excluded directory
            let path_str = path.to_string_lossy();
            if self.config.exclude_dirs.iter().any(|d| path_str.contains(d.as_str())) {
                continue;
            }

            // Check file size
            let metadata = fs::metadata(path)
                .map_err(|e| ContextError::IoError(path.to_path_buf(), e.to_string()))?;

            if metadata.len() > self.config.max_file_size {
                continue;
            }

            // Check total size limit
            if total_size + metadata.len() > self.config.max_total_size {
                break;
            }

            // Read file content
            let content = fs::read_to_string(path)
                .map_err(|e| ContextError::IoError(path.to_path_buf(), e.to_string()))?;

            let relative_path = path
                .strip_prefix(&self.root_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            total_size += metadata.len();

            files.push(FileContent {
                path: relative_path,
                content,
            });
        }

        Ok(files)
    }
}

/// Context gathered from a codebase.
#[derive(Debug, Clone)]
pub struct Context {
    /// Directory structure tree
    pub structure: String,
    /// File contents
    pub files: Vec<FileContent>,
}

impl Context {
    /// Formats the context for inclusion in a prompt.
    pub fn to_prompt_string(&self) -> String {
        let mut result = String::new();

        result.push_str("## Directory Structure\n\n```\n");
        result.push_str(&self.structure);
        result.push_str("```\n\n");

        result.push_str("## File Contents\n\n");

        for file in &self.files {
            result.push_str(&format!("### {}\n\n```\n", file.path));
            result.push_str(&file.content);
            result.push_str("\n```\n\n");
        }

        result
    }
}

/// Content of a single file.
#[derive(Debug, Clone)]
pub struct FileContent {
    /// Relative path from root
    pub path: String,
    /// File content
    pub content: String,
}

/// Errors that can occur during context building.
#[derive(Debug, Error)]
pub enum ContextError {
    #[error("IO error at {0}: {1}")]
    IoError(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

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
}

//! Change applier for agent phase.
//!
//! Applies generated code changes to the filesystem.
//! Supports backup and rollback capabilities.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::AgentError;
use super::types::FileOperation;

/// A single change to apply to the filesystem.
#[derive(Debug, Clone)]
pub struct PendingChange {
    /// Path to the file (relative to project root).
    pub path: String,
    /// Operation to perform.
    pub operation: ChangeOperation,
}

/// Operation type for a pending change.
#[derive(Debug, Clone)]
pub enum ChangeOperation {
    /// Create a new file with the given content.
    Create { content: String },
    /// Modify an existing file.
    Modify {
        /// Original content (for backup/rollback).
        original: String,
        /// New content to write.
        content: String,
    },
    /// Delete a file.
    Delete {
        /// Original content (for rollback).
        original: String,
    },
}

/// Result of applying a change.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    /// Path to the file.
    pub path: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Backup path if backup was created.
    pub backup_path: Option<PathBuf>,
}

/// Options for applying changes.
#[derive(Debug, Clone, Default)]
pub struct ApplyOptions {
    /// Create backups before modifying files.
    pub create_backups: bool,
    /// Backup directory (defaults to .arq/backups/).
    pub backup_dir: Option<PathBuf>,
    /// Dry run - don't actually write files.
    pub dry_run: bool,
}

/// Applies code changes to the filesystem.
pub struct ChangeApplier {
    /// Root directory for file operations.
    root: PathBuf,
    /// Options for applying changes.
    options: ApplyOptions,
    /// Backup of original files for rollback.
    backups: HashMap<String, String>,
}

impl ChangeApplier {
    /// Create a new change applier.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            options: ApplyOptions::default(),
            backups: HashMap::new(),
        }
    }

    /// Create a change applier with options.
    pub fn with_options(root: impl Into<PathBuf>, options: ApplyOptions) -> Self {
        Self {
            root: root.into(),
            options,
            backups: HashMap::new(),
        }
    }

    /// Apply a single change to the filesystem.
    pub fn apply(&mut self, change: &PendingChange) -> Result<ApplyResult, AgentError> {
        let full_path = self.root.join(&change.path);

        match &change.operation {
            ChangeOperation::Create { content } => {
                self.apply_create(&full_path, &change.path, content)
            }
            ChangeOperation::Modify { original, content } => {
                self.apply_modify(&full_path, &change.path, original, content)
            }
            ChangeOperation::Delete { original } => {
                self.apply_delete(&full_path, &change.path, original)
            }
        }
    }

    /// Apply multiple changes.
    pub fn apply_all(&mut self, changes: &[PendingChange]) -> Vec<ApplyResult> {
        changes
            .iter()
            .map(|c| {
                self.apply(c).unwrap_or_else(|e| ApplyResult {
                    path: c.path.clone(),
                    success: false,
                    error: Some(e.to_string()),
                    backup_path: None,
                })
            })
            .collect()
    }

    /// Rollback all applied changes.
    pub fn rollback(&self) -> Result<(), AgentError> {
        for (path, original) in &self.backups {
            let full_path = self.root.join(path);

            if original.is_empty() {
                // File was created, delete it
                if full_path.exists() {
                    fs::remove_file(&full_path).map_err(|e| AgentError::FileSystemError {
                        path: path.clone(),
                        message: format!("Failed to remove file during rollback: {}", e),
                    })?;
                }
            } else {
                // File was modified/deleted, restore it
                fs::write(&full_path, original).map_err(|e| AgentError::FileSystemError {
                    path: path.clone(),
                    message: format!("Failed to restore file during rollback: {}", e),
                })?;
            }
        }
        Ok(())
    }

    /// Clear the backup history.
    pub fn clear_backups(&mut self) {
        self.backups.clear();
    }

    /// Get the number of backed up files.
    pub fn backup_count(&self) -> usize {
        self.backups.len()
    }

    /// Apply a create operation.
    fn apply_create(
        &mut self,
        full_path: &Path,
        rel_path: &str,
        content: &str,
    ) -> Result<ApplyResult, AgentError> {
        // Store empty backup for rollback (file didn't exist)
        self.backups.insert(rel_path.to_string(), String::new());

        if self.options.dry_run {
            return Ok(ApplyResult {
                path: rel_path.to_string(),
                success: true,
                error: None,
                backup_path: None,
            });
        }

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AgentError::FileSystemError {
                path: rel_path.to_string(),
                message: format!("Failed to create parent directories: {}", e),
            })?;
        }

        // Write the file
        fs::write(full_path, content).map_err(|e| AgentError::FileSystemError {
            path: rel_path.to_string(),
            message: format!("Failed to create file: {}", e),
        })?;

        Ok(ApplyResult {
            path: rel_path.to_string(),
            success: true,
            error: None,
            backup_path: None,
        })
    }

    /// Apply a modify operation.
    fn apply_modify(
        &mut self,
        full_path: &Path,
        rel_path: &str,
        original: &str,
        content: &str,
    ) -> Result<ApplyResult, AgentError> {
        // Store original content for rollback
        self.backups
            .insert(rel_path.to_string(), original.to_string());

        let mut backup_path = None;

        // Create backup file if enabled
        if self.options.create_backups {
            let backup_dir = self
                .options
                .backup_dir
                .clone()
                .unwrap_or_else(|| self.root.join(".arq/backups"));

            fs::create_dir_all(&backup_dir).ok();

            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let backup_name = format!("{}_{}.bak", rel_path.replace(['/', '\\'], "_"), timestamp);
            let backup_file = backup_dir.join(&backup_name);

            if !self.options.dry_run {
                fs::write(&backup_file, original).ok();
            }
            backup_path = Some(backup_file);
        }

        if self.options.dry_run {
            return Ok(ApplyResult {
                path: rel_path.to_string(),
                success: true,
                error: None,
                backup_path,
            });
        }

        // Write the modified content
        fs::write(full_path, content).map_err(|e| AgentError::FileSystemError {
            path: rel_path.to_string(),
            message: format!("Failed to modify file: {}", e),
        })?;

        Ok(ApplyResult {
            path: rel_path.to_string(),
            success: true,
            error: None,
            backup_path,
        })
    }

    /// Apply a delete operation.
    fn apply_delete(
        &mut self,
        full_path: &Path,
        rel_path: &str,
        original: &str,
    ) -> Result<ApplyResult, AgentError> {
        // Store original content for rollback
        self.backups
            .insert(rel_path.to_string(), original.to_string());

        let mut backup_path = None;

        // Create backup file if enabled
        if self.options.create_backups {
            let backup_dir = self
                .options
                .backup_dir
                .clone()
                .unwrap_or_else(|| self.root.join(".arq/backups"));

            fs::create_dir_all(&backup_dir).ok();

            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let backup_name = format!("{}_{}.bak", rel_path.replace(['/', '\\'], "_"), timestamp);
            let backup_file = backup_dir.join(&backup_name);

            if !self.options.dry_run {
                fs::write(&backup_file, original).ok();
            }
            backup_path = Some(backup_file);
        }

        if self.options.dry_run {
            return Ok(ApplyResult {
                path: rel_path.to_string(),
                success: true,
                error: None,
                backup_path,
            });
        }

        // Delete the file
        if full_path.exists() {
            fs::remove_file(full_path).map_err(|e| AgentError::FileSystemError {
                path: rel_path.to_string(),
                message: format!("Failed to delete file: {}", e),
            })?;
        }

        Ok(ApplyResult {
            path: rel_path.to_string(),
            success: true,
            error: None,
            backup_path,
        })
    }
}

/// Convert a FileOperation to ChangeOperation.
impl From<&FileOperation> for ChangeOperation {
    fn from(op: &FileOperation) -> Self {
        match op {
            FileOperation::Create => ChangeOperation::Create {
                content: String::new(),
            },
            FileOperation::Modify { original } => ChangeOperation::Modify {
                original: original.clone(),
                content: String::new(),
            },
            FileOperation::Delete => ChangeOperation::Delete {
                original: String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_apply_create() {
        let temp = TempDir::new().unwrap();
        let mut applier = ChangeApplier::new(temp.path());

        let change = PendingChange {
            path: "new_file.txt".to_string(),
            operation: ChangeOperation::Create {
                content: "Hello, World!".to_string(),
            },
        };

        let result = applier.apply(&change).unwrap();
        assert!(result.success);

        let content = fs::read_to_string(temp.path().join("new_file.txt")).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_apply_modify() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("existing.txt");
        fs::write(&file_path, "Original content").unwrap();

        let mut applier = ChangeApplier::new(temp.path());

        let change = PendingChange {
            path: "existing.txt".to_string(),
            operation: ChangeOperation::Modify {
                original: "Original content".to_string(),
                content: "Modified content".to_string(),
            },
        };

        let result = applier.apply(&change).unwrap();
        assert!(result.success);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Modified content");
    }

    #[test]
    fn test_apply_delete() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("to_delete.txt");
        fs::write(&file_path, "Delete me").unwrap();

        let mut applier = ChangeApplier::new(temp.path());

        let change = PendingChange {
            path: "to_delete.txt".to_string(),
            operation: ChangeOperation::Delete {
                original: "Delete me".to_string(),
            },
        };

        let result = applier.apply(&change).unwrap();
        assert!(result.success);
        assert!(!file_path.exists());
    }

    #[test]
    fn test_rollback() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("rollback_test.txt");
        fs::write(&file_path, "Original").unwrap();

        let mut applier = ChangeApplier::new(temp.path());

        // Apply modification
        let change = PendingChange {
            path: "rollback_test.txt".to_string(),
            operation: ChangeOperation::Modify {
                original: "Original".to_string(),
                content: "Modified".to_string(),
            },
        };
        applier.apply(&change).unwrap();

        // Verify modification
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Modified");

        // Rollback
        applier.rollback().unwrap();

        // Verify rollback
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Original");
    }

    #[test]
    fn test_dry_run() {
        let temp = TempDir::new().unwrap();
        let options = ApplyOptions {
            dry_run: true,
            ..Default::default()
        };
        let mut applier = ChangeApplier::with_options(temp.path(), options);

        let change = PendingChange {
            path: "dry_run_file.txt".to_string(),
            operation: ChangeOperation::Create {
                content: "Should not exist".to_string(),
            },
        };

        let result = applier.apply(&change).unwrap();
        assert!(result.success);
        assert!(!temp.path().join("dry_run_file.txt").exists());
    }

    #[test]
    fn test_create_nested_directories() {
        let temp = TempDir::new().unwrap();
        let mut applier = ChangeApplier::new(temp.path());

        let change = PendingChange {
            path: "deep/nested/dir/file.txt".to_string(),
            operation: ChangeOperation::Create {
                content: "Nested content".to_string(),
            },
        };

        let result = applier.apply(&change).unwrap();
        assert!(result.success);

        let content = fs::read_to_string(temp.path().join("deep/nested/dir/file.txt")).unwrap();
        assert_eq!(content, "Nested content");
    }
}

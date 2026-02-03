//! RollbackFile tool - restores files from backups.

use async_trait::async_trait;
use std::path::Path;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for restoring files from backups.
pub struct RollbackFileTool;

#[async_trait]
impl Tool for RollbackFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rollback_file".to_string(),
            description: "Restore a file to its previous state from backup. \
                         Use this when a change caused issues and needs to be reverted. \
                         The tool will find the most recent backup for the specified file."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to restore (relative to project root)"
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
        let backup_dir = context.root.join(".arq_backups");

        // Check if backup directory exists
        if !backup_dir.exists() {
            return ToolResult::failure(format!(
                "No backups found. The backup directory {} does not exist.",
                backup_dir.display()
            ));
        }

        // Find the most recent backup for this file
        let file_name = match Path::new(path).file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => return ToolResult::failure("Invalid file path"),
        };

        let backup_path = match find_latest_backup(&backup_dir, &file_name) {
            Ok(path) => path,
            Err(result) => return result,
        };

        if context.dry_run {
            return ToolResult::success(format!(
                "[DRY RUN] Would restore {} from backup {}",
                path,
                backup_path.display()
            ))
            .with_modified_files(vec![path.to_string()]);
        }

        // Read backup content
        let backup_content = match std::fs::read(&backup_path) {
            Ok(content) => content,
            Err(e) => {
                return ToolResult::failure(format!(
                    "Failed to read backup {}: {}",
                    backup_path.display(),
                    e
                ))
            }
        };

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::failure(format!("Failed to create directories: {}", e));
            }
        }

        // Write restored content
        match std::fs::write(&full_path, &backup_content) {
            Ok(_) => ToolResult::success(format!(
                "Restored {} from backup {} ({} bytes)",
                path,
                backup_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                backup_content.len()
            ))
            .with_modified_files(vec![path.to_string()]),
            Err(e) => ToolResult::failure(format!("Failed to restore {}: {}", path, e)),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // Restoring files needs confirmation
    }
}

/// Find the latest backup for a file.
fn find_latest_backup(
    backup_dir: &Path,
    file_name: &str,
) -> Result<std::path::PathBuf, ToolResult> {
    let entries = match std::fs::read_dir(backup_dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(ToolResult::failure(format!(
                "Failed to read backup directory: {}",
                e
            )))
        }
    };

    // Collect matching backups (format: YYYYMMDD_HHMMSS_filename)
    let mut matching_backups: Vec<(String, std::path::PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let entry_name = entry.file_name().to_string_lossy().to_string();

        // Check if this backup matches our file (ends with _filename)
        if entry_name.ends_with(&format!("_{}", file_name)) {
            // Extract timestamp prefix for sorting
            // Format: YYYYMMDD_HHMMSS_filename
            if let Some(timestamp) = entry_name.strip_suffix(&format!("_{}", file_name)) {
                matching_backups.push((timestamp.to_string(), entry.path()));
            }
        }
    }

    if matching_backups.is_empty() {
        return Err(ToolResult::failure(format!(
            "No backups found for '{}'. Backups are created automatically when files are modified.",
            file_name
        )));
    }

    // Sort by timestamp descending (most recent first)
    matching_backups.sort_by(|a, b| b.0.cmp(&a.0));

    Ok(matching_backups[0].1.clone())
}

/// Restore all backed up files to their original state.
/// Returns the number of files restored.
pub fn rollback_all_files(
    root: &Path,
    backed_up_files: &[(String, String)],
) -> Result<(usize, Vec<String>), String> {
    let mut restored = 0;
    let mut restored_files = Vec::new();

    // Restore in reverse order (most recent changes first)
    for (original_path, backup_path) in backed_up_files.iter().rev() {
        let full_original = root.join(original_path);
        let full_backup = Path::new(backup_path);

        // Read backup
        let content = match std::fs::read(full_backup) {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("Failed to read backup {}: {}", backup_path, e));
            }
        };

        // Create parent directories if needed
        if let Some(parent) = full_original.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Err(format!(
                    "Failed to create directories for {}: {}",
                    original_path, e
                ));
            }
        }

        // Restore file
        if let Err(e) = std::fs::write(&full_original, &content) {
            return Err(format!("Failed to restore {}: {}", original_path, e));
        }

        restored += 1;
        restored_files.push(original_path.clone());
    }

    Ok((restored, restored_files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_latest_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path();

        // Create some fake backup files
        std::fs::write(backup_dir.join("20240101_120000_test.txt"), "old").unwrap();
        std::fs::write(backup_dir.join("20240102_120000_test.txt"), "new").unwrap();
        std::fs::write(backup_dir.join("20240101_120000_other.txt"), "other").unwrap();

        let result = find_latest_backup(backup_dir, "test.txt").unwrap();
        assert!(result.to_string_lossy().contains("20240102"));
    }

    #[test]
    fn test_find_latest_backup_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path();

        std::fs::write(backup_dir.join("20240101_120000_other.txt"), "data").unwrap();

        let result = find_latest_backup(backup_dir, "test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_rollback_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create backup directory
        let backup_dir = root.join(".arq_backups");
        std::fs::create_dir_all(&backup_dir).unwrap();

        // Create original file
        let original = root.join("src/test.txt");
        std::fs::create_dir_all(original.parent().unwrap()).unwrap();
        std::fs::write(&original, "modified content").unwrap();

        // Create backup
        let backup = backup_dir.join("20240101_120000_test.txt");
        std::fs::write(&backup, "original content").unwrap();

        let backed_up_files = vec![(
            "src/test.txt".to_string(),
            backup.to_string_lossy().to_string(),
        )];

        let (restored, files) = rollback_all_files(root, &backed_up_files).unwrap();
        assert_eq!(restored, 1);
        assert_eq!(files, vec!["src/test.txt"]);

        // Verify content was restored
        let content = std::fs::read_to_string(&original).unwrap();
        assert_eq!(content, "original content");
    }
}

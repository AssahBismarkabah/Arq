use std::fs;
use std::path::PathBuf;

use crate::config::{StorageConfig, DEFAULT_CURRENT_FILE};
use crate::planning::Plan;
use crate::research::ResearchDoc;
use crate::task::{Task, TaskSummary};

use super::error::StorageError;
use super::Storage;

/// File-based storage implementation.
///
/// Stores tasks in a directory structure:
/// ```text
/// {base_path}/
///   current           # Contains current task ID
///   tasks/
///     {task-id}/
///       task.json
///       research-doc.md
///       plan.yaml
/// ```
pub struct FileStorage {
    base_path: PathBuf,
    config: StorageConfig,
}

impl FileStorage {
    /// Creates a new FileStorage with the given base path and default config.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            config: StorageConfig::default(),
        }
    }

    /// Creates a new FileStorage with custom configuration.
    pub fn with_config(config: StorageConfig) -> Self {
        Self {
            base_path: PathBuf::from(&config.data_dir),
            config,
        }
    }

    /// Returns the path to the tasks directory.
    fn tasks_dir(&self) -> PathBuf {
        self.base_path.join(&self.config.tasks_dir)
    }

    /// Returns the path to a specific task's directory.
    fn task_dir(&self, id: &str) -> PathBuf {
        self.tasks_dir().join(id)
    }

    /// Returns the path to a task's metadata file.
    fn task_file(&self, id: &str) -> PathBuf {
        self.task_dir(id).join(&self.config.task_file)
    }

    /// Returns the path to a task's research document.
    fn research_doc_file(&self, id: &str) -> PathBuf {
        self.task_dir(id).join(&self.config.research_file)
    }

    /// Returns the path to a task's plan file.
    fn plan_file(&self, id: &str) -> PathBuf {
        self.task_dir(id).join(&self.config.plan_file)
    }

    /// Returns the path to the current task marker file.
    fn current_file(&self) -> PathBuf {
        self.base_path.join(DEFAULT_CURRENT_FILE)
    }

    /// Ensures the tasks directory exists.
    fn ensure_tasks_dir(&self) -> Result<(), StorageError> {
        let dir = self.tasks_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| StorageError::io(&dir, e))?;
        }
        Ok(())
    }

    /// Ensures a task's directory exists.
    fn ensure_task_dir(&self, id: &str) -> Result<(), StorageError> {
        self.ensure_tasks_dir()?;
        let dir = self.task_dir(id);
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| StorageError::io(&dir, e))?;
        }
        Ok(())
    }
}

impl Storage for FileStorage {
    fn save_task(&self, task: &Task) -> Result<(), StorageError> {
        self.ensure_task_dir(&task.id)?;

        let path = self.task_file(&task.id);
        let json = serde_json::to_string_pretty(task)?;
        fs::write(&path, json).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }

    fn load_task(&self, id: &str) -> Result<Task, StorageError> {
        let path = self.task_file(id);
        if !path.exists() {
            return Err(StorageError::TaskNotFound(id.to_string()));
        }

        let json = fs::read_to_string(&path).map_err(|e| StorageError::io(&path, e))?;
        let task: Task = serde_json::from_str(&json)?;

        Ok(task)
    }

    fn list_tasks(&self) -> Result<Vec<TaskSummary>, StorageError> {
        let tasks_dir = self.tasks_dir();
        if !tasks_dir.exists() {
            return Ok(Vec::new());
        }

        let mut summaries = Vec::new();

        let entries = fs::read_dir(&tasks_dir).map_err(|e| StorageError::io(&tasks_dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| StorageError::io(&tasks_dir, e))?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(id) = path.file_name().and_then(|n| n.to_str()) {
                    match self.load_task(id) {
                        Ok(task) => summaries.push(task.to_summary()),
                        Err(_) => continue, // Skip invalid tasks
                    }
                }
            }
        }

        // Sort by updated_at descending (most recent first)
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(summaries)
    }

    fn delete_task(&self, id: &str) -> Result<(), StorageError> {
        let dir = self.task_dir(id);
        if !dir.exists() {
            return Err(StorageError::TaskNotFound(id.to_string()));
        }

        fs::remove_dir_all(&dir).map_err(|e| StorageError::io(&dir, e))?;

        // Clear current if this was the current task
        if let Ok(Some(current_id)) = self.get_current_task_id() {
            if current_id == id {
                self.set_current_task_id(None)?;
            }
        }

        Ok(())
    }

    fn save_research_doc(&self, task_id: &str, doc: &ResearchDoc) -> Result<(), StorageError> {
        self.ensure_task_dir(task_id)?;

        let path = self.research_doc_file(task_id);
        let markdown = doc.to_markdown();
        fs::write(&path, markdown).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }

    fn save_plan(&self, task_id: &str, plan: &Plan) -> Result<(), StorageError> {
        self.ensure_task_dir(task_id)?;

        let path = self.plan_file(task_id);
        let yaml = plan.to_yaml()?;
        fs::write(&path, yaml).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }

    fn get_current_task_id(&self) -> Result<Option<String>, StorageError> {
        let path = self.current_file();
        if !path.exists() {
            return Ok(None);
        }

        let id = fs::read_to_string(&path)
            .map_err(|e| StorageError::io(&path, e))?
            .trim()
            .to_string();

        if id.is_empty() {
            Ok(None)
        } else {
            Ok(Some(id))
        }
    }

    fn set_current_task_id(&self, id: Option<&str>) -> Result<(), StorageError> {
        let path = self.current_file();

        // Ensure base directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
            }
        }

        match id {
            Some(id) => {
                fs::write(&path, id).map_err(|e| StorageError::io(&path, e))?;
            }
            None => {
                if path.exists() {
                    fs::remove_file(&path).map_err(|e| StorageError::io(&path, e))?;
                }
            }
        }

        Ok(())
    }
}


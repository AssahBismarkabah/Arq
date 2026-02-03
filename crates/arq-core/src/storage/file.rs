use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agent::AgentLoopState;
use crate::config::{StorageConfig, DEFAULT_CURRENT_FILE};
use crate::llm::Message;
use crate::planning::Plan;
use crate::research::ResearchDoc;
use crate::task::{Task, TaskSummary};

use super::error::StorageError;
use super::Storage;

/// Session data persisted for agent loop resumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentSessionData {
    /// The agent loop state.
    state: AgentLoopState,
    /// Conversation history.
    messages: Vec<Message>,
    /// Timestamp when saved.
    saved_at: chrono::DateTime<chrono::Utc>,
}

/// File-based storage implementation.
///
/// Stores data in two locations:
/// ```text
/// project/.arq/                    # User-visible outputs
///   research-doc.md                # Current task's research
///   plan.yaml                      # Current task's plan
///
/// ~/.arq/projects/{hash}/          # Internal data
///   current                        # Current task ID
///   tasks/{task-id}/
///     task.json                    # Task metadata
/// ```
pub struct FileStorage {
    /// Base path for internal data (~/.arq/projects/{hash}/)
    base_path: PathBuf,
    config: StorageConfig,
}

impl Default for FileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStorage {
    /// Creates a new FileStorage with default config, using project-specific directory.
    pub fn new() -> Self {
        let config = StorageConfig::default();
        let base_path = config.project_dir();
        Self { base_path, config }
    }

    /// Creates a new FileStorage with custom configuration.
    pub fn with_config(config: StorageConfig) -> Self {
        let base_path = config.project_dir();
        Self { base_path, config }
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

    /// Returns the path to the local .arq directory in the project.
    fn local_arq_dir(&self) -> PathBuf {
        self.config.local_arq_dir()
    }

    /// Returns the path to research-doc.md in local .arq/.
    fn research_doc_file(&self) -> PathBuf {
        self.config.local_research_path()
    }

    /// Returns the path to plan.yaml in local .arq/.
    fn plan_file(&self) -> PathBuf {
        self.config.local_plan_path()
    }

    /// Ensures the local .arq directory exists.
    fn ensure_local_arq_dir(&self) -> Result<(), StorageError> {
        let dir = self.local_arq_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| StorageError::io(&dir, e))?;
        }
        Ok(())
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

    /// Returns the path to an agent session file for a task.
    fn agent_session_file(&self, task_id: &str) -> PathBuf {
        self.task_dir(task_id).join("agent-session.json")
    }

    /// Returns the path to the project memory file (.arq/MEMORY.md).
    fn project_memory_file(&self) -> PathBuf {
        self.local_arq_dir().join("MEMORY.md")
    }

    /// Returns the path to a task's memory file.
    fn task_memory_file(&self, task_id: &str) -> PathBuf {
        self.task_dir(task_id).join("memory.md")
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

    fn save_research_doc(&self, _task_id: &str, doc: &ResearchDoc) -> Result<(), StorageError> {
        self.ensure_local_arq_dir()?;

        let path = self.research_doc_file();
        let markdown = doc.to_markdown();
        fs::write(&path, markdown).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }

    fn save_plan(&self, _task_id: &str, plan: &Plan) -> Result<(), StorageError> {
        self.ensure_local_arq_dir()?;

        let path = self.plan_file();
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

    fn save_agent_session(
        &self,
        task_id: &str,
        state: &AgentLoopState,
        messages: &[Message],
    ) -> Result<(), StorageError> {
        self.ensure_task_dir(task_id)?;

        let session = AgentSessionData {
            state: state.clone(),
            messages: messages.to_vec(),
            saved_at: chrono::Utc::now(),
        };

        let path = self.agent_session_file(task_id);
        let json = serde_json::to_string_pretty(&session)?;
        fs::write(&path, json).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }

    fn load_agent_session(
        &self,
        task_id: &str,
    ) -> Result<Option<(AgentLoopState, Vec<Message>)>, StorageError> {
        let path = self.agent_session_file(task_id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path).map_err(|e| StorageError::io(&path, e))?;
        let session: AgentSessionData = serde_json::from_str(&json)?;

        Ok(Some((session.state, session.messages)))
    }

    fn clear_agent_session(&self, task_id: &str) -> Result<(), StorageError> {
        let path = self.agent_session_file(task_id);

        if path.exists() {
            fs::remove_file(&path).map_err(|e| StorageError::io(&path, e))?;
        }

        Ok(())
    }

    fn has_agent_session(&self, task_id: &str) -> Result<bool, StorageError> {
        let path = self.agent_session_file(task_id);
        Ok(path.exists())
    }

    fn load_project_memory(&self) -> Result<Option<String>, StorageError> {
        let path = self.project_memory_file();

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|e| StorageError::io(&path, e))?;

        if content.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }

    fn save_project_memory(&self, memory: &str) -> Result<(), StorageError> {
        self.ensure_local_arq_dir()?;

        let path = self.project_memory_file();
        fs::write(&path, memory).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }

    fn load_task_memory(&self, task_id: &str) -> Result<Option<String>, StorageError> {
        let path = self.task_memory_file(task_id);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|e| StorageError::io(&path, e))?;

        if content.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }

    fn save_task_memory(&self, task_id: &str, memory: &str) -> Result<(), StorageError> {
        self.ensure_task_dir(task_id)?;

        let path = self.task_memory_file(task_id);
        fs::write(&path, memory).map_err(|e| StorageError::io(&path, e))?;

        Ok(())
    }
}

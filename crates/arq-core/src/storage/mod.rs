mod error;
mod file;

pub use error::StorageError;
pub use file::FileStorage;

use crate::agent::AgentLoopState;
use crate::llm::Message;
use crate::planning::Plan;
use crate::research::ResearchDoc;
use crate::task::{Task, TaskSummary};

/// Trait for task storage backends.
///
/// Implementations handle persisting tasks and their artifacts
/// to various storage systems (file system, database, etc.).
pub trait Storage {
    /// Saves a task to storage.
    fn save_task(&self, task: &Task) -> Result<(), StorageError>;

    /// Loads a task by ID.
    fn load_task(&self, id: &str) -> Result<Task, StorageError>;

    /// Lists all tasks as summaries.
    fn list_tasks(&self) -> Result<Vec<TaskSummary>, StorageError>;

    /// Deletes a task and all its artifacts.
    fn delete_task(&self, id: &str) -> Result<(), StorageError>;

    /// Saves a research document for a task.
    fn save_research_doc(&self, task_id: &str, doc: &ResearchDoc) -> Result<(), StorageError>;

    /// Saves a plan for a task.
    fn save_plan(&self, task_id: &str, plan: &Plan) -> Result<(), StorageError>;

    /// Gets the current task ID (if set).
    fn get_current_task_id(&self) -> Result<Option<String>, StorageError>;

    /// Sets the current task ID.
    fn set_current_task_id(&self, id: Option<&str>) -> Result<(), StorageError>;

    /// Save agent session state for resumption.
    ///
    /// Saves the current agent loop state and conversation history
    /// so the session can be resumed later if interrupted.
    fn save_agent_session(
        &self,
        task_id: &str,
        state: &AgentLoopState,
        messages: &[Message],
    ) -> Result<(), StorageError>;

    /// Load agent session state.
    ///
    /// Returns None if no saved session exists for this task.
    fn load_agent_session(
        &self,
        task_id: &str,
    ) -> Result<Option<(AgentLoopState, Vec<Message>)>, StorageError>;

    /// Clear agent session state.
    ///
    /// Called when a session completes successfully or user wants to start fresh.
    fn clear_agent_session(&self, task_id: &str) -> Result<(), StorageError>;

    /// Check if there's a resumable agent session for a task.
    fn has_agent_session(&self, task_id: &str) -> Result<bool, StorageError>;

    /// Load project-wide memory (.arq/MEMORY.md).
    ///
    /// Project memory contains persistent context like architecture decisions,
    /// coding conventions, and important notes that should be available across
    /// all tasks in the project.
    fn load_project_memory(&self) -> Result<Option<String>, StorageError>;

    /// Save project-wide memory.
    fn save_project_memory(&self, memory: &str) -> Result<(), StorageError>;

    /// Load task-specific memory.
    ///
    /// Task memory contains context specific to the current task,
    /// like intermediate findings and implementation notes.
    fn load_task_memory(&self, task_id: &str) -> Result<Option<String>, StorageError>;

    /// Save task-specific memory.
    fn save_task_memory(&self, task_id: &str, memory: &str) -> Result<(), StorageError>;
}

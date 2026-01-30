mod error;
mod file;

pub use error::StorageError;
pub use file::FileStorage;

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
}

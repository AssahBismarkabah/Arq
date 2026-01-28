use thiserror::Error;

use crate::phase::Phase;
use crate::planning::Plan;
use crate::research::ResearchDoc;
use crate::storage::{Storage, StorageError};
use crate::task::{Task, TaskSummary};

/// Manages tasks and their lifecycle.
///
/// Provides a high-level API for creating, retrieving, and updating tasks
/// with automatic persistence.
pub struct TaskManager<S: Storage> {
    storage: S,
}

impl<S: Storage> TaskManager<S> {
    /// Creates a new TaskManager with the given storage backend.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Creates a new task and persists it.
    pub fn create_task(&mut self, prompt: &str) -> Result<Task, ManagerError> {
        let task = Task::new(prompt);
        self.storage.save_task(&task)?;
        self.storage.set_current_task_id(Some(&task.id))?;
        Ok(task)
    }

    /// Gets a task by ID.
    pub fn get_task(&self, id: &str) -> Result<Task, ManagerError> {
        Ok(self.storage.load_task(id)?)
    }

    /// Gets the current task (if any).
    pub fn get_current_task(&self) -> Result<Option<Task>, ManagerError> {
        match self.storage.get_current_task_id()? {
            Some(id) => Ok(Some(self.storage.load_task(&id)?)),
            None => Ok(None),
        }
    }

    /// Sets the current task by ID.
    pub fn set_current_task(&mut self, id: &str) -> Result<(), ManagerError> {
        // Verify task exists
        self.storage.load_task(id)?;
        self.storage.set_current_task_id(Some(id))?;
        Ok(())
    }

    /// Clears the current task.
    pub fn clear_current_task(&mut self) -> Result<(), ManagerError> {
        self.storage.set_current_task_id(None)?;
        Ok(())
    }

    /// Lists all tasks.
    pub fn list_tasks(&self) -> Result<Vec<TaskSummary>, ManagerError> {
        Ok(self.storage.list_tasks()?)
    }

    /// Deletes a task by ID.
    pub fn delete_task(&mut self, id: &str) -> Result<(), ManagerError> {
        self.storage.delete_task(id)?;
        Ok(())
    }

    /// Sets the research document for a task and persists it.
    pub fn set_research_doc(
        &mut self,
        task_id: &str,
        doc: ResearchDoc,
    ) -> Result<Task, ManagerError> {
        let mut task = self.storage.load_task(task_id)?;
        task.set_research_doc(doc.clone())
            .map_err(|e| ManagerError::TaskError(e.to_string()))?;
        self.storage.save_task(&task)?;
        self.storage.save_research_doc(task_id, &doc)?;
        Ok(task)
    }

    /// Sets the plan for a task and persists it.
    pub fn set_plan(&mut self, task_id: &str, plan: Plan) -> Result<Task, ManagerError> {
        let mut task = self.storage.load_task(task_id)?;
        task.set_plan(plan.clone())
            .map_err(|e| ManagerError::TaskError(e.to_string()))?;
        self.storage.save_task(&task)?;
        self.storage.save_plan(task_id, &plan)?;
        Ok(task)
    }

    /// Advances a task to the next phase.
    pub fn advance_phase(&mut self, task_id: &str) -> Result<Phase, ManagerError> {
        let mut task = self.storage.load_task(task_id)?;

        if !task.can_advance() {
            return Err(ManagerError::CannotAdvance {
                phase: task.phase,
                reason: "Prerequisites not met".to_string(),
            });
        }

        task.advance_phase();
        self.storage.save_task(&task)?;
        Ok(task.phase)
    }
}

/// Errors that can occur in TaskManager operations.
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Task error: {0}")]
    TaskError(String),

    #[error("Cannot advance from {phase:?}: {reason}")]
    CannotAdvance { phase: Phase, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::FileStorage;
    use tempfile::TempDir;

    fn create_test_manager() -> (TaskManager<FileStorage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path());
        let manager = TaskManager::new(storage);
        (manager, temp_dir)
    }

    #[test]
    fn test_create_task() {
        let (mut manager, _temp) = create_test_manager();

        let task = manager.create_task("Test task").unwrap();
        assert_eq!(task.prompt, "Test task");
        assert_eq!(task.phase, Phase::Research);

        // Should be set as current
        let current = manager.get_current_task().unwrap();
        assert!(current.is_some());
        assert_eq!(current.unwrap().id, task.id);
    }

    #[test]
    fn test_list_tasks() {
        let (mut manager, _temp) = create_test_manager();

        manager.create_task("Task 1").unwrap();
        manager.create_task("Task 2").unwrap();

        let tasks = manager.list_tasks().unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_delete_task() {
        let (mut manager, _temp) = create_test_manager();

        let task = manager.create_task("Task to delete").unwrap();
        manager.delete_task(&task.id).unwrap();

        assert!(manager.get_task(&task.id).is_err());
    }

    #[test]
    fn test_set_research_doc() {
        let (mut manager, _temp) = create_test_manager();

        let task = manager.create_task("Test task").unwrap();
        let doc = ResearchDoc::new("Test task");

        let updated = manager.set_research_doc(&task.id, doc).unwrap();
        assert!(updated.research_doc.is_some());
        assert!(updated.can_advance());
    }

    #[test]
    fn test_advance_phase() {
        let (mut manager, _temp) = create_test_manager();

        let task = manager.create_task("Test task").unwrap();

        // Can't advance without research doc
        assert!(manager.advance_phase(&task.id).is_err());

        // Add research doc
        let doc = ResearchDoc::new("Test task");
        manager.set_research_doc(&task.id, doc).unwrap();

        // Now can advance
        let new_phase = manager.advance_phase(&task.id).unwrap();
        assert_eq!(new_phase, Phase::Planning);
    }
}

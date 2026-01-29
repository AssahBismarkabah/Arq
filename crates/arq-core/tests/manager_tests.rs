use arq_core::{FileStorage, StorageConfig, TaskManager, Phase, ResearchDoc};
use tempfile::TempDir;

fn create_test_manager() -> (TaskManager<FileStorage>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        project_root: Some(temp_dir.path().to_path_buf()),
        ..StorageConfig::default()
    };
    let storage = FileStorage::with_config(config);
    let manager = TaskManager::new(storage);
    (manager, temp_dir)
}

#[test]
fn test_create_task() {
    let (mut manager, _temp) = create_test_manager();

    let task = manager.create_task("Test task").unwrap();

    assert_eq!(task.prompt, "Test task");
    assert_eq!(task.phase, Phase::Research);

    // Should be set as current task
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

    let tasks = manager.list_tasks().unwrap();
    assert_eq!(tasks.len(), 0);
}

#[test]
fn test_set_research_doc() {
    let (mut manager, _temp) = create_test_manager();

    let task = manager.create_task("Research task").unwrap();

    let doc = ResearchDoc::new("test");
    manager.set_research_doc(&task.id, doc).unwrap();

    let updated = manager.get_current_task().unwrap().unwrap();
    assert!(updated.research_doc.is_some());
}

#[test]
fn test_advance_phase() {
    let (mut manager, _temp) = create_test_manager();

    let task = manager.create_task("Advance task").unwrap();

    // Set research doc so we can advance
    let doc = ResearchDoc::new("test");
    manager.set_research_doc(&task.id, doc).unwrap();

    // Advance to Planning
    let new_phase = manager.advance_phase(&task.id).unwrap();
    assert_eq!(new_phase, Phase::Planning);

    let updated = manager.get_current_task().unwrap().unwrap();
    assert_eq!(updated.phase, Phase::Planning);
}

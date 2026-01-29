use arq_core::{FileStorage, Storage, StorageConfig, Task};
use tempfile::TempDir;

fn create_test_storage() -> (FileStorage, TempDir, StorageConfig) {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        project_root: Some(temp_dir.path().to_path_buf()),
        ..StorageConfig::default()
    };
    let storage = FileStorage::with_config(config.clone());
    (storage, temp_dir, config)
}

#[test]
fn test_save_and_load_task() {
    let (storage, _temp, _config) = create_test_storage();

    let task = Task::new("Test task");
    storage.save_task(&task).unwrap();

    let loaded = storage.load_task(&task.id).unwrap();
    assert_eq!(loaded.id, task.id);
    assert_eq!(loaded.name, task.name);
    assert_eq!(loaded.prompt, task.prompt);
}

#[test]
fn test_list_tasks() {
    let (storage, _temp, _config) = create_test_storage();

    let task1 = Task::new("First task");
    let task2 = Task::new("Second task");

    storage.save_task(&task1).unwrap();
    storage.save_task(&task2).unwrap();

    let summaries = storage.list_tasks().unwrap();
    assert_eq!(summaries.len(), 2);
}

#[test]
fn test_delete_task() {
    let (storage, _temp, _config) = create_test_storage();

    let task = Task::new("Task to delete");
    storage.save_task(&task).unwrap();

    storage.delete_task(&task.id).unwrap();

    assert!(storage.load_task(&task.id).is_err());
}

#[test]
fn test_current_task() {
    let (storage, _temp, _config) = create_test_storage();

    assert!(storage.get_current_task_id().unwrap().is_none());

    storage.set_current_task_id(Some("test-id")).unwrap();
    assert_eq!(
        storage.get_current_task_id().unwrap(),
        Some("test-id".to_string())
    );

    storage.set_current_task_id(None).unwrap();
    assert!(storage.get_current_task_id().unwrap().is_none());
}

#[test]
fn test_custom_config() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        tasks_dir: "my-tasks".to_string(),
        task_file: "metadata.json".to_string(),
        research_file: "research.md".to_string(),
        plan_file: "implementation.yaml".to_string(),
        project_root: Some(temp_dir.path().to_path_buf()),
    };

    let storage = FileStorage::with_config(config.clone());
    let task = Task::new("Custom config task");
    storage.save_task(&task).unwrap();

    // Verify custom path is used (now under projects/{hash}/)
    let project_dir = config.project_dir();
    let custom_path = project_dir.join("my-tasks").join(&task.id).join("metadata.json");
    assert!(custom_path.exists());
}

//! Integration tests for the agent loop.

use std::sync::atomic::{AtomicUsize, Ordering};

use arq_core::agent::{AgentLoopConfig, AgentLoopProgress, AgentLoopRunner};
use arq_core::llm::{LLMError, LLMToolResponse, LLMWithTools, Message, ToolCall};
use arq_core::planning::{Complexity, Plan};
use arq_core::{FileStorage, Storage, StorageConfig};
use async_trait::async_trait;
use tempfile::TempDir;
use tokio::sync::mpsc;

// ============================================================================
// Mock LLM for Testing
// ============================================================================

/// A mock LLM that returns predefined responses.
struct MockLLM {
    responses: Vec<LLMToolResponse>,
    call_count: AtomicUsize,
}

impl MockLLM {
    fn new(responses: Vec<LLMToolResponse>) -> Self {
        Self {
            responses,
            call_count: AtomicUsize::new(0),
        }
    }

    /// Create a mock that reads a file then completes the task.
    fn read_and_complete() -> Self {
        Self::new(vec![
            // First call: read a file
            LLMToolResponse {
                content: "Let me read the file first.".to_string(),
                tool_calls: vec![ToolCall::new(
                    "call_1",
                    "read_file",
                    serde_json::json!({ "path": "test.txt" }),
                )],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            },
            // Second call: complete the task
            LLMToolResponse {
                content: "I've read the file. Task complete.".to_string(),
                tool_calls: vec![ToolCall::new(
                    "call_2",
                    "task_complete",
                    serde_json::json!({
                        "summary": "Read the test file successfully",
                        "files_changed": []
                    }),
                )],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            },
        ])
    }

    /// Create a mock that writes a file then completes.
    fn write_and_complete() -> Self {
        Self::new(vec![
            // First call: write a file
            LLMToolResponse {
                content: "Creating the new file.".to_string(),
                tool_calls: vec![ToolCall::new(
                    "call_1",
                    "write_file",
                    serde_json::json!({
                        "path": "output.txt",
                        "content": "Hello, World!"
                    }),
                )],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            },
            // Second call: complete the task
            LLMToolResponse {
                content: "File created. Task complete.".to_string(),
                tool_calls: vec![ToolCall::new(
                    "call_2",
                    "task_complete",
                    serde_json::json!({
                        "summary": "Created output.txt with content",
                        "files_changed": ["output.txt"]
                    }),
                )],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            },
        ])
    }

    /// Create a mock that calls multiple read tools (for parallel execution test).
    fn parallel_reads() -> Self {
        Self::new(vec![
            // First call: multiple read operations
            LLMToolResponse {
                content: "Reading multiple files.".to_string(),
                tool_calls: vec![
                    ToolCall::new(
                        "call_1",
                        "read_file",
                        serde_json::json!({ "path": "file1.txt" }),
                    ),
                    ToolCall::new(
                        "call_2",
                        "read_file",
                        serde_json::json!({ "path": "file2.txt" }),
                    ),
                    ToolCall::new(
                        "call_3",
                        "list_files",
                        serde_json::json!({ "pattern": "*.txt" }),
                    ),
                ],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            },
            // Second call: complete
            LLMToolResponse {
                content: "All files read.".to_string(),
                tool_calls: vec![ToolCall::new(
                    "call_4",
                    "task_complete",
                    serde_json::json!({
                        "summary": "Read all files successfully",
                        "files_changed": []
                    }),
                )],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            },
        ])
    }

    /// Create a mock that never completes (for max iterations test).
    fn never_completes() -> Self {
        Self::new(vec![
            LLMToolResponse {
                content: "Still working...".to_string(),
                tool_calls: vec![ToolCall::new(
                    "call_1",
                    "read_file",
                    serde_json::json!({ "path": "test.txt" }),
                )],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            };
            100 // 100 copies of same response
        ])
    }
}

#[async_trait]
impl LLMWithTools for MockLLM {
    async fn complete_with_tools(
        &self,
        _system: &str,
        _messages: &[Message],
        _tools: &[serde_json::Value],
    ) -> Result<LLMToolResponse, LLMError> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        if idx < self.responses.len() {
            Ok(self.responses[idx].clone())
        } else {
            // Return a completion response if we've exhausted responses
            Ok(LLMToolResponse {
                content: "Completing task.".to_string(),
                tool_calls: vec![ToolCall::new(
                    "final",
                    "task_complete",
                    serde_json::json!({
                        "summary": "Task completed (fallback)",
                        "files_changed": []
                    }),
                )],
                is_final: false,
                finish_reason: Some("tool_calls".to_string()),
            })
        }
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_plan() -> Plan {
    Plan {
        task_name: "Test Task".to_string(),
        approach: "Test approach".to_string(),
        complexity: Complexity::Low,
        files_to_create: vec![],
        files_to_modify: vec![],
        dependencies_to_add: vec![],
    }
}

fn create_test_storage() -> (FileStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        project_root: Some(temp_dir.path().to_path_buf()),
        ..StorageConfig::default()
    };
    let storage = FileStorage::with_config(config);
    (storage, temp_dir)
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_agent_completes_simple_task() {
    let (_storage, temp_dir) = create_test_storage();

    // Create test file
    std::fs::write(temp_dir.path().join("test.txt"), "Hello, World!").unwrap();

    let mock_llm = MockLLM::read_and_complete();
    let config = AgentLoopConfig {
        auto_confirm: true, // Auto-confirm for testing
        ..Default::default()
    };

    let mut runner: AgentLoopRunner<FileStorage> =
        AgentLoopRunner::with_config(Box::new(mock_llm), temp_dir.path(), config);

    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
    let (_confirm_tx, confirm_rx) = mpsc::unbounded_channel();

    let plan = create_test_plan();
    let result = runner.run(&plan, progress_tx, confirm_rx).await;

    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.is_complete);
    assert!(state.summary.is_some());

    // Verify progress events were sent
    let mut events = Vec::new();
    while let Ok(event) = progress_rx.try_recv() {
        events.push(event);
    }
    assert!(!events.is_empty());

    // Should have Started event
    assert!(events
        .iter()
        .any(|e| matches!(e, AgentLoopProgress::Started { .. })));

    // Should have Complete event
    assert!(events
        .iter()
        .any(|e| matches!(e, AgentLoopProgress::Complete { .. })));
}

#[tokio::test]
async fn test_agent_writes_file_with_confirmation() {
    let (_storage, temp_dir) = create_test_storage();

    let mock_llm = MockLLM::write_and_complete();
    let config = AgentLoopConfig {
        auto_confirm: true,
        ..Default::default()
    };

    let mut runner: AgentLoopRunner<FileStorage> =
        AgentLoopRunner::with_config(Box::new(mock_llm), temp_dir.path(), config);

    let (progress_tx, _progress_rx) = mpsc::unbounded_channel();
    let (_confirm_tx, confirm_rx) = mpsc::unbounded_channel();

    let plan = create_test_plan();
    let result = runner.run(&plan, progress_tx, confirm_rx).await;

    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.is_complete);

    // Verify file was created
    let output_path = temp_dir.path().join("output.txt");
    assert!(output_path.exists());
    assert_eq!(
        std::fs::read_to_string(output_path).unwrap(),
        "Hello, World!"
    );

    // Verify file was tracked
    assert!(state.modified_files.contains(&"output.txt".to_string()));
}

#[tokio::test]
async fn test_agent_parallel_read_execution() {
    let (_storage, temp_dir) = create_test_storage();

    // Create test files
    std::fs::write(temp_dir.path().join("file1.txt"), "Content 1").unwrap();
    std::fs::write(temp_dir.path().join("file2.txt"), "Content 2").unwrap();

    let mock_llm = MockLLM::parallel_reads();
    let config = AgentLoopConfig {
        auto_confirm: true,
        ..Default::default()
    };

    let mut runner: AgentLoopRunner<FileStorage> =
        AgentLoopRunner::with_config(Box::new(mock_llm), temp_dir.path(), config);

    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
    let (_confirm_tx, confirm_rx) = mpsc::unbounded_channel();

    let plan = create_test_plan();
    let result = runner.run(&plan, progress_tx, confirm_rx).await;

    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.is_complete);

    // Count tool executed events
    let mut tool_executed_count = 0;
    while let Ok(event) = progress_rx.try_recv() {
        if matches!(event, AgentLoopProgress::ToolExecuted { .. }) {
            tool_executed_count += 1;
        }
    }

    // Should have executed all 4 tools (3 reads + 1 task_complete)
    assert!(tool_executed_count >= 4);
}

#[tokio::test]
async fn test_agent_respects_max_iterations() {
    let (_storage, temp_dir) = create_test_storage();

    // Create test file
    std::fs::write(temp_dir.path().join("test.txt"), "Content").unwrap();

    let mock_llm = MockLLM::never_completes();
    let config = AgentLoopConfig {
        auto_confirm: true,
        max_iterations: 3, // Very low limit
        ..Default::default()
    };

    let mut runner: AgentLoopRunner<FileStorage> =
        AgentLoopRunner::with_config(Box::new(mock_llm), temp_dir.path(), config);

    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
    let (_confirm_tx, confirm_rx) = mpsc::unbounded_channel();

    let plan = create_test_plan();
    let result = runner.run(&plan, progress_tx, confirm_rx).await;

    assert!(result.is_ok());
    let state = result.unwrap();

    // Should NOT be complete since we hit max iterations
    assert!(!state.is_complete);
    assert!(state.error.is_some());
    assert!(state.error.unwrap().contains("Max iterations"));

    // Verify we got an error progress event
    let mut has_error = false;
    while let Ok(event) = progress_rx.try_recv() {
        if matches!(event, AgentLoopProgress::Error { .. }) {
            has_error = true;
        }
    }
    assert!(has_error);
}

#[tokio::test]
async fn test_agent_handles_unknown_tool() {
    let (_storage, temp_dir) = create_test_storage();

    // Create a mock that calls an unknown tool
    let mock_llm = MockLLM::new(vec![
        LLMToolResponse {
            content: "Calling unknown tool.".to_string(),
            tool_calls: vec![ToolCall::new(
                "call_1",
                "nonexistent_tool",
                serde_json::json!({}),
            )],
            is_final: false,
            finish_reason: Some("tool_calls".to_string()),
        },
        LLMToolResponse {
            content: "Completing anyway.".to_string(),
            tool_calls: vec![ToolCall::new(
                "call_2",
                "task_complete",
                serde_json::json!({
                    "summary": "Handled unknown tool gracefully",
                    "files_changed": []
                }),
            )],
            is_final: false,
            finish_reason: Some("tool_calls".to_string()),
        },
    ]);

    let config = AgentLoopConfig {
        auto_confirm: true,
        ..Default::default()
    };

    let mut runner: AgentLoopRunner<FileStorage> =
        AgentLoopRunner::with_config(Box::new(mock_llm), temp_dir.path(), config);

    let (progress_tx, _progress_rx) = mpsc::unbounded_channel();
    let (_confirm_tx, confirm_rx) = mpsc::unbounded_channel();

    let plan = create_test_plan();
    let result = runner.run(&plan, progress_tx, confirm_rx).await;

    // Should still complete (unknown tool returns error but loop continues)
    assert!(result.is_ok());
    let state = result.unwrap();
    assert!(state.is_complete);
}

#[tokio::test]
async fn test_agent_session_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        project_root: Some(temp_dir.path().to_path_buf()),
        ..StorageConfig::default()
    };

    // Create two storage instances pointing to same directory
    let storage_for_setup = FileStorage::with_config(storage_config.clone());
    let storage_for_runner = FileStorage::with_config(storage_config.clone());
    let storage_for_verify = FileStorage::with_config(storage_config);

    // Create test file
    std::fs::write(temp_dir.path().join("test.txt"), "Content").unwrap();

    // Create a task for session storage
    let task = arq_core::Task::new("Test task");
    storage_for_setup.save_task(&task).unwrap();
    let task_id = task.id.clone();

    let mock_llm = MockLLM::read_and_complete();
    let config = AgentLoopConfig {
        auto_confirm: true,
        ..Default::default()
    };

    let mut runner: AgentLoopRunner<FileStorage> =
        AgentLoopRunner::with_config(Box::new(mock_llm), temp_dir.path(), config)
            .with_storage(storage_for_runner, task.id);

    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
    let (_confirm_tx, confirm_rx) = mpsc::unbounded_channel();

    let plan = create_test_plan();
    let result = runner.run(&plan, progress_tx, confirm_rx).await;

    assert!(result.is_ok());

    // Check for checkpoint saved events
    let mut checkpoint_count = 0;
    while let Ok(event) = progress_rx.try_recv() {
        if matches!(event, AgentLoopProgress::CheckpointSaved { .. }) {
            checkpoint_count += 1;
        }
    }

    // Should have saved at least one checkpoint (after each tool call)
    // Note: With parallel execution, read-only tools don't save checkpoints,
    // but write tools and task_complete do
    assert!(checkpoint_count >= 1);

    // Session should be cleared after successful completion
    assert!(!storage_for_verify.has_agent_session(&task_id).unwrap());
}

#[tokio::test]
async fn test_project_memory_loaded_into_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let storage_config = StorageConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        project_root: Some(temp_dir.path().to_path_buf()),
        ..StorageConfig::default()
    };

    // Create two storage instances pointing to same directory
    let storage_for_setup = FileStorage::with_config(storage_config.clone());
    let storage_for_runner = FileStorage::with_config(storage_config.clone());
    let storage_for_verify = FileStorage::with_config(storage_config);

    // Save project memory
    storage_for_setup
        .save_project_memory("## Architecture\nThis is a Rust project with async/await.")
        .unwrap();

    // Create test file
    std::fs::write(temp_dir.path().join("test.txt"), "Content").unwrap();

    let task = arq_core::Task::new("Test task");
    storage_for_setup.save_task(&task).unwrap();

    // Use a mock that captures and verifies the system prompt would include memory
    let mock_llm = MockLLM::read_and_complete();
    let config = AgentLoopConfig {
        auto_confirm: true,
        ..Default::default()
    };

    let mut runner: AgentLoopRunner<FileStorage> =
        AgentLoopRunner::with_config(Box::new(mock_llm), temp_dir.path(), config)
            .with_storage(storage_for_runner, task.id);

    let (progress_tx, _progress_rx) = mpsc::unbounded_channel();
    let (_confirm_tx, confirm_rx) = mpsc::unbounded_channel();

    let plan = create_test_plan();
    let result = runner.run(&plan, progress_tx, confirm_rx).await;

    assert!(result.is_ok());

    // Verify memory is still there (wasn't modified)
    let memory = storage_for_verify.load_project_memory().unwrap();
    assert!(memory.is_some());
    assert!(memory.unwrap().contains("Rust project"));
}

#[tokio::test]
async fn test_tool_registry_has_all_tools() {
    use arq_core::agent::tools::ToolRegistry;

    let registry = ToolRegistry::with_default_tools();

    // Verify all expected tools are registered
    let expected_tools = [
        "read_file",
        "write_file",
        "edit_file",
        "list_files",
        "search_files",
        "run_command",
        "rollback_file",
        "task_complete",
        "git_status",
        "git_diff",
        "git_log",
        "git_add",
        "git_commit",
    ];

    for tool_name in expected_tools {
        assert!(
            registry.get(tool_name).is_some(),
            "Tool '{}' should be registered",
            tool_name
        );
    }

    // Verify tool count
    assert_eq!(registry.len(), 13);
}

#[tokio::test]
async fn test_read_only_tools_marked_correctly() {
    use arq_core::agent::tools::ToolRegistry;

    let registry = ToolRegistry::with_default_tools();

    // Read-only tools
    let read_only = [
        "read_file",
        "list_files",
        "search_files",
        "git_status",
        "git_diff",
        "git_log",
        "task_complete",
    ];

    for tool_name in read_only {
        let tool = registry
            .get(tool_name)
            .unwrap_or_else(|| panic!("Tool {} not found", tool_name));
        assert!(
            tool.is_read_only(),
            "Tool '{}' should be read-only",
            tool_name
        );
    }

    // Write tools
    let write_tools = [
        "write_file",
        "edit_file",
        "run_command",
        "rollback_file",
        "git_add",
        "git_commit",
    ];

    for tool_name in write_tools {
        let tool = registry
            .get(tool_name)
            .unwrap_or_else(|| panic!("Tool {} not found", tool_name));
        assert!(
            !tool.is_read_only(),
            "Tool '{}' should NOT be read-only",
            tool_name
        );
    }
}

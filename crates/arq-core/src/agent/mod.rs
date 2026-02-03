//! Agent phase - executes the approved plan.
//!
//! The Agent is the third and final phase in Arq's workflow.
//! It implements the approved plan using an agentic loop with tools.
//!
//! # Core Principle
//!
//! "Build exactly what was approved."
//!
//! The Agent is a disciplined executor that implements the specification
//! using an iterative tool-calling loop until the task is complete.
//!
//! # Agentic Loop Flow
//!
//! 1. Load plan.yaml as the contract
//! 2. LLM decides what tool to call (read_file, write_file, run_command, etc.)
//! 3. Tool executes, result returned to LLM
//! 4. LLM sees result, decides next action
//! 5. Repeat until task_complete is called
//! 6. User reviews changes and approves
//!
//! # Available Tools
//!
//! - `read_file` - Read file contents
//! - `write_file` - Create or overwrite files
//! - `edit_file` - Search/replace edits
//! - `run_command` - Execute shell commands (build, test, etc.)
//! - `list_files` - List files matching patterns
//! - `search_files` - Search for patterns in code
//! - `task_complete` - Signal task completion

// Legacy modules (kept for migration)
mod applier;
mod conformance;
mod diff;
mod error;
mod executor;
mod generator;
mod progress;
mod types;

// New agentic loop modules
mod loop_runner;
pub mod tools;
mod verification;

// Legacy exports (kept for backward compatibility)
pub use applier::{ApplyOptions, ApplyResult, ChangeApplier, ChangeOperation, PendingChange};
pub use conformance::ConformanceChecker;
pub use diff::{DiffGenerator, DiffLine, DiffLineType, DiffStats, FileDiff};
pub use error::AgentError;
pub use executor::{AgentExecutor, ConformanceStatus, ExecutionItem, ExecutionResult};
pub use generator::CodeGenerator;
pub use progress::AgentProgress;
pub use types::{
    ConformanceCheck, ConformanceResult, Deviation, DeviationSeverity, ExecutionSummary,
    FileOperation, GeneratedCode, ItemStatus, TestResults,
};

// New agentic loop exports
pub use loop_runner::{
    AgentLoopConfig, AgentLoopProgress, AgentLoopRunner, AgentLoopState, ToolConfirmation,
};
pub use verification::{
    detect_and_get_commands, detect_project_type, get_verification_commands, ProjectType,
    VerificationCommands,
};

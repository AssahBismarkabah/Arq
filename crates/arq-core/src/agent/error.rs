//! Agent phase error types.

use thiserror::Error;

/// Errors that can occur during the agent phase.
#[derive(Debug, Error)]
pub enum AgentError {
    /// No plan found for the task.
    #[error("No plan found. Complete the planning phase first.")]
    NoPlan,

    /// Plan file could not be loaded.
    #[error("Failed to load plan: {0}")]
    PlanLoadError(String),

    /// Code generation failed.
    #[error("Code generation failed: {0}")]
    GenerationError(String),

    /// Conformance check failed with critical deviations.
    #[error("Conformance check failed: {0}")]
    ConformanceError(String),

    /// Failed to read source file.
    #[error("Failed to read file '{path}': {message}")]
    FileReadError { path: String, message: String },

    /// Failed to write file.
    #[error("Failed to write file '{path}': {message}")]
    FileWriteError { path: String, message: String },

    /// Failed to create directory.
    #[error("Failed to create directory '{path}': {message}")]
    DirectoryError { path: String, message: String },

    /// LLM error during generation.
    #[error("LLM error: {0}")]
    LLMError(#[from] crate::llm::LLMError),

    /// Invalid plan item.
    #[error("Invalid plan item at index {index}: {message}")]
    InvalidPlanItem { index: usize, message: String },

    /// Operation cancelled by user.
    #[error("Operation cancelled")]
    Cancelled,

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Filesystem operation error.
    #[error("Filesystem error for '{path}': {message}")]
    FileSystemError { path: String, message: String },
}

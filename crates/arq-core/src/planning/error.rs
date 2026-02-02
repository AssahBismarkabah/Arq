use thiserror::Error;

/// Errors that can occur during the planning phase.
#[derive(Debug, Error)]
pub enum PlanningError {
    /// Research document is required to generate a plan
    #[error("Research document required")]
    NoResearchDoc,

    /// Error from the LLM
    #[error("LLM error: {0}")]
    LLMError(#[from] crate::llm::LLMError),

    /// Failed to parse the LLM response
    #[error("Failed to parse plan: {0}")]
    ParseError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// No approach was selected
    #[error("No approach selected")]
    NoApproachSelected,

    /// Invalid approach index
    #[error("Invalid approach index: {0}")]
    InvalidApproachIndex(usize),
}

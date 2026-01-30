use serde::{Deserialize, Serialize};

/// Represents the current phase of a task in Arq.
///
/// Tasks progress linearly through phases:
/// Research → Planning → Agent → Complete
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Analyzing codebase, gathering context, validating understanding
    #[default]
    Research,
    /// Creating architectural decisions and specifications
    Planning,
    /// Executing the approved plan, generating code
    Agent,
    /// Task completed successfully
    Complete,
}

impl Phase {
    /// Returns the next phase in the workflow.
    /// Returns None if already complete.
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Research => Some(Phase::Planning),
            Phase::Planning => Some(Phase::Agent),
            Phase::Agent => Some(Phase::Complete),
            Phase::Complete => None,
        }
    }

    /// Returns true if this phase can transition to the next phase.
    pub fn can_advance(&self) -> bool {
        self.next().is_some()
    }

    /// Returns a human-readable name for the phase.
    pub fn display_name(&self) -> &'static str {
        match self {
            Phase::Research => "Research",
            Phase::Planning => "Planning",
            Phase::Agent => "Agent",
            Phase::Complete => "Complete",
        }
    }
}

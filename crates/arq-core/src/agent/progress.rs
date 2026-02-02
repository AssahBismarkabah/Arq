//! Agent phase progress tracking.

use serde::{Deserialize, Serialize};

/// Progress updates during agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentProgress {
    /// Loading the plan from file.
    LoadingPlan,

    /// Starting execution of plan items.
    StartingExecution { total_items: usize },

    /// Generating code for a plan item.
    GeneratingCode {
        item_index: usize,
        total_items: usize,
        item_description: String,
    },

    /// Checking conformance of generated code.
    CheckingConformance { item_index: usize },

    /// Awaiting user review of generated code.
    AwaitingReview { item_index: usize },

    /// User accepted the generated code.
    ItemAccepted { item_index: usize },

    /// User requested regeneration.
    Regenerating { item_index: usize },

    /// Running tests (optional).
    RunningTests,

    /// All items processed, awaiting final approval.
    AwaitingApproval,

    /// Applying changes to filesystem.
    ApplyingChanges,

    /// Agent execution complete.
    Complete {
        files_created: usize,
        files_modified: usize,
    },

    /// Agent execution failed.
    Failed { message: String },
}

impl AgentProgress {
    /// Get a human-readable description of the progress.
    pub fn description(&self) -> String {
        match self {
            Self::LoadingPlan => "Loading plan...".to_string(),
            Self::StartingExecution { total_items } => {
                format!("Starting execution of {} items", total_items)
            }
            Self::GeneratingCode {
                item_index,
                total_items,
                item_description,
            } => {
                format!(
                    "Generating [{}/{}]: {}",
                    item_index + 1,
                    total_items,
                    item_description
                )
            }
            Self::CheckingConformance { item_index } => {
                format!("Checking conformance for item {}", item_index + 1)
            }
            Self::AwaitingReview { item_index } => {
                format!("Awaiting review for item {}", item_index + 1)
            }
            Self::ItemAccepted { item_index } => {
                format!("Item {} accepted", item_index + 1)
            }
            Self::Regenerating { item_index } => {
                format!("Regenerating item {}", item_index + 1)
            }
            Self::RunningTests => "Running tests...".to_string(),
            Self::AwaitingApproval => "Awaiting final approval".to_string(),
            Self::ApplyingChanges => "Applying changes to filesystem...".to_string(),
            Self::Complete {
                files_created,
                files_modified,
            } => {
                format!(
                    "Complete: {} created, {} modified",
                    files_created, files_modified
                )
            }
            Self::Failed { message } => format!("Failed: {}", message),
        }
    }
}

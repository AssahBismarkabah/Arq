/// Progress events during the planning phase.
///
/// These events are sent to the TUI to update progress indicators
/// and provide feedback to the user about what's happening.
#[derive(Debug, Clone)]
pub enum PlanningProgress {
    /// Planning has started
    Started,
    /// Loading the research document
    LoadingResearch,
    /// Generating implementation approaches
    GeneratingApproaches,
    /// Approaches are ready for user selection
    ApproachesReady(usize),
    /// User has selected an approach, generating specification
    GeneratingSpec,
    /// Checking complexity and validating the plan
    CheckingComplexity,
    /// Planning completed successfully
    Complete,
    /// An error occurred
    Error(String),
}

impl PlanningProgress {
    /// Returns a human-readable description of the progress state.
    pub fn description(&self) -> &str {
        match self {
            PlanningProgress::Started => "Starting planning phase...",
            PlanningProgress::LoadingResearch => "Loading research document...",
            PlanningProgress::GeneratingApproaches => "Generating approaches...",
            PlanningProgress::ApproachesReady(_) => "Approaches ready",
            PlanningProgress::GeneratingSpec => "Generating specification...",
            PlanningProgress::CheckingComplexity => "Checking complexity...",
            PlanningProgress::Complete => "Planning complete",
            PlanningProgress::Error(_) => "Error occurred",
        }
    }

    /// Returns true if this is a terminal state (Complete or Error).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PlanningProgress::Complete | PlanningProgress::Error(_)
        )
    }

    /// Returns true if this is an error state.
    pub fn is_error(&self) -> bool {
        matches!(self, PlanningProgress::Error(_))
    }
}

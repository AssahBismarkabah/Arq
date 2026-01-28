use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::phase::Phase;
use crate::planning::Plan;
use crate::research::ResearchDoc;

/// Represents a single task in Arq.
///
/// A task holds the state for one user request, tracking it through
/// the Research → Planning → Agent workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task
    pub id: String,
    /// Human-readable name derived from the prompt
    pub name: String,
    /// The original user prompt describing what they want to do
    pub prompt: String,
    /// Current phase of the task
    pub phase: Phase,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// When the task was last updated
    pub updated_at: DateTime<Utc>,
    /// Research document, populated after Research phase completes
    pub research_doc: Option<ResearchDoc>,
    /// Plan specification, populated after Planning phase completes
    pub plan: Option<Plan>,
}

impl Task {
    /// Creates a new task with the given prompt.
    ///
    /// The task starts in the Research phase.
    pub fn new(prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        let name = Self::derive_name(&prompt);
        let now = Utc::now();

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            prompt,
            phase: Phase::Research,
            created_at: now,
            updated_at: now,
            research_doc: None,
            plan: None,
        }
    }

    /// Derives a task name from the prompt.
    ///
    /// Takes the first few words and converts to kebab-case.
    fn derive_name(prompt: &str) -> String {
        prompt
            .split_whitespace()
            .take(5)
            .collect::<Vec<_>>()
            .join("-")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect()
    }

    /// Attempts to advance to the next phase.
    ///
    /// Returns true if advancement was successful, false if already complete
    /// or if prerequisites are not met.
    pub fn advance_phase(&mut self) -> bool {
        if !self.can_advance() {
            return false;
        }

        if let Some(next) = self.phase.next() {
            self.phase = next;
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Checks if the task can advance to the next phase.
    ///
    /// Each phase has prerequisites:
    /// - Research → Planning: requires research_doc
    /// - Planning → Agent: requires plan
    /// - Agent → Complete: always allowed (completion is handled separately)
    pub fn can_advance(&self) -> bool {
        match self.phase {
            Phase::Research => self.research_doc.is_some(),
            Phase::Planning => self.plan.is_some(),
            Phase::Agent => true,
            Phase::Complete => false,
        }
    }

    /// Sets the research document and validates phase.
    pub fn set_research_doc(&mut self, doc: ResearchDoc) -> Result<(), TaskError> {
        if self.phase != Phase::Research {
            return Err(TaskError::WrongPhase {
                expected: Phase::Research,
                actual: self.phase,
            });
        }
        self.research_doc = Some(doc);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Sets the plan and validates phase.
    pub fn set_plan(&mut self, plan: Plan) -> Result<(), TaskError> {
        if self.phase != Phase::Planning {
            return Err(TaskError::WrongPhase {
                expected: Phase::Planning,
                actual: self.phase,
            });
        }
        self.plan = Some(plan);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Converts the task to a summary (for listings).
    pub fn to_summary(&self) -> TaskSummary {
        TaskSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            phase: self.phase,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// A lightweight summary of a task for listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub name: String,
    pub phase: Phase,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Wrong phase: expected {expected:?}, got {actual:?}")]
    WrongPhase { expected: Phase, actual: Phase },
}


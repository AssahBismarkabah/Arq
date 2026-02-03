//! Event handling for the TUI.

use crossterm::event::{KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;

use arq_core::{
    AgentLoopProgress, AgentProgress, ApproachOptions, ExecutionSummary, GeneratedCode, Plan,
    PlanningProgress, ResearchDoc, ResearchProgress,
};

/// Result of a completed research task.
#[derive(Debug, Clone)]
pub struct ResearchResult {
    /// The task ID for persistence
    pub task_id: String,
    /// The research document
    pub doc: ResearchDoc,
}

/// Result of generated approaches.
#[derive(Debug, Clone)]
pub struct ApproachesResult {
    /// The task ID
    pub task_id: String,
    /// The generated approaches
    pub options: ApproachOptions,
}

/// Result of a completed planning task.
#[derive(Debug, Clone)]
pub struct PlanningResult {
    /// The task ID for persistence
    pub task_id: String,
    /// The generated plan
    pub plan: Plan,
}

/// Result of agent code generation for one item.
#[derive(Debug, Clone)]
pub struct AgentGeneratedResult {
    /// The task ID
    pub task_id: String,
    /// Current item index (0-based)
    pub current_index: usize,
    /// Total number of items
    pub total_items: usize,
    /// The generated code with conformance info
    pub generated: GeneratedCode,
}

/// Result of agent execution completion (all changes applied).
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used for async change application
pub struct AgentCompleteResult {
    /// The task ID
    pub task_id: String,
    /// Summary of changes applied
    pub summary: ExecutionSummary,
}

/// Events that can occur in the application.
#[derive(Debug, Clone)]
pub enum Event {
    /// A key was pressed
    Key(KeyEvent),
    /// A tick occurred (for animations/updates)
    Tick,
    /// A chunk of streaming text arrived
    StreamChunk(String),
    /// Streaming completed
    StreamComplete,
    /// Research progress update
    ResearchProgress(ResearchProgress),
    /// Research completed successfully with full doc
    ResearchComplete(ResearchResult),
    /// Research failed with error message
    ResearchFailed(String),
    /// Planning progress update
    PlanningProgress(PlanningProgress),
    /// Planning approaches generated
    PlanningApproaches(ApproachesResult),
    /// Planning completed successfully with full plan
    PlanningComplete(PlanningResult),
    /// Planning failed with error message
    PlanningFailed(String),
    /// Agent progress update
    AgentProgress(AgentProgress),
    /// Agent generated code for one item - awaiting user review
    AgentGenerated(AgentGeneratedResult),
    /// Agent execution completed (for async change application)
    #[allow(dead_code)]
    AgentComplete(AgentCompleteResult),
    /// Agent execution failed
    AgentFailed(String),
    /// Agent agentic loop progress update
    AgentLoop(AgentLoopProgress),
    /// Agent loop requests tool confirmation
    AgentToolConfirmation {
        tool_name: String,
        args: serde_json::Value,
        request_id: String,
    },
    /// Agent loop completed successfully
    AgentLoopComplete {
        summary: String,
        files_changed: Vec<String>,
    },
    /// Files were successfully rolled back
    FilesRolledBack { count: usize, files: Vec<String> },
    /// Rollback failed with error message
    RollbackFailed(String),
}

/// Handles events from various sources.
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    /// Create a new event handler.
    pub fn new() -> Self {
        let tick_rate = Duration::from_millis(100);
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        // Spawn the event polling task
        tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut interval = tokio::time::interval(tick_rate);

            loop {
                let crossterm_event = reader.next().fuse();
                let tick = interval.tick();

                tokio::select! {
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                if let crossterm::event::Event::Key(key) = evt {
                                    // Only handle key press events, not release
                                    if key.kind == KeyEventKind::Press
                                        && event_tx.send(Event::Key(key)).is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                            Some(Err(_)) => {}
                            None => break,
                        }
                    }
                    _ = tick => {
                        if event_tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Get the next event.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    /// Get the sender for external events (LLM streaming).
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self._tx.clone()
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

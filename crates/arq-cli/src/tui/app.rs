//! Application state and main event loop.

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::io::Stdout;
use tokio::sync::mpsc;

use arq_core::{
    Approach, ApproachOptions, Config, ContextBuilder, FileStorage, KnowledgeGraph, KnowledgeStore,
    Plan, PlanningProgress, PlanningRunner, ResearchDoc, ResearchProgress, ResearchRunner, Task,
    TaskManager,
};

use super::event::{ApproachesResult, Event, EventHandler, PlanningResult, ResearchResult};
use super::ui;

/// The selected tab in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectedTab {
    #[default]
    Researcher,
    Planner,
    Agent,
}

impl SelectedTab {
    pub fn next(self) -> Self {
        match self {
            Self::Researcher => Self::Planner,
            Self::Planner => Self::Agent,
            Self::Agent => Self::Researcher,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Researcher => Self::Agent,
            Self::Planner => Self::Researcher,
            Self::Agent => Self::Planner,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Researcher => "Researcher",
            Self::Planner => "Planner",
            Self::Agent => "Agent",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Researcher => 0,
            Self::Planner => 1,
            Self::Agent => 2,
        }
    }
}

/// Input mode for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

/// Research validation state.
#[derive(Debug, Clone, Default)]
pub enum ResearchState {
    /// No research in progress
    #[default]
    Idle,
    /// Research is running with streaming
    Researching,
    /// Research complete, awaiting user approval or correction
    AwaitingValidation {
        task_id: String,
        pending_doc: ResearchDoc,
    },
    /// Processing user correction
    Refining,
}

/// Planning phase state.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields stored for future display/debugging
pub enum PlanningState {
    /// No planning in progress
    #[default]
    Idle,
    /// Generating implementation approaches
    GeneratingApproaches,
    /// Awaiting user selection of an approach
    AwaitingSelection {
        task_id: String,
        approaches: ApproachOptions,
    },
    /// Generating plan from selected approach
    GeneratingPlan {
        task_id: String,
        selected_approach: Approach,
    },
    /// Awaiting user approval of generated plan
    AwaitingApproval { task_id: String, pending_plan: Plan },
}

/// A chat message in the conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[allow(dead_code)] // Will be used for display
    pub timestamp: DateTime<Utc>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "You",
            Self::Assistant => "Arq",
            Self::System => "System",
        }
    }
}

/// Status of a progress item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStatus {
    #[default]
    Pending,
    InProgress,
    Complete,
    Failed,
}

impl ProgressStatus {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::InProgress => "◐",
            Self::Complete => "●",
            Self::Failed => "✗",
        }
    }
}

/// A progress item in the checklist.
#[derive(Debug, Clone)]
pub struct ProgressItem {
    pub label: String,
    pub status: ProgressStatus,
}

impl ProgressItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: ProgressStatus::Pending,
        }
    }
}

/// Status messages shown while researching.
const THINKING_MESSAGES: &[&str] = &[
    "Thinking...",
    "Analyzing code...",
    "Reading files...",
    "Processing context...",
    "Reasoning...",
    "Understanding patterns...",
    "Connecting dots...",
    "Almost there...",
];

/// Main application state.
pub struct App {
    /// Currently selected tab
    pub selected_tab: SelectedTab,
    /// Current input mode
    pub input_mode: InputMode,
    /// Chat messages for Researcher tab
    pub researcher_messages: Vec<ChatMessage>,
    /// Chat messages for Planner tab
    pub planner_messages: Vec<ChatMessage>,
    /// Chat messages for Agent tab
    pub agent_messages: Vec<ChatMessage>,
    /// Input buffer for user typing
    pub input_buffer: String,
    /// Progress items for current operation
    pub progress_items: Vec<ProgressItem>,
    /// Whether streaming is active
    pub is_streaming: bool,
    /// Current streaming buffer
    pub stream_buffer: String,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Scroll offset for chat (per tab)
    pub scroll_offsets: [usize; 3],
    /// Configuration
    pub config: Config,
    /// Task manager for persistence
    pub manager: TaskManager<FileStorage>,
    /// Current task
    pub current_task: Option<Task>,
    /// Status message
    pub status_message: Option<String>,
    /// Research validation state
    pub research_state: ResearchState,
    /// Planning phase state
    pub planning_state: PlanningState,
    /// Index of currently selected model in available_models
    pub selected_model_index: usize,
    /// Tick counter for cycling messages
    pub tick_count: usize,
    /// Knowledge graph for semantic search (initialized lazily, for future TUI integration)
    #[allow(dead_code)]
    pub knowledge_graph: Option<std::sync::Arc<KnowledgeGraph>>,
}

impl App {
    /// Create a new app instance.
    ///
    /// If `restore_state` is true, restores the previous task context and shows
    /// relevant messages. If false, starts with a clean welcome screen.
    pub fn new(config: Config, manager: TaskManager<FileStorage>, restore_state: bool) -> Self {
        let current_task = if restore_state {
            manager.get_current_task().ok().flatten()
        } else {
            None
        };

        // Find the index of the current model in available_models
        let selected_model_index = if !config.llm.available_models.is_empty() {
            let current_model = config.llm.model_or_default();
            config
                .llm
                .available_models
                .iter()
                .position(|m| m == &current_model)
                .unwrap_or(0)
        } else {
            0
        };

        let mut app = Self {
            selected_tab: SelectedTab::Researcher,
            input_mode: InputMode::Normal,
            researcher_messages: Vec::new(),
            planner_messages: Vec::new(),
            agent_messages: Vec::new(),
            input_buffer: String::new(),
            progress_items: Vec::new(),
            is_streaming: false,
            stream_buffer: String::new(),
            should_quit: false,
            scroll_offsets: [0; 3],
            config,
            manager,
            current_task: current_task.clone(),
            status_message: None,
            research_state: ResearchState::Idle,
            planning_state: PlanningState::Idle,
            selected_model_index,
            tick_count: 0,
            knowledge_graph: None, // Initialized lazily during first research
        };

        // Restore state from current task
        if let Some(ref task) = current_task {
            // Add welcome message to researcher tab
            app.researcher_messages.push(ChatMessage::system(format!(
                "Current task: {} ({})",
                task.name,
                task.phase.display_name()
            )));

            // Restore tab and state based on task phase
            match task.phase {
                arq_core::Phase::Research => {
                    app.selected_tab = SelectedTab::Researcher;
                    if let Some(ref doc) = task.research_doc {
                        // Research is complete, show summary
                        app.researcher_messages.push(ChatMessage::assistant(format!(
                            "**Previous Research Summary:**\n{}\n\n**Suggested Approach:**\n{}",
                            doc.summary, doc.suggested_approach
                        )));
                        app.researcher_messages.push(ChatMessage::system(
                            "Research complete. Press Tab to switch to Planner tab.",
                        ));
                    }
                }
                arq_core::Phase::Planning => {
                    app.selected_tab = SelectedTab::Planner;
                    // Show research summary in researcher tab
                    if let Some(ref doc) = task.research_doc {
                        app.researcher_messages.push(ChatMessage::assistant(format!(
                            "**Research Summary:**\n{}",
                            doc.summary
                        )));
                        app.researcher_messages
                            .push(ChatMessage::system("Research complete."));
                    }
                    // Show plan or prompt in planner tab
                    if let Some(ref plan) = task.plan {
                        if let Ok(yaml) = plan.to_yaml() {
                            app.planner_messages.push(ChatMessage::assistant(format!(
                                "**Saved Plan:**\n```yaml\n{}\n```",
                                yaml
                            )));
                        }
                        app.planner_messages.push(ChatMessage::system(
                            "Plan complete. Press Tab to switch to Agent tab.",
                        ));
                    } else {
                        app.planner_messages.push(ChatMessage::system(
                            "Press [i] then Enter to generate implementation approaches.",
                        ));
                    }
                }
                arq_core::Phase::Agent => {
                    app.selected_tab = SelectedTab::Agent;
                    app.agent_messages.push(ChatMessage::system(
                        "Agent phase ready. (Implementation pending)",
                    ));
                }
                arq_core::Phase::Complete => {
                    app.selected_tab = SelectedTab::Agent;
                    app.agent_messages
                        .push(ChatMessage::system("Task complete! All phases finished."));
                }
            }
        } else {
            app.researcher_messages.push(ChatMessage::system(
                "Welcome to Arq! No active task. Type a prompt to start research.",
            ));
        }

        // Initialize progress items for current tab
        app.reset_progress_items();

        app
    }

    /// Reset progress items based on current tab.
    fn reset_progress_items(&mut self) {
        self.progress_items = match self.selected_tab {
            SelectedTab::Researcher => vec![
                ProgressItem::new("Gathering context"),
                ProgressItem::new("Searching knowledge graph"),
                ProgressItem::new("Calling LLM"),
                ProgressItem::new("Parsing response"),
                ProgressItem::new("Saving research doc"),
            ],
            SelectedTab::Planner => vec![
                ProgressItem::new("Loading research"),
                ProgressItem::new("Generating approaches"),
                ProgressItem::new("Building specification"),
                ProgressItem::new("Checking complexity"),
            ],
            SelectedTab::Agent => vec![
                ProgressItem::new("Loading plan"),
                ProgressItem::new("Generating code"),
                ProgressItem::new("Checking conformance"),
                ProgressItem::new("Running tests"),
            ],
        };
    }

    /// Run the main event loop.
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut events = EventHandler::new();

        loop {
            // Draw UI
            terminal.draw(|frame| ui::render(self, frame))?;

            // Handle events
            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => self.handle_key_event(key, events.sender()),
                    Event::Tick => {
                        // Update cycling status messages during research
                        self.tick_count = self.tick_count.wrapping_add(1);
                        if matches!(
                            self.research_state,
                            ResearchState::Researching | ResearchState::Refining
                        ) {
                            // Cycle message every ~8 ticks (about 2 seconds at 250ms tick rate)
                            let msg_index = (self.tick_count / 8) % THINKING_MESSAGES.len();
                            self.status_message = Some(THINKING_MESSAGES[msg_index].to_string());
                        }
                    }
                    Event::StreamChunk(text) => {
                        self.stream_buffer.push_str(&text);
                    }
                    Event::StreamComplete => {
                        if !self.stream_buffer.is_empty() {
                            let content = std::mem::take(&mut self.stream_buffer);
                            self.chat_messages_mut()
                                .push(ChatMessage::assistant(content));
                        }
                        self.is_streaming = false;
                    }
                    Event::ResearchProgress(progress) => {
                        self.handle_research_progress(progress);
                    }
                    Event::ResearchComplete(result) => {
                        self.handle_research_complete(result);
                    }
                    Event::ResearchFailed(error) => {
                        self.handle_research_failed(error);
                    }
                    Event::PlanningProgress(progress) => {
                        self.handle_planning_progress(progress);
                    }
                    Event::PlanningApproaches(result) => {
                        self.handle_planning_approaches(result);
                    }
                    Event::PlanningComplete(result) => {
                        self.handle_planning_complete(result);
                    }
                    Event::PlanningFailed(error) => {
                        self.handle_planning_failed(error);
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle research progress updates.
    fn handle_research_progress(&mut self, progress: ResearchProgress) {
        match progress {
            ResearchProgress::Started => {
                self.set_progress_status(0, ProgressStatus::InProgress);
            }
            ResearchProgress::GatheringContext => {
                self.set_progress_status(0, ProgressStatus::InProgress);
            }
            ResearchProgress::SearchingKnowledgeGraph => {
                self.set_progress_status(0, ProgressStatus::Complete);
                self.set_progress_status(1, ProgressStatus::InProgress);
            }
            ResearchProgress::KnowledgeGraphResults { count } => {
                self.set_progress_status(1, ProgressStatus::Complete);
                self.status_message = Some(format!("Found {} relevant code segments", count));
            }
            ResearchProgress::CallingLLM => {
                // Mark context gathering complete (in case we skipped knowledge graph)
                self.set_progress_status(0, ProgressStatus::Complete);
                self.set_progress_status(1, ProgressStatus::Complete);
                self.set_progress_status(2, ProgressStatus::InProgress);
            }
            ResearchProgress::ParsingResponse => {
                self.set_progress_status(2, ProgressStatus::Complete);
                self.set_progress_status(3, ProgressStatus::InProgress);
            }
            ResearchProgress::Complete => {
                self.set_progress_status(3, ProgressStatus::Complete);
                self.set_progress_status(4, ProgressStatus::Complete);
            }
            ResearchProgress::Error(msg) => {
                self.chat_messages_mut()
                    .push(ChatMessage::system(format!("Error: {}", msg)));
                // Mark current item as failed
                for item in &mut self.progress_items {
                    if item.status == ProgressStatus::InProgress {
                        item.status = ProgressStatus::Failed;
                        break;
                    }
                }
            }
        }
    }

    /// Set progress status for item at index.
    fn set_progress_status(&mut self, index: usize, status: ProgressStatus) {
        if let Some(item) = self.progress_items.get_mut(index) {
            item.status = status;
        }
    }

    /// Handle research completion - await user validation before saving.
    fn handle_research_complete(&mut self, result: ResearchResult) {
        self.is_streaming = false;

        // Use the document's built-in markdown formatting for complete display
        let content = result.doc.to_markdown();
        self.chat_messages_mut()
            .push(ChatMessage::assistant(&content));

        // Set awaiting validation state (DON'T save yet - wait for approval)
        self.research_state = ResearchState::AwaitingValidation {
            task_id: result.task_id,
            pending_doc: result.doc,
        };

        // Prompt user for validation
        self.chat_messages_mut().push(ChatMessage::system(
            "Is this understanding correct?\n\
             Press [a] to approve and save, or type corrections.",
        ));
        self.status_message =
            Some("Awaiting approval... [a] approve, [i] type corrections".to_string());
    }

    /// Handle research failure.
    fn handle_research_failed(&mut self, error: String) {
        self.is_streaming = false;
        self.research_state = ResearchState::Idle;
        self.chat_messages_mut()
            .push(ChatMessage::system(format!("Research failed: {}", error)));

        // Mark progress as failed
        for item in &mut self.progress_items {
            if item.status == ProgressStatus::InProgress {
                item.status = ProgressStatus::Failed;
                break;
            }
        }
    }

    /// Handle planning progress updates.
    fn handle_planning_progress(&mut self, progress: PlanningProgress) {
        match progress {
            PlanningProgress::Started => {
                self.set_progress_status(0, ProgressStatus::InProgress);
            }
            PlanningProgress::LoadingResearch => {
                self.set_progress_status(0, ProgressStatus::InProgress);
            }
            PlanningProgress::GeneratingApproaches => {
                self.set_progress_status(0, ProgressStatus::Complete);
                self.set_progress_status(1, ProgressStatus::InProgress);
            }
            PlanningProgress::ApproachesReady(count) => {
                self.set_progress_status(1, ProgressStatus::Complete);
                self.status_message = Some(format!("Generated {} approaches", count));
            }
            PlanningProgress::GeneratingSpec => {
                self.set_progress_status(2, ProgressStatus::InProgress);
            }
            PlanningProgress::CheckingComplexity => {
                self.set_progress_status(2, ProgressStatus::Complete);
                self.set_progress_status(3, ProgressStatus::InProgress);
            }
            PlanningProgress::Complete => {
                self.set_progress_status(3, ProgressStatus::Complete);
            }
            PlanningProgress::Error(msg) => {
                self.chat_messages_mut()
                    .push(ChatMessage::system(format!("Planning error: {}", msg)));
                for item in &mut self.progress_items {
                    if item.status == ProgressStatus::InProgress {
                        item.status = ProgressStatus::Failed;
                        break;
                    }
                }
            }
        }
    }

    /// Handle approaches generated.
    fn handle_planning_approaches(&mut self, result: ApproachesResult) {
        self.is_streaming = false;

        // Display approaches for user selection
        let content = result.options.to_display_string();
        self.chat_messages_mut()
            .push(ChatMessage::assistant(&content));

        // Add instruction
        self.chat_messages_mut().push(ChatMessage::system(
            "Press [i] to edit, type 1/2/3 (or custom approach), then Enter.",
        ));

        // Set awaiting selection state
        self.planning_state = PlanningState::AwaitingSelection {
            task_id: result.task_id,
            approaches: result.options,
        };

        self.status_message = Some("[i] Edit → type 1/2/3 → Enter to select".to_string());
    }

    /// Handle planning complete.
    fn handle_planning_complete(&mut self, result: PlanningResult) {
        self.is_streaming = false;

        // Display plan for approval
        let content = match result.plan.to_yaml() {
            Ok(yaml) => format!("## Generated Plan\n\n```yaml\n{}\n```", yaml),
            Err(_) => format!("{:?}", result.plan),
        };
        self.chat_messages_mut()
            .push(ChatMessage::assistant(&content));

        // Set awaiting approval state
        self.planning_state = PlanningState::AwaitingApproval {
            task_id: result.task_id,
            pending_plan: result.plan,
        };

        self.chat_messages_mut().push(ChatMessage::system(
            "Press [a] to approve and save the plan, or type refinements.",
        ));
        self.status_message =
            Some("Awaiting approval... [a] approve, [i] type refinements".to_string());
    }

    /// Handle planning failure.
    fn handle_planning_failed(&mut self, error: String) {
        self.is_streaming = false;
        self.planning_state = PlanningState::Idle;
        self.chat_messages_mut()
            .push(ChatMessage::system(format!("Planning failed: {}", error)));

        for item in &mut self.progress_items {
            if item.status == ProgressStatus::InProgress {
                item.status = ProgressStatus::Failed;
                break;
            }
        }
    }

    /// Approve plan and save - called when user presses 'a' during approval.
    fn approve_plan(&mut self, task_id: String, plan: Plan) {
        // Auto-advance to Planning phase if still in Research phase
        if let Some(ref task) = self.current_task {
            if task.phase == arq_core::Phase::Research && task.research_doc.is_some() {
                if let Err(e) = self.manager.advance_phase(&task_id) {
                    self.chat_messages_mut().push(ChatMessage::system(format!(
                        "Failed to advance to Planning phase: {}",
                        e
                    )));
                    self.planning_state = PlanningState::AwaitingApproval {
                        task_id,
                        pending_plan: plan,
                    };
                    return;
                }
            }
        }

        match self.manager.set_plan(&task_id, plan.clone()) {
            Ok(task) => {
                self.current_task = Some(task);
                self.status_message = Some("Plan saved to .arq/plan.yaml".to_string());
                self.chat_messages_mut().push(ChatMessage::system(
                    "Plan approved and saved. You can now proceed to Agent tab.",
                ));
                self.set_progress_status(3, ProgressStatus::Complete);
                self.planning_state = PlanningState::Idle;
            }
            Err(e) => {
                self.chat_messages_mut()
                    .push(ChatMessage::system(format!("Failed to save plan: {}", e)));
                // Restore state for retry
                self.planning_state = PlanningState::AwaitingApproval {
                    task_id,
                    pending_plan: plan,
                };
            }
        }
    }

    /// Approve research and save - called when user presses 'a' during validation.
    fn approve_research(&mut self, task_id: String, doc: ResearchDoc) {
        match self.manager.set_research_doc(&task_id, doc.clone()) {
            Ok(task) => {
                self.current_task = Some(task);
                self.status_message = Some("Research saved to .arq/research-doc.md".to_string());
                self.chat_messages_mut().push(ChatMessage::system(
                    "Research approved and saved. You can now proceed to Planner tab.",
                ));
                // Mark final progress item complete
                self.set_progress_status(4, ProgressStatus::Complete);
            }
            Err(e) => {
                self.chat_messages_mut().push(ChatMessage::system(format!(
                    "Failed to save research: {}",
                    e
                )));
                // Restore state for retry
                self.research_state = ResearchState::AwaitingValidation {
                    task_id,
                    pending_doc: doc,
                };
            }
        }
    }

    /// Handle a key event.
    fn handle_key_event(&mut self, key: KeyEvent, event_tx: mpsc::UnboundedSender<Event>) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode_key(key),
            InputMode::Editing => self.handle_editing_mode_key(key, event_tx),
        }
    }

    /// Handle key in normal mode.
    fn handle_normal_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Tab | KeyCode::Right => {
                let next = self.selected_tab.next();
                if self.can_switch_to_tab(&next) {
                    self.selected_tab = next;
                    self.reset_progress_items();
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                let prev = self.selected_tab.previous();
                if self.can_switch_to_tab(&prev) {
                    self.selected_tab = prev;
                    self.reset_progress_items();
                }
            }
            KeyCode::Char('i') | KeyCode::Enter => {
                self.input_mode = InputMode::Editing;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up();
            }
            KeyCode::Char('a') => {
                // Approve research if awaiting validation
                if let ResearchState::AwaitingValidation {
                    task_id,
                    pending_doc,
                } = std::mem::replace(&mut self.research_state, ResearchState::Idle)
                {
                    self.approve_research(task_id, pending_doc);
                }
                // Approve plan if awaiting approval
                else if let PlanningState::AwaitingApproval {
                    task_id,
                    pending_plan,
                } = std::mem::replace(&mut self.planning_state, PlanningState::Idle)
                {
                    self.approve_plan(task_id, pending_plan);
                }
            }
            KeyCode::Char('m') => {
                // Cycle through available models
                self.cycle_model();
            }
            _ => {}
        }
    }

    /// Handle key in editing mode.
    fn handle_editing_mode_key(&mut self, key: KeyEvent, event_tx: mpsc::UnboundedSender<Event>) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                self.submit_input(event_tx);
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }
    }

    /// Submit the current input.
    fn submit_input(&mut self, event_tx: mpsc::UnboundedSender<Event>) {
        if self.input_buffer.is_empty() || self.is_streaming {
            return;
        }

        let input = std::mem::take(&mut self.input_buffer);
        self.chat_messages_mut().push(ChatMessage::user(&input));

        match self.selected_tab {
            SelectedTab::Researcher => {
                // Check current state to decide action
                match &self.research_state {
                    ResearchState::Idle => {
                        // New research
                        self.start_research(input, event_tx);
                    }
                    ResearchState::AwaitingValidation { .. } => {
                        // User is providing correction - extract values and refine
                        if let ResearchState::AwaitingValidation {
                            task_id,
                            pending_doc,
                        } = std::mem::replace(&mut self.research_state, ResearchState::Refining)
                        {
                            self.refine_research(task_id, pending_doc, input, event_tx);
                        }
                    }
                    ResearchState::Researching | ResearchState::Refining => {
                        // Already streaming, ignore input
                    }
                }
            }
            SelectedTab::Planner => {
                match &self.planning_state {
                    PlanningState::Idle => {
                        // Start generating approaches
                        self.start_planning(event_tx);
                    }
                    PlanningState::AwaitingSelection { .. } => {
                        // User typed a number (1, 2, 3) or custom approach
                        if let Ok(idx) = input.parse::<usize>() {
                            if idx > 0 {
                                self.select_approach(idx - 1, event_tx);
                            }
                        } else {
                            // Custom approach description - create custom approach
                            self.create_custom_approach(input, event_tx);
                        }
                    }
                    PlanningState::AwaitingApproval { .. } => {
                        // User is providing refinement feedback
                        self.refine_plan(input, event_tx);
                    }
                    PlanningState::GeneratingApproaches | PlanningState::GeneratingPlan { .. } => {
                        // Already generating, ignore input
                    }
                }
            }
            SelectedTab::Agent => {
                self.chat_messages_mut()
                    .push(ChatMessage::system("Agent phase not yet implemented."));
            }
        }

        self.input_mode = InputMode::Normal;
    }

    /// Start a research task with streaming.
    fn start_research(&mut self, prompt: String, event_tx: mpsc::UnboundedSender<Event>) {
        self.is_streaming = true;
        self.stream_buffer.clear();
        self.reset_progress_items();
        self.status_message = Some("Starting research...".to_string());

        // Create task via manager (persists immediately)
        let task = match self.manager.create_task(&prompt) {
            Ok(task) => task,
            Err(e) => {
                self.chat_messages_mut()
                    .push(ChatMessage::system(format!("Failed to create task: {}", e)));
                self.is_streaming = false;
                return;
            }
        };

        let task_id = task.id.clone();
        self.current_task = Some(task.clone());

        // Get config values we need
        let config = self.config.clone();

        // Get the knowledge graph db path for semantic search
        let kg_db_path = config.knowledge.db_full_path(&config.storage);

        // Add message to show research is starting
        self.chat_messages_mut().push(ChatMessage::system(format!(
            "Researching: {} ...",
            task.prompt
        )));

        // Spawn the research task
        tokio::spawn(async move {
            match run_research_task(task, config, kg_db_path, event_tx.clone()).await {
                Ok(doc) => {
                    let _ = event_tx.send(Event::ResearchComplete(ResearchResult { task_id, doc }));
                }
                Err(error) => {
                    let _ = event_tx.send(Event::ResearchFailed(error));
                }
            }
        });

        // Set state to Researching
        self.research_state = ResearchState::Researching;
    }

    /// Refine research based on user correction.
    fn refine_research(
        &mut self,
        task_id: String,
        original_doc: ResearchDoc,
        correction: String,
        event_tx: mpsc::UnboundedSender<Event>,
    ) {
        self.is_streaming = true;
        self.stream_buffer.clear();
        self.reset_progress_items();
        self.research_state = ResearchState::Refining;

        // Build refinement prompt that includes original findings + correction
        let refinement_prompt = format!(
            "Previous research findings:\n\n## Summary\n{}\n\n## Suggested Approach\n{}\n\n---\n\n\
             User correction/feedback:\n{}\n\n\
             Please update the research based on this feedback. \
             Address the user's concerns and provide corrected findings.",
            original_doc.summary, original_doc.suggested_approach, correction
        );

        // Create a temporary task for the refinement (uses refinement prompt)
        let task = Task::new(&refinement_prompt);

        let config = self.config.clone();
        let task_id_clone = task_id.clone();
        let kg_db_path = config.knowledge.db_full_path(&config.storage);

        // Spawn the refinement task (reuses run_research_task)
        tokio::spawn(async move {
            match run_research_task(task, config, kg_db_path, event_tx.clone()).await {
                Ok(doc) => {
                    // Return with original task_id so we save to the right task
                    let _ = event_tx.send(Event::ResearchComplete(ResearchResult {
                        task_id: task_id_clone,
                        doc,
                    }));
                }
                Err(error) => {
                    let _ = event_tx.send(Event::ResearchFailed(error));
                }
            }
        });
    }

    /// Start planning phase - generate approaches from research.
    fn start_planning(&mut self, event_tx: mpsc::UnboundedSender<Event>) {
        // Get current task and research doc
        let task = match &self.current_task {
            Some(t) => t.clone(),
            None => {
                self.chat_messages_mut()
                    .push(ChatMessage::system("No current task. Create a task first."));
                return;
            }
        };

        let research_doc = match &task.research_doc {
            Some(doc) => doc.clone(),
            None => {
                self.chat_messages_mut().push(ChatMessage::system(
                    "No research document. Complete research first.",
                ));
                return;
            }
        };

        self.is_streaming = true;
        self.stream_buffer.clear();
        self.reset_progress_items();
        self.planning_state = PlanningState::GeneratingApproaches;
        self.status_message = Some("Generating approaches...".to_string());

        let task_id = task.id.clone();
        let config = self.config.clone();

        self.chat_messages_mut().push(ChatMessage::system(
            "Generating implementation approaches based on your research...",
        ));

        // Spawn the planning task
        tokio::spawn(async move {
            match run_planning_approaches(research_doc, config, event_tx.clone()).await {
                Ok(options) => {
                    let _ = event_tx.send(Event::PlanningApproaches(ApproachesResult {
                        task_id,
                        options,
                    }));
                }
                Err(error) => {
                    let _ = event_tx.send(Event::PlanningFailed(error));
                }
            }
        });
    }

    /// Select an approach and generate detailed plan.
    fn select_approach(&mut self, index: usize, event_tx: mpsc::UnboundedSender<Event>) {
        // Extract state values
        let (task_id, approaches) = if let PlanningState::AwaitingSelection {
            task_id,
            approaches,
        } =
            std::mem::replace(&mut self.planning_state, PlanningState::Idle)
        {
            (task_id, approaches)
        } else {
            return;
        };

        // Get the selected approach
        let approach = match approaches.get(index) {
            Some(a) => a.clone(),
            None => {
                self.chat_messages_mut().push(ChatMessage::system(format!(
                    "Invalid selection. Please choose 1-{}.",
                    approaches.len()
                )));
                // Restore state
                self.planning_state = PlanningState::AwaitingSelection {
                    task_id,
                    approaches,
                };
                return;
            }
        };

        // Get research doc from current task
        let research_doc = match &self.current_task {
            Some(t) => match &t.research_doc {
                Some(doc) => doc.clone(),
                None => {
                    self.chat_messages_mut()
                        .push(ChatMessage::system("Research document not found."));
                    return;
                }
            },
            None => {
                self.chat_messages_mut()
                    .push(ChatMessage::system("No current task."));
                return;
            }
        };

        self.is_streaming = true;
        self.stream_buffer.clear();
        self.planning_state = PlanningState::GeneratingPlan {
            task_id: task_id.clone(),
            selected_approach: approach.clone(),
        };
        self.status_message = Some("Generating detailed plan...".to_string());

        self.chat_messages_mut().push(ChatMessage::system(format!(
            "Selected: {}. Generating detailed specification...",
            approach.name
        )));

        let config = self.config.clone();

        // Spawn the plan generation task
        tokio::spawn(async move {
            match run_planning_spec(research_doc, approach, config, event_tx.clone()).await {
                Ok(plan) => {
                    let _ =
                        event_tx.send(Event::PlanningComplete(PlanningResult { task_id, plan }));
                }
                Err(error) => {
                    let _ = event_tx.send(Event::PlanningFailed(error));
                }
            }
        });
    }

    /// Create a custom approach from user description.
    fn create_custom_approach(
        &mut self,
        description: String,
        event_tx: mpsc::UnboundedSender<Event>,
    ) {
        // Extract state values
        let (task_id, _approaches) = if let PlanningState::AwaitingSelection {
            task_id,
            approaches,
        } =
            std::mem::replace(&mut self.planning_state, PlanningState::Idle)
        {
            (task_id, approaches)
        } else {
            return;
        };

        // Get research doc from current task
        let research_doc = match &self.current_task {
            Some(t) => match &t.research_doc {
                Some(doc) => doc.clone(),
                None => {
                    self.chat_messages_mut()
                        .push(ChatMessage::system("Research document not found."));
                    return;
                }
            },
            None => {
                self.chat_messages_mut()
                    .push(ChatMessage::system("No current task."));
                return;
            }
        };

        // Create a custom approach from user input
        let approach = Approach::new("custom", "Custom Approach")
            .with_description(&description)
            .with_complexity(arq_core::planning::Complexity::Medium);

        self.is_streaming = true;
        self.stream_buffer.clear();
        self.planning_state = PlanningState::GeneratingPlan {
            task_id: task_id.clone(),
            selected_approach: approach.clone(),
        };
        self.status_message = Some("Generating plan for custom approach...".to_string());

        self.chat_messages_mut().push(ChatMessage::system(
            "Generating plan for your custom approach...",
        ));

        let config = self.config.clone();

        // Spawn the plan generation task
        tokio::spawn(async move {
            match run_planning_spec(research_doc, approach, config, event_tx.clone()).await {
                Ok(plan) => {
                    let _ =
                        event_tx.send(Event::PlanningComplete(PlanningResult { task_id, plan }));
                }
                Err(error) => {
                    let _ = event_tx.send(Event::PlanningFailed(error));
                }
            }
        });
    }

    /// Refine plan based on user feedback.
    fn refine_plan(&mut self, feedback: String, event_tx: mpsc::UnboundedSender<Event>) {
        // For now, regenerate plan with the feedback included in the prompt
        // Extract state values
        let (task_id, _pending_plan) = if let PlanningState::AwaitingApproval {
            task_id,
            pending_plan,
        } =
            std::mem::replace(&mut self.planning_state, PlanningState::Idle)
        {
            (task_id, pending_plan)
        } else {
            return;
        };

        // Get research doc from current task
        let research_doc = match &self.current_task {
            Some(t) => match &t.research_doc {
                Some(doc) => doc.clone(),
                None => {
                    self.chat_messages_mut()
                        .push(ChatMessage::system("Research document not found."));
                    return;
                }
            },
            None => {
                self.chat_messages_mut()
                    .push(ChatMessage::system("No current task."));
                return;
            }
        };

        // Create a refinement approach with the feedback
        let approach = Approach::new("refinement", "Refined Approach")
            .with_description(format!(
                "Based on user feedback: {}\n\nPlease regenerate the plan addressing these concerns.",
                feedback
            ))
            .with_complexity(arq_core::planning::Complexity::Medium);

        self.is_streaming = true;
        self.stream_buffer.clear();
        self.planning_state = PlanningState::GeneratingPlan {
            task_id: task_id.clone(),
            selected_approach: approach.clone(),
        };
        self.status_message = Some("Refining plan...".to_string());

        self.chat_messages_mut().push(ChatMessage::system(
            "Regenerating plan based on your feedback...",
        ));

        let config = self.config.clone();

        // Spawn the plan generation task
        tokio::spawn(async move {
            match run_planning_spec(research_doc, approach, config, event_tx.clone()).await {
                Ok(plan) => {
                    let _ =
                        event_tx.send(Event::PlanningComplete(PlanningResult { task_id, plan }));
                }
                Err(error) => {
                    let _ = event_tx.send(Event::PlanningFailed(error));
                }
            }
        });
    }

    /// Check if we can switch to the given tab.
    /// Gates access: Planner requires saved research, Agent requires saved plan.
    fn can_switch_to_tab(&mut self, tab: &SelectedTab) -> bool {
        match tab {
            SelectedTab::Researcher => true, // Always accessible
            SelectedTab::Planner => {
                // Requires saved research document
                let has_research = self
                    .current_task
                    .as_ref()
                    .map(|t| t.research_doc.is_some())
                    .unwrap_or(false);
                if !has_research {
                    self.status_message = Some("Complete and approve research first".to_string());
                    return false;
                }
                true
            }
            SelectedTab::Agent => {
                // Requires saved plan (not implemented yet, allow for now)
                let has_plan = self
                    .current_task
                    .as_ref()
                    .map(|t| t.plan.is_some())
                    .unwrap_or(false);
                if !has_plan {
                    self.status_message = Some("Complete planning first".to_string());
                    return false;
                }
                true
            }
        }
    }

    /// Scroll chat up.
    fn scroll_up(&mut self) {
        let offset = self.scroll_offset().saturating_add(1);
        self.set_scroll_offset(offset);
    }

    /// Scroll chat down.
    fn scroll_down(&mut self) {
        let offset = self.scroll_offset().saturating_sub(1);
        self.set_scroll_offset(offset);
    }

    /// Cycle through available models.
    fn cycle_model(&mut self) {
        let models = &self.config.llm.available_models;
        if models.is_empty() {
            self.status_message = Some("No models configured in available_models".to_string());
            return;
        }

        // Cycle to next model
        self.selected_model_index = (self.selected_model_index + 1) % models.len();
        let new_model = models[self.selected_model_index].clone();

        // Update config
        self.config.llm.model = Some(new_model.clone());

        self.status_message = Some(format!(
            "Model: {} ({}/{})",
            new_model,
            self.selected_model_index + 1,
            models.len()
        ));
    }

    /// Get the current model name for display.
    pub fn current_model(&self) -> String {
        self.config.llm.model_or_default()
    }

    /// Get chat messages for the current tab.
    pub fn chat_messages(&self) -> &Vec<ChatMessage> {
        match self.selected_tab {
            SelectedTab::Researcher => &self.researcher_messages,
            SelectedTab::Planner => &self.planner_messages,
            SelectedTab::Agent => &self.agent_messages,
        }
    }

    /// Get mutable chat messages for the current tab.
    pub fn chat_messages_mut(&mut self) -> &mut Vec<ChatMessage> {
        match self.selected_tab {
            SelectedTab::Researcher => &mut self.researcher_messages,
            SelectedTab::Planner => &mut self.planner_messages,
            SelectedTab::Agent => &mut self.agent_messages,
        }
    }

    /// Get scroll offset for the current tab.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offsets[self.selected_tab.index()]
    }

    /// Set scroll offset for the current tab.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offsets[self.selected_tab.index()] = offset;
    }
}

/// Run a research task with streaming and progress updates.
/// Returns the full ResearchDoc for persistence.
async fn run_research_task(
    task: Task,
    config: Config,
    kg_db_path: std::path::PathBuf,
    event_tx: mpsc::UnboundedSender<Event>,
) -> Result<arq_core::ResearchDoc, String> {
    use arq_core::{ClaudeClient, OpenAIClient, StreamChunk};
    use std::env;
    use std::sync::Arc;

    // Create context builder with config
    let cwd = env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
    let context_builder = ContextBuilder::with_config(cwd.clone(), config.context.clone());

    // Try to initialize knowledge graph for semantic search
    let knowledge_store: Option<Arc<dyn KnowledgeStore>> =
        match KnowledgeGraph::open(&kg_db_path).await {
            Ok(kg) => {
                // Check if initialized, if not initialize and index
                let kg = Arc::new(kg);
                if !kg.is_initialized().await.unwrap_or(false) {
                    if let Err(e) = kg.initialize().await {
                        eprintln!("Failed to initialize knowledge graph: {}", e);
                        None
                    } else {
                        // Index the codebase on first run
                        let _ = event_tx.send(Event::ResearchProgress(
                            ResearchProgress::SearchingKnowledgeGraph,
                        ));
                        if let Err(e) = kg.index_directory(&cwd).await {
                            eprintln!("Failed to index codebase: {}", e);
                        }
                        Some(kg as Arc<dyn KnowledgeStore>)
                    }
                } else {
                    Some(kg as Arc<dyn KnowledgeStore>)
                }
            }
            Err(e) => {
                eprintln!("Failed to open knowledge graph: {}", e);
                None
            }
        };

    // Create channels for progress and streaming
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ResearchProgress>();
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<StreamChunk>();

    // Forward progress events to TUI
    let event_tx_progress = event_tx.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = event_tx_progress.send(Event::ResearchProgress(progress));
        }
    });

    // Forward stream chunks to TUI
    let event_tx_stream = event_tx.clone();
    tokio::spawn(async move {
        while let Some(chunk) = stream_rx.recv().await {
            if chunk.is_final {
                let _ = event_tx_stream.send(Event::StreamComplete);
            } else {
                let _ = event_tx_stream.send(Event::StreamChunk(chunk.text));
            }
        }
    });

    // Run research based on provider type
    // (ResearchRunner is generic, so we handle each provider type separately)
    let provider = config.llm.provider.as_str();
    let model = config.llm.model_or_default();

    // Helper macro to create runner with or without knowledge store
    macro_rules! create_runner {
        ($client:expr) => {
            if let Some(ref kg) = knowledge_store {
                ResearchRunner::with_knowledge_store(
                    $client,
                    context_builder.clone(),
                    Arc::clone(kg),
                )
            } else {
                ResearchRunner::new($client, context_builder.clone())
            }
        };
    }

    let doc = match provider {
        "anthropic" | "claude" => {
            let api_key = config
                .llm
                .api_key_or_env()
                .ok_or_else(|| "ANTHROPIC_API_KEY not set".to_string())?;
            let client = ClaudeClient::new(api_key).with_model(&model);
            let runner = create_runner!(client);
            runner
                .run_streaming(&task, progress_tx, stream_tx)
                .await
                .map_err(|e| format!("Research failed: {}", e))?
        }
        "ollama" => {
            let base_url = config.llm.base_url_or_default();
            let client = OpenAIClient::new(&base_url, "", &model);
            let runner = create_runner!(client);
            runner
                .run_streaming(&task, progress_tx, stream_tx)
                .await
                .map_err(|e| format!("Research failed: {}", e))?
        }
        _ => {
            // OpenAI or OpenAI-compatible (use non-streaming for compatibility)
            let base_url = config.llm.base_url_or_default();
            let api_key = config.llm.api_key_or_env().unwrap_or_default();
            let client = OpenAIClient::new(&base_url, &api_key, &model);
            let runner = create_runner!(client);
            // Use non-streaming for better compatibility with various providers
            let doc = runner
                .run_with_progress(&task, progress_tx)
                .await
                .map_err(|e| format!("Research failed: {}", e))?;
            // Send done signal to stream
            let _ = stream_tx.send(StreamChunk::done());
            doc
        }
    };

    Ok(doc)
}

/// Run planning approaches generation.
/// Returns ApproachOptions for user selection.
async fn run_planning_approaches(
    research_doc: ResearchDoc,
    config: Config,
    event_tx: mpsc::UnboundedSender<Event>,
) -> Result<ApproachOptions, String> {
    use arq_core::{ClaudeClient, OpenAIClient, PlanningProgress as CorePlanningProgress};

    // Send progress events
    let _ = event_tx.send(Event::PlanningProgress(CorePlanningProgress::Started));
    let _ = event_tx.send(Event::PlanningProgress(
        CorePlanningProgress::LoadingResearch,
    ));

    let provider = config.llm.provider.as_str();
    let model = config.llm.model_or_default();

    let _ = event_tx.send(Event::PlanningProgress(
        CorePlanningProgress::GeneratingApproaches,
    ));

    let options = match provider {
        "anthropic" | "claude" => {
            let api_key = config
                .llm
                .api_key_or_env()
                .ok_or_else(|| "ANTHROPIC_API_KEY not set".to_string())?;
            let client = ClaudeClient::new(api_key).with_model(&model);
            let runner = PlanningRunner::new(client);
            runner
                .generate_approaches(&research_doc)
                .await
                .map_err(|e| format!("Planning failed: {}", e))?
        }
        "ollama" => {
            let base_url = config.llm.base_url_or_default();
            let client = OpenAIClient::new(&base_url, "", &model);
            let runner = PlanningRunner::new(client);
            runner
                .generate_approaches(&research_doc)
                .await
                .map_err(|e| format!("Planning failed: {}", e))?
        }
        _ => {
            let base_url = config.llm.base_url_or_default();
            let api_key = config.llm.api_key_or_env().unwrap_or_default();
            let client = OpenAIClient::new(&base_url, &api_key, &model);
            let runner = PlanningRunner::new(client);
            runner
                .generate_approaches(&research_doc)
                .await
                .map_err(|e| format!("Planning failed: {}", e))?
        }
    };

    let count = options.len();
    let _ = event_tx.send(Event::PlanningProgress(
        CorePlanningProgress::ApproachesReady(count),
    ));

    Ok(options)
}

/// Run plan specification generation from a selected approach.
/// Returns Plan for user approval.
async fn run_planning_spec(
    research_doc: ResearchDoc,
    approach: Approach,
    config: Config,
    event_tx: mpsc::UnboundedSender<Event>,
) -> Result<Plan, String> {
    use arq_core::{ClaudeClient, OpenAIClient, PlanningProgress as CorePlanningProgress};

    let _ = event_tx.send(Event::PlanningProgress(
        CorePlanningProgress::GeneratingSpec,
    ));

    let provider = config.llm.provider.as_str();
    let model = config.llm.model_or_default();

    let plan = match provider {
        "anthropic" | "claude" => {
            let api_key = config
                .llm
                .api_key_or_env()
                .ok_or_else(|| "ANTHROPIC_API_KEY not set".to_string())?;
            let client = ClaudeClient::new(api_key).with_model(&model);
            let runner = PlanningRunner::new(client);
            runner
                .generate_plan(&research_doc, &approach)
                .await
                .map_err(|e| format!("Plan generation failed: {}", e))?
        }
        "ollama" => {
            let base_url = config.llm.base_url_or_default();
            let client = OpenAIClient::new(&base_url, "", &model);
            let runner = PlanningRunner::new(client);
            runner
                .generate_plan(&research_doc, &approach)
                .await
                .map_err(|e| format!("Plan generation failed: {}", e))?
        }
        _ => {
            let base_url = config.llm.base_url_or_default();
            let api_key = config.llm.api_key_or_env().unwrap_or_default();
            let client = OpenAIClient::new(&base_url, &api_key, &model);
            let runner = PlanningRunner::new(client);
            runner
                .generate_plan(&research_doc, &approach)
                .await
                .map_err(|e| format!("Plan generation failed: {}", e))?
        }
    };

    let _ = event_tx.send(Event::PlanningProgress(
        CorePlanningProgress::CheckingComplexity,
    ));
    let _ = event_tx.send(Event::PlanningProgress(CorePlanningProgress::Complete));

    Ok(plan)
}

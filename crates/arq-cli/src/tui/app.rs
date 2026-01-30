//! Application state and main event loop.

use std::io::Stdout;
use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use tokio::sync::mpsc;

use arq_core::{
    Config, ContextBuilder, FileStorage, ResearchProgress, ResearchRunner,
    StreamChunk, Task, TaskManager,
};

use super::event::{Event, EventHandler, ResearchResult};
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

/// Main application state.
pub struct App {
    /// Currently selected tab
    pub selected_tab: SelectedTab,
    /// Current input mode
    pub input_mode: InputMode,
    /// Chat messages for current tab
    pub chat_messages: Vec<ChatMessage>,
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
    /// Scroll offset for chat
    pub scroll_offset: usize,
    /// Configuration
    pub config: Config,
    /// Task manager for persistence
    pub manager: TaskManager<FileStorage>,
    /// Current task
    pub current_task: Option<Task>,
    /// Status message
    pub status_message: Option<String>,
}

impl App {
    /// Create a new app instance.
    pub fn new(config: Config, manager: TaskManager<FileStorage>) -> Self {
        let current_task = manager.get_current_task().ok().flatten();

        let mut app = Self {
            selected_tab: SelectedTab::Researcher,
            input_mode: InputMode::Normal,
            chat_messages: Vec::new(),
            input_buffer: String::new(),
            progress_items: Vec::new(),
            is_streaming: false,
            stream_buffer: String::new(),
            should_quit: false,
            scroll_offset: 0,
            config,
            manager,
            current_task: current_task.clone(),
            status_message: None,
        };

        // Add welcome message
        if let Some(ref task) = current_task {
            app.chat_messages.push(ChatMessage::system(format!(
                "Current task: {} ({})",
                task.name,
                task.phase.display_name()
            )));
        } else {
            app.chat_messages.push(ChatMessage::system(
                "Welcome to Arq! No active task. Type a prompt to start research.",
            ));
        }

        // Initialize progress items for research phase
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
                        // Update any animations or timers
                    }
                    Event::StreamChunk(text) => {
                        self.stream_buffer.push_str(&text);
                    }
                    Event::StreamComplete => {
                        if !self.stream_buffer.is_empty() {
                            self.chat_messages.push(ChatMessage::assistant(
                                std::mem::take(&mut self.stream_buffer),
                            ));
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
                self.status_message = Some(format!("Found {} relevant code segments", count));
            }
            ResearchProgress::CallingLLM => {
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
                self.chat_messages.push(ChatMessage::system(format!("Error: {}", msg)));
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

    /// Handle research completion - save doc and update UI.
    fn handle_research_complete(&mut self, result: ResearchResult) {
        self.is_streaming = false;

        // Format summary for display
        let summary = format!(
            "## Research Summary\n\n{}\n\n## Suggested Approach\n\n{}",
            result.doc.summary, result.doc.suggested_approach
        );
        self.chat_messages.push(ChatMessage::assistant(&summary));

        // Save via TaskManager
        match self.manager.set_research_doc(&result.task_id, result.doc) {
            Ok(task) => {
                self.current_task = Some(task);
                self.status_message = Some("Research saved to .arq/research-doc.md".to_string());
                self.chat_messages.push(ChatMessage::system(
                    "Research document saved. You can proceed to Planner tab."
                ));
            }
            Err(e) => {
                self.chat_messages.push(ChatMessage::system(format!(
                    "Warning: Failed to save research: {}", e
                )));
            }
        }
    }

    /// Handle research failure.
    fn handle_research_failed(&mut self, error: String) {
        self.is_streaming = false;
        self.chat_messages.push(ChatMessage::system(format!("Research failed: {}", error)));

        // Mark progress as failed
        for item in &mut self.progress_items {
            if item.status == ProgressStatus::InProgress {
                item.status = ProgressStatus::Failed;
                break;
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
                self.selected_tab = self.selected_tab.next();
                self.reset_progress_items();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.selected_tab = self.selected_tab.previous();
                self.reset_progress_items();
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
        self.chat_messages.push(ChatMessage::user(&input));

        match self.selected_tab {
            SelectedTab::Researcher => {
                self.start_research(input, event_tx);
            }
            SelectedTab::Planner => {
                self.chat_messages.push(ChatMessage::system(
                    "Planning phase not yet implemented.",
                ));
            }
            SelectedTab::Agent => {
                self.chat_messages.push(ChatMessage::system(
                    "Agent phase not yet implemented.",
                ));
            }
        }

        self.input_mode = InputMode::Normal;
    }

    /// Start a research task with streaming.
    fn start_research(&mut self, prompt: String, event_tx: mpsc::UnboundedSender<Event>) {
        self.is_streaming = true;
        self.stream_buffer.clear();
        self.reset_progress_items();

        // Create task via manager (persists immediately)
        let task = match self.manager.create_task(&prompt) {
            Ok(task) => task,
            Err(e) => {
                self.chat_messages.push(ChatMessage::system(format!(
                    "Failed to create task: {}", e
                )));
                self.is_streaming = false;
                return;
            }
        };

        let task_id = task.id.clone();
        self.current_task = Some(task.clone());

        // Get config values we need
        let config = self.config.clone();

        // Spawn the research task
        tokio::spawn(async move {
            match run_research_task(task, config, event_tx.clone()).await {
                Ok(doc) => {
                    let _ = event_tx.send(Event::ResearchComplete(ResearchResult {
                        task_id,
                        doc,
                    }));
                }
                Err(error) => {
                    let _ = event_tx.send(Event::ResearchFailed(error));
                }
            }
        });
    }

    /// Scroll chat up.
    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scroll chat down.
    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }
}

/// Run a research task with streaming and progress updates.
/// Returns the full ResearchDoc for persistence.
async fn run_research_task(
    task: Task,
    config: Config,
    event_tx: mpsc::UnboundedSender<Event>,
) -> Result<arq_core::ResearchDoc, String> {
    use arq_core::{ClaudeClient, OpenAIClient};
    use std::env;

    // Create context builder with config
    let cwd = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let context_builder = ContextBuilder::with_config(cwd, config.context.clone());

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

    let doc = match provider {
        "anthropic" | "claude" => {
            let api_key = config.llm.api_key_or_env()
                .ok_or_else(|| "ANTHROPIC_API_KEY not set".to_string())?;
            let client = ClaudeClient::new(api_key).with_model(&model);
            let runner = ResearchRunner::new(client, context_builder);
            runner
                .run_streaming(&task, progress_tx, stream_tx)
                .await
                .map_err(|e| format!("Research failed: {}", e))?
        }
        "ollama" => {
            let base_url = config.llm.base_url_or_default();
            let client = OpenAIClient::new(&base_url, "", &model);
            let runner = ResearchRunner::new(client, context_builder);
            runner
                .run_streaming(&task, progress_tx, stream_tx)
                .await
                .map_err(|e| format!("Research failed: {}", e))?
        }
        _ => {
            // OpenAI or OpenAI-compatible
            let base_url = config.llm.base_url_or_default();
            let api_key = config.llm.api_key_or_env().unwrap_or_default();
            let client = OpenAIClient::new(&base_url, &api_key, &model);
            let runner = ResearchRunner::new(client, context_builder);
            runner
                .run_streaming(&task, progress_tx, stream_tx)
                .await
                .map_err(|e| format!("Research failed: {}", e))?
        }
    };

    Ok(doc)
}

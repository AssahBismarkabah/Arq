//! TUI chat interface for Arq.
//!
//! Provides an interactive terminal UI with:
//! - Tabs for Research, Planning, and Agent phases
//! - Chat input/output display
//! - Progress checklist
//! - Streaming LLM responses

mod app;
mod components;
mod event;
mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::stdout;

use app::App;
use arq_core::{Config, FileStorage, TaskManager};

/// Run the TUI application.
pub async fn run(
    config: Config,
    manager: TaskManager<FileStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(config, manager);

    // Run the main loop
    let result = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

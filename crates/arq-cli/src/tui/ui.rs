//! UI rendering for the TUI.

use ratatui::{
    prelude::*,
    widgets::Paragraph,
};

use super::app::{App, InputMode, ResearchState};
use super::components::{chat, input, progress, tabs};

/// Render the entire UI.
pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();

    // Main layout: tabs, content, input, status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tab bar
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Status bar
        ])
        .split(area);

    // Render tabs
    tabs::render(app, frame, chunks[0]);

    // Render main content (chat + progress)
    render_main_content(app, frame, chunks[1]);

    // Render input
    input::render(app, frame, chunks[2]);

    // Render status bar
    render_status_bar(app, frame, chunks[3]);
}

/// Render the main content area (chat and progress side by side).
fn render_main_content(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Chat
            Constraint::Percentage(30),  // Progress
        ])
        .split(area);

    // Render chat
    chat::render(app, frame, chunks[0]);

    // Render progress
    progress::render(app, frame, chunks[1]);
}

/// Render the status bar.
fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    // Context-aware key bindings based on research state
    let mode_str = match (&app.input_mode, &app.research_state) {
        (InputMode::Editing, _) => "[Enter] Send  [Esc] Cancel",
        (InputMode::Normal, ResearchState::AwaitingValidation { .. }) => {
            "[a] Approve  [i] Edit corrections  [Tab] Switch  [q] Quit"
        }
        (InputMode::Normal, ResearchState::Researching | ResearchState::Refining) => {
            "Researching...  [q] Quit"
        }
        (InputMode::Normal, ResearchState::Idle) => {
            "[i] Edit  [Tab] Switch  [j/k] Scroll  [q] Quit"
        }
    };

    let task_info = app.current_task.as_ref().map_or_else(
        || "No active task".to_string(),
        |t| format!("{} ({})", t.name, t.phase.display_name()),
    );

    // Show status_message if present, otherwise show task_info
    let right_side = app.status_message.as_ref()
        .map(|s| s.as_str())
        .unwrap_or(&task_info);

    let status = format!("{}  |  {}", mode_str, right_side);

    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(status_bar, area);
}

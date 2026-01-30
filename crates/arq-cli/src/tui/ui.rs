//! UI rendering for the TUI.

use ratatui::{
    prelude::*,
    widgets::Paragraph,
};

use super::app::{App, InputMode};
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
    let mode_str = match app.input_mode {
        InputMode::Normal => "[i] Edit  [Tab] Switch  [j/k] Scroll  [q] Quit",
        InputMode::Editing => "[Enter] Send  [Esc] Cancel",
    };

    let task_info = app.current_task.as_ref().map_or_else(
        || "No active task".to_string(),
        |t| format!("{} ({})", t.name, t.phase.display_name()),
    );

    let status = format!("{}  |  {}", mode_str, task_info);

    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(status_bar, area);
}

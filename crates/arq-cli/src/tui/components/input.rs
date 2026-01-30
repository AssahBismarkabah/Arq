//! Input field component.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Paragraph},
};

use crate::tui::app::{App, InputMode};

/// Render the input field.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let border_style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::DarkGray),
        InputMode::Editing => Style::default().fg(Color::Yellow),
    };

    let block = Block::default()
        .title(" Input ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner_area = block.inner(area);

    // Build input text with cursor
    let input_text = if app.input_mode == InputMode::Editing {
        format!("{}_", app.input_buffer)
    } else if app.input_buffer.is_empty() {
        "Press 'i' to type...".to_string()
    } else {
        app.input_buffer.clone()
    };

    let text_style = if app.input_mode == InputMode::Editing {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input = Paragraph::new(input_text)
        .style(text_style)
        .block(block);

    frame.render_widget(input, area);

    // Set cursor position in editing mode
    if app.input_mode == InputMode::Editing {
        let cursor_x = inner_area.x + app.input_buffer.len() as u16;
        let cursor_y = inner_area.y;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}

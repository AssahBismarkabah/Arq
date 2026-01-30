//! Chat message display component.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Paragraph},
};

use crate::tui::app::{App, MessageRole};

/// Wrap text to fit within a given width.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for line in text.lines() {
        if line.len() <= width {
            lines.push(line.to_string());
        } else {
            // Word wrap long lines
            let mut current_line = String::new();
            for word in line.split_whitespace() {
                let test_len = if current_line.is_empty() {
                    word.len()
                } else {
                    current_line.len() + 1 + word.len()
                };

                if test_len <= width {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                } else {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                    // Handle words longer than width
                    if word.len() > width {
                        let mut remaining = word;
                        while remaining.len() > width {
                            lines.push(remaining[..width].to_string());
                            remaining = &remaining[width..];
                        }
                        current_line = remaining.to_string();
                    } else {
                        current_line = word.to_string();
                    }
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }

    // Add empty line if text was empty
    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Render the chat message list.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let tab_title = app.selected_tab.title();

    let block = Block::default()
        .title(format!(" {} Chat ", tab_title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Calculate available width for text (subtract prefix width)
    let prefix_width = 10; // "[System] " is the longest prefix
    let text_width = (inner_area.width as usize).saturating_sub(prefix_width);

    // Build all lines with proper wrapping
    let mut all_lines: Vec<Line> = Vec::new();

    for msg in &app.chat_messages {
        let (prefix_style, content_style) = match msg.role {
            MessageRole::User => (
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                Style::default().fg(Color::White),
            ),
            MessageRole::Assistant => (
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                Style::default().fg(Color::White),
            ),
            MessageRole::System => (
                Style::default().fg(Color::Yellow),
                Style::default().fg(Color::DarkGray),
            ),
        };

        let prefix = format!("[{}] ", msg.role.as_str());
        let indent = "       "; // Spaces to align continuation lines

        // Wrap the message content
        let wrapped_lines = wrap_text(&msg.content, text_width);

        for (i, line) in wrapped_lines.into_iter().enumerate() {
            if i == 0 {
                all_lines.push(Line::from(vec![
                    Span::styled(prefix.clone(), prefix_style),
                    Span::styled(line, content_style),
                ]));
            } else {
                all_lines.push(Line::from(vec![
                    Span::styled(indent.to_string(), Style::default()),
                    Span::styled(line, content_style),
                ]));
            }
        }
    }

    // Add streaming buffer if active
    if app.is_streaming && !app.stream_buffer.is_empty() {
        let wrapped_lines = wrap_text(&app.stream_buffer, text_width);

        for (i, line) in wrapped_lines.into_iter().enumerate() {
            if i == 0 {
                all_lines.push(Line::from(vec![
                    Span::styled(
                        "[Arq] ".to_string(),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(line, Style::default().fg(Color::White)),
                ]));
            } else {
                all_lines.push(Line::from(vec![
                    Span::styled("       ".to_string(), Style::default()),
                    Span::styled(line, Style::default().fg(Color::White)),
                ]));
            }
        }
    }

    // Calculate scroll position
    let visible_height = inner_area.height as usize;
    let total_lines = all_lines.len();

    // Auto-scroll to bottom unless user has scrolled up
    let start_index = if app.scroll_offset > 0 {
        // User has scrolled up - show earlier content
        total_lines.saturating_sub(visible_height).saturating_sub(app.scroll_offset)
    } else {
        // Auto-scroll to bottom
        total_lines.saturating_sub(visible_height)
    };

    // Check if there's more content above or below
    let has_more_above = start_index > 0;
    let has_more_below = start_index + visible_height < total_lines;

    let visible_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(start_index)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, inner_area);

    // Show scroll indicators if there's more content
    if has_more_above {
        let indicator = Paragraph::new("▲ more above (k to scroll up)")
            .style(Style::default().fg(Color::DarkGray));
        let indicator_area = Rect::new(inner_area.x, inner_area.y, inner_area.width, 1);
        frame.render_widget(indicator, indicator_area);
    }

    if has_more_below {
        let indicator = Paragraph::new("▼ more below (j to scroll down)")
            .style(Style::default().fg(Color::DarkGray));
        let indicator_area = Rect::new(
            inner_area.x,
            inner_area.y + inner_area.height - 1,
            inner_area.width,
            1,
        );
        frame.render_widget(indicator, indicator_area);
    }
}

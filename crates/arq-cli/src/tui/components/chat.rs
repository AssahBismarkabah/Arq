//! Chat message display component.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, List, ListItem},
};

use crate::tui::app::{App, MessageRole};

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

    // Build message items
    let mut items: Vec<ListItem> = app
        .chat_messages
        .iter()
        .map(|msg| {
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

            // Wrap long messages
            let lines: Vec<Line> = msg.content
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    if i == 0 {
                        Line::from(vec![
                            Span::styled(prefix.clone(), prefix_style),
                            Span::styled(line, content_style),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("       "), // Indent continuation
                            Span::styled(line, content_style),
                        ])
                    }
                })
                .collect();

            ListItem::new(lines)
        })
        .collect();

    // Add streaming buffer if active
    if app.is_streaming && !app.stream_buffer.is_empty() {
        let streaming_lines: Vec<Line> = app.stream_buffer
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    Line::from(vec![
                        Span::styled(
                            "[Arq] ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(line, Style::default().fg(Color::White)),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw("       "),
                        Span::styled(line, Style::default().fg(Color::White)),
                    ])
                }
            })
            .collect();

        // Add cursor at the end
        items.push(ListItem::new(streaming_lines));
    }

    // Calculate scroll position
    let visible_height = inner_area.height as usize;
    let total_items = items.len();

    // Auto-scroll to bottom unless user has scrolled up
    let start_index = if app.scroll_offset > 0 {
        total_items.saturating_sub(visible_height + app.scroll_offset)
    } else {
        total_items.saturating_sub(visible_height)
    };

    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(start_index)
        .take(visible_height)
        .collect();

    let list = List::new(visible_items);
    frame.render_widget(list, inner_area);
}

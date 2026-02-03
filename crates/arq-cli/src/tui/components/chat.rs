//! Chat message display component with syntax highlighting.

use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::tui::app::{App, MessageRole, PlanningState, ResearchState};
use crate::tui::highlight::{highlight_markdown, Highlighter};

/// Wrap text to fit within a given width (character-aware for UTF-8).
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for line in text.lines() {
        let char_count = line.chars().count();
        if char_count <= width {
            lines.push(line.to_string());
        } else {
            // Word wrap long lines
            let mut current_line = String::new();
            let mut current_len = 0;

            for word in line.split_whitespace() {
                let word_len = word.chars().count();
                let test_len = if current_line.is_empty() {
                    word_len
                } else {
                    current_len + 1 + word_len
                };

                if test_len <= width {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                        current_len += 1;
                    }
                    current_line.push_str(word);
                    current_len += word_len;
                } else {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = String::new();
                        current_len = 0;
                    }
                    // Handle words longer than width - split by characters
                    if word_len > width {
                        let chars: Vec<char> = word.chars().collect();
                        let mut i = 0;
                        while i < chars.len() {
                            let end = (i + width).min(chars.len());
                            let chunk: String = chars[i..end].iter().collect();
                            if end < chars.len() {
                                lines.push(chunk);
                            } else {
                                current_line = chunk;
                                current_len = end - i;
                            }
                            i = end;
                        }
                    } else {
                        current_line = word.to_string();
                        current_len = word_len;
                    }
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Render the chat message list with syntax highlighting.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let tab_title = app.selected_tab.title();

    let block = Block::default()
        .title(format!(" {} Chat ", tab_title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let prefix_width = 10;
    let text_width = (inner_area.width as usize).saturating_sub(prefix_width);

    let mut all_lines: Vec<Line> = Vec::new();

    for msg in app.chat_messages() {
        let prefix_style = match msg.role {
            MessageRole::User => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            MessageRole::Assistant => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            MessageRole::System => Style::default().fg(Color::Yellow),
        };

        let prefix = format!("[{}] ", msg.role.as_str());
        let indent = "       ";

        // Create highlighter for this message
        let mut highlighter = Highlighter::new();

        for (i, line) in msg.content.lines().enumerate() {
            let wrapped_lines = wrap_text(line, text_width);

            for (j, wrapped_line) in wrapped_lines.into_iter().enumerate() {
                let mut spans = Vec::new();

                // Add prefix or indent
                if i == 0 && j == 0 {
                    spans.push(Span::styled(prefix.clone(), prefix_style));
                } else {
                    spans.push(Span::styled(indent.to_string(), Style::default()));
                }

                // Apply syntax highlighting based on role
                match msg.role {
                    MessageRole::Assistant => {
                        // Check for markdown first
                        if let Some(md_spans) = highlight_markdown(&wrapped_line) {
                            spans.extend(md_spans);
                        } else {
                            spans.extend(highlighter.highlight_line(&wrapped_line));
                        }
                    }
                    MessageRole::System => {
                        spans.push(Span::styled(
                            wrapped_line,
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                    MessageRole::User => {
                        spans.push(Span::styled(
                            wrapped_line,
                            Style::default().fg(Color::White),
                        ));
                    }
                }

                all_lines.push(Line::from(spans));
            }
        }

        // Handle empty messages
        if msg.content.is_empty() {
            all_lines.push(Line::from(vec![
                Span::styled(prefix.clone(), prefix_style),
                Span::raw(String::new()),
            ]));
        }
    }

    // Add streaming buffer if active (but not during research/planning which shows raw JSON)
    let is_research_or_planning = matches!(
        app.research_state,
        ResearchState::Researching | ResearchState::Refining
    ) || matches!(
        app.planning_state,
        PlanningState::GeneratingApproaches | PlanningState::GeneratingPlan { .. }
    );

    if app.is_streaming && !app.stream_buffer.is_empty() && !is_research_or_planning {
        let mut highlighter = Highlighter::new();

        for (i, line) in app.stream_buffer.lines().enumerate() {
            let wrapped_lines = wrap_text(line, text_width);

            for (j, wrapped_line) in wrapped_lines.into_iter().enumerate() {
                let mut spans = Vec::new();

                if i == 0 && j == 0 {
                    spans.push(Span::styled(
                        "[Arq] ".to_string(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::styled("       ".to_string(), Style::default()));
                }

                // Check for markdown first
                if let Some(md_spans) = highlight_markdown(&wrapped_line) {
                    spans.extend(md_spans);
                } else {
                    spans.extend(highlighter.highlight_line(&wrapped_line));
                }

                all_lines.push(Line::from(spans));
            }
        }
    }

    // Calculate scroll position
    let visible_height = inner_area.height as usize;
    let total_lines = all_lines.len();

    let start_index = if app.scroll_offset() > 0 {
        total_lines
            .saturating_sub(visible_height)
            .saturating_sub(app.scroll_offset())
    } else {
        total_lines.saturating_sub(visible_height)
    };

    let has_more_above = start_index > 0;
    let has_more_below = start_index + visible_height < total_lines;

    let visible_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(start_index)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, inner_area);

    // Show scroll indicators
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

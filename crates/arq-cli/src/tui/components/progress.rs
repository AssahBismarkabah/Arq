//! Progress checklist component.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, List, ListItem},
};

use crate::tui::app::{App, ProgressStatus};

/// Render the progress checklist.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Progress ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app
        .progress_items
        .iter()
        .map(|item| {
            let (icon_style, label_style) = match item.status {
                ProgressStatus::Pending => (
                    Style::default().fg(Color::DarkGray),
                    Style::default().fg(Color::DarkGray),
                ),
                ProgressStatus::InProgress => (
                    Style::default().fg(Color::Yellow),
                    Style::default().fg(Color::Yellow),
                ),
                ProgressStatus::Complete => (
                    Style::default().fg(Color::Green),
                    Style::default().fg(Color::White),
                ),
                ProgressStatus::Failed => (
                    Style::default().fg(Color::Red),
                    Style::default().fg(Color::Red),
                ),
            };

            let line = Line::from(vec![
                Span::styled(format!(" {} ", item.status.icon()), icon_style),
                Span::styled(&item.label, label_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner_area);
}

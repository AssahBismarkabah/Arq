//! Tab bar component.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Tabs as RataTabs},
};

use crate::tui::app::App;

/// Render the tab bar.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let titles = vec!["Researcher", "Planner", "Agent"];

    let tabs = RataTabs::new(titles)
        .block(
            Block::default()
                .title(" Arq ")
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(app.selected_tab.index())
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" | ");

    frame.render_widget(tabs, area);
}

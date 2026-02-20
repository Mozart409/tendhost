//! Event log panel widget

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, EventLevel, Focus};
use crate::config;

/// Render the event log panel
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == Focus::Events {
        config::focused_border_style()
    } else {
        config::unfocused_border_style()
    };

    let items: Vec<ListItem> = app
        .event_log
        .iter()
        .take(area.height.saturating_sub(2) as usize)
        .map(|entry| {
            let time = entry.timestamp.format("%H:%M:%S");
            let style = match entry.level {
                EventLevel::Info => Style::default().fg(Color::White),
                EventLevel::Success => Style::default().fg(Color::Green),
                EventLevel::Warning => Style::default().fg(Color::Yellow),
                EventLevel::Error => Style::default().fg(Color::Red),
            };
            let text = format!("{} {}", time, entry.message);
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Events ")
            .borders(Borders::ALL)
            .border_style(border_style),
    );

    frame.render_widget(list, area);
}

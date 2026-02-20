//! Layout calculations for the TUI

use ratatui::prelude::*;

/// Layout areas for the UI
pub struct LayoutAreas {
    pub hosts: Rect,
    pub details: Rect,
    pub events: Rect,
    pub statusbar: Rect,
}

/// Calculate layout areas based on terminal size
pub fn calculate_layout(area: Rect) -> LayoutAreas {
    // Main vertical split: content + status bar
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    let content_area = vertical[0];
    let statusbar = vertical[1];

    // Content: hosts list (left) + details/events (right)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Host list
            Constraint::Percentage(50), // Details + events
        ])
        .split(content_area);

    let hosts = horizontal[0];

    // Right panel: details (top) + events (bottom)
    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60), // Details
            Constraint::Percentage(40), // Events
        ])
        .split(horizontal[1]);

    let details = right_panel[0];
    let events = right_panel[1];

    LayoutAreas {
        hosts,
        details,
        events,
        statusbar,
    }
}

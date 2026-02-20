//! Status bar widget

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, ConnectionState};

/// Render the status bar
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let connection_status = match &app.connection_state {
        ConnectionState::Connected => ("● Connected", Color::Green),
        ConnectionState::Connecting => ("◐ Connecting...", Color::Yellow),
        ConnectionState::Disconnected => ("○ Disconnected", Color::Red),
        ConnectionState::Reconnecting { .. } => {
            // Simplified version without attempt count to avoid lifetime issues
            ("◐ Reconnecting", Color::Yellow)
        }
    };

    let keybindings = "[j/k] Navigate  [Enter] Details  [u] Update  [r] Reboot  [?] Help  [q] Quit";

    let status_line = Line::from(vec![
        Span::styled(
            connection_status.0,
            Style::default().fg(connection_status.1),
        ),
        Span::raw("  │  "),
        Span::styled(keybindings, Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(status_line);
    frame.render_widget(paragraph, area);
}

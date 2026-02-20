//! Help popup widget

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Render the help popup
pub fn render(frame: &mut Frame) {
    let help_text = r"
  Navigation
  ──────────
  j/↓       Move down
  k/↑       Move up
  g         Jump to first
  G         Jump to last
  Tab       Switch panel focus
  Enter     Show host details
  Esc       Close popup/clear search

  Actions
  ───────
  u         Trigger update
  U         Fleet update
  r         Reboot host
  R         Retry failed host
  a         Acknowledge failure
  i         Refresh inventory

  General
  ───────
  /         Search hosts
  ?         Toggle help
  q         Quit
";

    // Calculate popup area (centered, 50x24)
    let area = frame.area();
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 24.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}

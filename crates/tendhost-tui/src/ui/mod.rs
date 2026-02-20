//! UI rendering modules

mod details;
mod events;
mod help;
mod hosts;
mod layout;
mod statusbar;

use ratatui::prelude::*;

use crate::app::App;

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let areas = layout::calculate_layout(frame.area());

    // Render main components
    hosts::render(frame, app, areas.hosts);
    details::render(frame, app, areas.details);
    events::render(frame, app, areas.events);
    statusbar::render(frame, app, areas.statusbar);

    // Render help popup if active
    if app.show_help {
        help::render(frame);
    }
}

//! TUI configuration

use ratatui::style::{Color, Modifier, Style};

/// State colors
pub fn state_color(state: &str) -> Color {
    match state.to_lowercase().as_str() {
        "idle" => Color::Green,
        "querying" | "pendingupdates" | "pending_updates" => Color::Yellow,
        "updating" => Color::Blue,
        "waitingreboot" | "waiting_reboot" | "verifying" => Color::Cyan,
        "rebooting" => Color::Magenta,
        "failed" => Color::Red,
        "offline" => Color::DarkGray,
        _ => Color::White,
    }
}

/// State symbol
pub fn state_symbol(state: &str, tick: u64) -> &'static str {
    match state.to_lowercase().as_str() {
        "querying" | "rebooting" | "verifying" => {
            if tick % 4 < 2 {
                "◐"
            } else {
                "◑"
            }
        }
        "idle" | "pendingupdates" | "pending_updates" => "●",
        "updating" => match tick % 4 {
            0 => "◐",
            1 => "◓",
            2 => "◑",
            _ => "◒",
        },
        "waitingreboot" | "waiting_reboot" => "◎",
        "failed" => "✗",
        "offline" => "○",
        _ => "?",
    }
}

/// Header style
pub fn header_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// Selected row style
pub fn selected_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD)
}

/// Normal row style
pub fn normal_style() -> Style {
    Style::default()
}

/// Border style for focused panel
pub fn focused_border_style() -> Style {
    Style::default().fg(Color::Cyan)
}

/// Border style for unfocused panel
pub fn unfocused_border_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

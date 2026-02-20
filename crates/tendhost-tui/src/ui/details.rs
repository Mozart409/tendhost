//! Host details panel widget

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Focus};
use crate::config;

/// Render the host details panel
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == Focus::Details {
        config::focused_border_style()
    } else {
        config::unfocused_border_style()
    };

    let title = match app.selected_host_name() {
        Some(name) => format!(" Host: {name} "),
        None => " Host Details ".to_string(),
    };

    let content = if let Some(details) = &app.host_details {
        format_details(details)
    } else if let Some(host) = app.hosts.get(app.selected_host) {
        format!(
            "Host: {}\nState: {}\nOS: {}\n\nPress Enter to load details",
            host.name, host.state, host.os
        )
    } else {
        "No host selected".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Format host details JSON into readable text
fn format_details(details: &serde_json::Value) -> String {
    let mut lines = Vec::new();

    // Extract common fields
    if let Some(name) = details.get("name").and_then(|v| v.as_str()) {
        lines.push(format!("Name: {name}"));
    }
    if let Some(state) = details.get("state").and_then(|v| v.as_str()) {
        lines.push(format!("State: {state}"));
    }
    if let Some(addr) = details.get("addr").and_then(|v| v.as_str()) {
        lines.push(format!("Address: {addr}"));
    }

    lines.push(String::new());

    // System info
    if let Some(system) = details.get("system") {
        if let Some(os) = system.get("os_name").and_then(|v| v.as_str()) {
            let version = system
                .get("os_version")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            lines.push(format!("OS: {os} {version}"));
        }
        if let Some(hostname) = system.get("hostname").and_then(|v| v.as_str()) {
            lines.push(format!("Hostname: {hostname}"));
        }
        if let Some(uptime) = system
            .get("uptime_seconds")
            .and_then(serde_json::Value::as_u64)
        {
            let uptime_str = format_uptime(uptime);
            lines.push(format!("Uptime: {uptime_str}"));
        }
    }

    lines.push(String::new());

    // Upgradable packages
    if let Some(packages) = details
        .get("upgradable_packages")
        .and_then(|v| v.as_array())
    {
        let pkg_count = packages.len();
        lines.push(format!("Upgradable Packages: {pkg_count}"));
        for (i, pkg) in packages.iter().take(10).enumerate() {
            if let Some(name) = pkg.get("name").and_then(|v| v.as_str()) {
                let from = pkg
                    .get("current_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let to = pkg
                    .get("new_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let prefix = if i == packages.len().min(10) - 1 {
                    "└──"
                } else {
                    "├──"
                };
                lines.push(format!("  {prefix} {name} ({from} → {to})"));
            }
        }
        if packages.len() > 10 {
            let more_count = packages.len() - 10;
            lines.push(format!("  ... ({more_count} more)"));
        }
    }

    lines.join("\n")
}

/// Format uptime seconds to human-readable string
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

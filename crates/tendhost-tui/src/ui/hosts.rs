//! Host list table widget

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

use crate::app::{App, Focus};
use crate::config;

/// Render the host list table
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let hosts = app.filtered_hosts();

    // Create header
    let header = Row::new(vec![
        Cell::from("Host"),
        Cell::from("State"),
        Cell::from("OS"),
        Cell::from("Pkgs"),
    ])
    .style(config::header_style())
    .height(1);

    // Create rows
    let rows: Vec<Row> = hosts
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let state_symbol = config::state_symbol(&host.state, app.tick);
            let state_color = config::state_color(&host.state);

            let state = &host.state;
            let cells = vec![
                Cell::from(host.name.clone()),
                Cell::from(format!("{state_symbol} {state}"))
                    .style(Style::default().fg(state_color)),
                Cell::from(host.os.clone()),
                Cell::from(
                    host.packages
                        .map_or_else(|| "--".to_string(), |p| p.to_string()),
                ),
            ];

            let style = if i == app.selected_host {
                config::selected_style()
            } else {
                config::normal_style()
            };

            Row::new(cells).style(style)
        })
        .collect();

    // Create table
    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(25),
        Constraint::Percentage(30),
        Constraint::Percentage(15),
    ];

    let border_style = if app.focus == Focus::HostList {
        config::focused_border_style()
    } else {
        config::unfocused_border_style()
    };

    let title = if app.search_active {
        let query = &app.search_query;
        format!(" Hosts (/{query}) ")
    } else {
        let count = hosts.len();
        format!(" Hosts ({count}) ")
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(config::selected_style())
        .highlight_symbol("▸ ");

    // Render with state for selection
    let mut state = TableState::default();
    state.select(Some(app.selected_host));

    frame.render_stateful_widget(table, area, &mut state);
}

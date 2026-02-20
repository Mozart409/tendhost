# tendhost-tui: Implementation Plan

## Overview

Implement a terminal user interface for monitoring and controlling tendhost daemon with real-time updates via WebSocket.

**Estimated time**: ~12 hours

## Prerequisites

- `tendhost-api` types defined
- `tendhost-client` complete (HTTP + WebSocket client)
- Daemon API running (for testing)

---

## Phase 1: Foundation (2 hours)

### Task 1.1: Update Dependencies

**File**: `Cargo.toml`

```toml
[package]
name = "tendhost-tui"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "tendhost-tui"
path = "src/main.rs"

[dependencies]
ratatui = { workspace = true }
crossterm = { workspace = true }
tokio = { workspace = true, features = ["full", "sync", "macros"] }
serde_json = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
color-eyre = { workspace = true }
clap = { workspace = true }
chrono = { workspace = true }
unicode-width = "0.2"

tendhost-api = { workspace = true }
tendhost-client = { workspace = true }
```

### Task 1.2: Create Action Types

**File**: `src/action.rs`

```rust
//! User actions for the TUI application

use std::time::Duration;

/// Actions that can be performed in the application
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Quit the application
    Quit,
    /// Tick event for animations/timers
    Tick,
    /// Render the UI
    Render,
    /// Navigate selection up
    Up,
    /// Navigate selection down
    Down,
    /// Jump to first item
    First,
    /// Jump to last item
    Last,
    /// Select/enter on current item
    Select,
    /// Go back / close popup
    Back,
    /// Show help popup
    Help,
    /// Trigger update on selected host
    TriggerUpdate,
    /// Trigger fleet update
    TriggerFleetUpdate,
    /// Trigger reboot on selected host
    TriggerReboot,
    /// Retry failed host
    RetryHost,
    /// Acknowledge failure
    AcknowledgeFailure,
    /// Refresh host inventory
    RefreshInventory,
    /// Toggle focus between panels
    ToggleFocus,
    /// Start search mode
    StartSearch,
    /// Update search query
    SearchInput(char),
    /// Clear search
    ClearSearch,
    /// WebSocket event received
    WsEvent(tendhost_api::events::WsEvent),
    /// WebSocket connected
    WsConnected,
    /// WebSocket disconnected
    WsDisconnected(String),
    /// HTTP error occurred
    HttpError(String),
    /// Host list loaded from HTTP
    HostsLoaded(Vec<serde_json::Value>),
    /// Host details loaded
    HostDetailsLoaded(String, serde_json::Value),
    /// No operation
    None,
}
```

### Task 1.3: Create Event Handling

**File**: `src/event.rs`

```rust
//! Event handling for terminal and application events

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::action::Action;

/// Terminal event types
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal key event
    Key(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick for animations
    Tick,
}

/// Event handler that polls for terminal events
pub struct EventHandler {
    /// Event sender
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver
    receiver: mpsc::UnboundedReceiver<Event>,
    /// Tick rate
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            tick_rate,
        }
    }

    /// Get the event sender for external events
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }

    /// Start the event loop in a background task
    pub fn start(&self) {
        let sender = self.sender.clone();
        let tick_rate = self.tick_rate;

        tokio::spawn(async move {
            let mut last_tick = std::time::Instant::now();

            loop {
                // Calculate timeout until next tick
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::ZERO);

                // Poll for events
                if event::poll(timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if sender.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if sender.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                // Send tick event
                if last_tick.elapsed() >= tick_rate {
                    if sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = std::time::Instant::now();
                }
            }
        });
    }

    /// Receive the next event
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}

/// Convert a key event to an action
pub fn key_to_action(key: KeyEvent) -> Action {
    match key.code {
        // Quit
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => Action::Up,
        KeyCode::Down | KeyCode::Char('j') => Action::Down,
        KeyCode::Char('g') => Action::First,
        KeyCode::Char('G') => Action::Last,
        KeyCode::Enter => Action::Select,
        KeyCode::Esc => Action::Back,
        KeyCode::Tab => Action::ToggleFocus,

        // Actions
        KeyCode::Char('u') => Action::TriggerUpdate,
        KeyCode::Char('U') => Action::TriggerFleetUpdate,
        KeyCode::Char('r') => Action::TriggerReboot,
        KeyCode::Char('R') => Action::RetryHost,
        KeyCode::Char('a') => Action::AcknowledgeFailure,
        KeyCode::Char('i') => Action::RefreshInventory,

        // Help and search
        KeyCode::Char('?') => Action::Help,
        KeyCode::Char('/') => Action::StartSearch,

        _ => Action::None,
    }
}
```

### Task 1.4: Create Main Entry Point

**File**: `src/main.rs`

```rust
//! tendhost TUI
//!
//! Terminal user interface for monitoring and controlling tendhost daemon

use std::io;
use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod action;
mod app;
mod config;
mod event;
mod ui;

use app::App;
use event::EventHandler;

/// tendhost Terminal UI
#[derive(Parser, Debug)]
#[command(name = "tendhost-tui", version, about)]
struct Args {
    /// Server address
    #[arg(short, long, default_value = "http://localhost:8080")]
    server: String,

    /// Tick rate in milliseconds
    #[arg(long, default_value = "250")]
    tick_rate: u64,

    /// Enable debug logging to file
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Parse arguments
    let args = Args::parse();

    // Initialize logging
    if args.debug {
        let file = std::fs::File::create("tendhost-tui.log")?;
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(file))
            .init();
    }

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let tick_rate = Duration::from_millis(args.tick_rate);
    let mut app = App::new(&args.server);
    let result = run_app(&mut terminal, &mut app, tick_rate).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

/// Run the application main loop
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
) -> Result<()> {
    // Create event handler
    let mut events = EventHandler::new(tick_rate);
    events.start();

    // Connect to daemon
    app.connect().await?;

    // Main loop
    while !app.should_quit() {
        // Draw UI
        terminal.draw(|frame| ui::render(frame, app))?;

        // Handle events
        if let Some(event) = events.next().await {
            let action = match event {
                event::Event::Key(key) => event::key_to_action(key),
                event::Event::Resize(_, _) => action::Action::Render,
                event::Event::Tick => action::Action::Tick,
            };
            app.handle_action(action).await?;
        }

        // Process any pending WebSocket events
        app.process_ws_events().await?;
    }

    Ok(())
}
```

### Task 1.5: Create App State

**File**: `src/app.rs`

```rust
//! Application state and logic

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use color_eyre::Result;
use tendhost_api::events::WsEvent;
use tendhost_client::{HttpClient, WsClient};
use tokio::sync::mpsc;

use crate::action::Action;

/// UI focus state
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Focus {
    #[default]
    HostList,
    Details,
    Events,
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
}

/// Event log entry
#[derive(Debug, Clone)]
pub struct EventLogEntry {
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub level: EventLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Host display data
#[derive(Debug, Clone, Default)]
pub struct HostDisplay {
    pub name: String,
    pub state: String,
    pub os: String,
    pub packages: Option<u32>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Application state
pub struct App {
    /// Server URL
    server_url: String,
    /// HTTP client
    http_client: Option<HttpClient>,
    /// WebSocket client
    ws_client: Option<WsClient>,
    /// WebSocket event receiver
    ws_receiver: Option<mpsc::Receiver<WsEvent>>,
    /// Should quit
    should_quit: bool,
    /// Current focus
    pub focus: Focus,
    /// Connection state
    pub connection_state: ConnectionState,
    /// Host list
    pub hosts: Vec<HostDisplay>,
    /// Selected host index
    pub selected_host: usize,
    /// Selected host details (JSON)
    pub host_details: Option<serde_json::Value>,
    /// Event log
    pub event_log: VecDeque<EventLogEntry>,
    /// Show help popup
    pub show_help: bool,
    /// Search mode active
    pub search_active: bool,
    /// Search query
    pub search_query: String,
    /// Error message (for toast)
    pub error_message: Option<String>,
    /// Tick counter for animations
    pub tick: u64,
}

impl App {
    /// Create a new application
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
            http_client: None,
            ws_client: None,
            ws_receiver: None,
            should_quit: false,
            focus: Focus::HostList,
            connection_state: ConnectionState::Disconnected,
            hosts: Vec::new(),
            selected_host: 0,
            host_details: None,
            event_log: VecDeque::with_capacity(100),
            show_help: false,
            search_active: false,
            search_query: String::new(),
            error_message: None,
            tick: 0,
        }
    }

    /// Check if app should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Connect to the daemon
    pub async fn connect(&mut self) -> Result<()> {
        self.connection_state = ConnectionState::Connecting;

        // Create HTTP client
        let http_client = HttpClient::new(&self.server_url)?;
        self.http_client = Some(http_client.clone());

        // Load initial host list
        self.load_hosts().await?;

        // Connect WebSocket
        let ws_url = self.server_url.replace("http", "ws") + "/ws/events";
        match WsClient::connect(&ws_url).await {
            Ok(ws_client) => {
                self.ws_client = Some(ws_client);
                self.connection_state = ConnectionState::Connected;
                self.log_event("Connected to daemon", EventLevel::Success);
            }
            Err(e) => {
                self.log_event(&format!("WebSocket failed: {e}"), EventLevel::Warning);
                // Continue without WebSocket - can still use HTTP
            }
        }

        Ok(())
    }

    /// Load hosts from HTTP API
    async fn load_hosts(&mut self) -> Result<()> {
        if let Some(client) = &self.http_client {
            match client.list_hosts().send().await {
                Ok(response) => {
                    self.hosts = response
                        .data
                        .iter()
                        .map(|h| {
                            HostDisplay {
                                name: h.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                state: h.get("state")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                os: h.get("os")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                packages: h.get("upgradable_packages")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32),
                                last_updated: None,
                            }
                        })
                        .collect();
                }
                Err(e) => {
                    self.log_event(&format!("Failed to load hosts: {e}"), EventLevel::Error);
                }
            }
        }
        Ok(())
    }

    /// Process WebSocket events
    pub async fn process_ws_events(&mut self) -> Result<()> {
        if let Some(ws_client) = &mut self.ws_client {
            // Non-blocking check for events
            while let Ok(event) = tokio::time::timeout(
                std::time::Duration::from_millis(1),
                async { ws_client.recv().await },
            )
            .await
            {
                if let Some(event) = event {
                    self.handle_ws_event(event);
                }
            }
        }
        Ok(())
    }

    /// Handle a WebSocket event
    fn handle_ws_event(&mut self, event: WsEvent) {
        match &event {
            WsEvent::HostStateChanged { host, from, to } => {
                self.log_event(
                    &format!("{host}: {from} -> {to}"),
                    EventLevel::Info,
                );
                // Update host state in list
                if let Some(h) = self.hosts.iter_mut().find(|h| h.name == *host) {
                    h.state = to.clone();
                }
            }
            WsEvent::UpdateProgress { host, package, progress } => {
                self.log_event(
                    &format!("{host}: Installing {package} ({progress}%)"),
                    EventLevel::Info,
                );
            }
            WsEvent::UpdateCompleted { host, result } => {
                self.log_event(
                    &format!("{host}: Update completed - {result}"),
                    EventLevel::Success,
                );
            }
            WsEvent::HostConnected { host } => {
                self.log_event(&format!("{host}: Connected"), EventLevel::Info);
            }
            WsEvent::HostDisconnected { host, reason } => {
                self.log_event(
                    &format!("{host}: Disconnected - {reason}"),
                    EventLevel::Warning,
                );
            }
        }
    }

    /// Log an event
    fn log_event(&mut self, message: &str, level: EventLevel) {
        let entry = EventLogEntry {
            timestamp: Utc::now(),
            message: message.to_string(),
            level,
        };
        self.event_log.push_front(entry);
        if self.event_log.len() > 100 {
            self.event_log.pop_back();
        }
    }

    /// Handle an action
    pub async fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::Tick => {
                self.tick = self.tick.wrapping_add(1);
            }
            Action::Up => {
                if self.selected_host > 0 {
                    self.selected_host -= 1;
                }
            }
            Action::Down => {
                if self.selected_host < self.hosts.len().saturating_sub(1) {
                    self.selected_host += 1;
                }
            }
            Action::First => {
                self.selected_host = 0;
            }
            Action::Last => {
                self.selected_host = self.hosts.len().saturating_sub(1);
            }
            Action::Select => {
                self.load_selected_host_details().await?;
            }
            Action::Back => {
                if self.show_help {
                    self.show_help = false;
                } else if self.search_active {
                    self.search_active = false;
                    self.search_query.clear();
                }
            }
            Action::Help => {
                self.show_help = !self.show_help;
            }
            Action::ToggleFocus => {
                self.focus = match self.focus {
                    Focus::HostList => Focus::Details,
                    Focus::Details => Focus::Events,
                    Focus::Events => Focus::HostList,
                };
            }
            Action::TriggerUpdate => {
                self.trigger_update_on_selected().await?;
            }
            Action::TriggerReboot => {
                self.trigger_reboot_on_selected().await?;
            }
            Action::RetryHost => {
                self.retry_selected_host().await?;
            }
            Action::StartSearch => {
                self.search_active = true;
            }
            Action::SearchInput(c) => {
                if self.search_active {
                    self.search_query.push(c);
                }
            }
            Action::ClearSearch => {
                self.search_query.clear();
            }
            _ => {}
        }
        Ok(())
    }

    /// Get the currently selected host name
    pub fn selected_host_name(&self) -> Option<&str> {
        self.hosts.get(self.selected_host).map(|h| h.name.as_str())
    }

    /// Load details for the selected host
    async fn load_selected_host_details(&mut self) -> Result<()> {
        if let (Some(client), Some(name)) = (&self.http_client, self.selected_host_name()) {
            match client.get_host(name).await {
                Ok(details) => {
                    self.host_details = Some(details);
                }
                Err(e) => {
                    self.log_event(&format!("Failed to load details: {e}"), EventLevel::Error);
                }
            }
        }
        Ok(())
    }

    /// Trigger update on selected host
    async fn trigger_update_on_selected(&mut self) -> Result<()> {
        if let (Some(client), Some(name)) = (&self.http_client, self.selected_host_name()) {
            let name = name.to_string();
            self.log_event(&format!("Triggering update on {name}"), EventLevel::Info);
            match client.update_host_packages(&name, false).await {
                Ok(_) => {
                    self.log_event(&format!("Update started on {name}"), EventLevel::Success);
                }
                Err(e) => {
                    self.log_event(&format!("Update failed: {e}"), EventLevel::Error);
                }
            }
        }
        Ok(())
    }

    /// Trigger reboot on selected host
    async fn trigger_reboot_on_selected(&mut self) -> Result<()> {
        if let (Some(client), Some(name)) = (&self.http_client, self.selected_host_name()) {
            let name = name.to_string();
            self.log_event(&format!("Triggering reboot on {name}"), EventLevel::Info);
            match client.reboot_host(&name).await {
                Ok(_) => {
                    self.log_event(&format!("Reboot started on {name}"), EventLevel::Success);
                }
                Err(e) => {
                    self.log_event(&format!("Reboot failed: {e}"), EventLevel::Error);
                }
            }
        }
        Ok(())
    }

    /// Retry a failed host
    async fn retry_selected_host(&mut self) -> Result<()> {
        if let (Some(client), Some(name)) = (&self.http_client, self.selected_host_name()) {
            let name = name.to_string();
            self.log_event(&format!("Retrying {name}"), EventLevel::Info);
            match client.retry_host(&name).await {
                Ok(_) => {
                    self.log_event(&format!("Retry started on {name}"), EventLevel::Success);
                }
                Err(e) => {
                    self.log_event(&format!("Retry failed: {e}"), EventLevel::Error);
                }
            }
        }
        Ok(())
    }

    /// Get filtered hosts based on search query
    pub fn filtered_hosts(&self) -> Vec<&HostDisplay> {
        if self.search_query.is_empty() {
            self.hosts.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.hosts
                .iter()
                .filter(|h| h.name.to_lowercase().contains(&query))
                .collect()
        }
    }
}
```

### Task 1.6: Create Config Module

**File**: `src/config.rs`

```rust
//! TUI configuration

use ratatui::style::{Color, Modifier, Style};

/// State colors
pub fn state_color(state: &str) -> Color {
    match state.to_lowercase().as_str() {
        "idle" => Color::Green,
        "querying" => Color::Yellow,
        "pendingupdates" | "pending_updates" => Color::Yellow,
        "updating" => Color::Blue,
        "waitingreboot" | "waiting_reboot" => Color::Cyan,
        "rebooting" => Color::Magenta,
        "verifying" => Color::Cyan,
        "failed" => Color::Red,
        "offline" => Color::DarkGray,
        _ => Color::White,
    }
}

/// State symbol
pub fn state_symbol(state: &str, tick: u64) -> &'static str {
    match state.to_lowercase().as_str() {
        "idle" => "●",
        "querying" => if tick % 4 < 2 { "◐" } else { "◑" },
        "pendingupdates" | "pending_updates" => "●",
        "updating" => match tick % 4 {
            0 => "◐",
            1 => "◓",
            2 => "◑",
            _ => "◒",
        },
        "waitingreboot" | "waiting_reboot" => "◎",
        "rebooting" => if tick % 4 < 2 { "◐" } else { "◑" },
        "verifying" => if tick % 4 < 2 { "◐" } else { "◑" },
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
```

**Acceptance criteria**:
- [ ] Terminal initializes and restores correctly
- [ ] App runs and quits with 'q'
- [ ] Tick events fire at configured rate
- [ ] Basic event handling works

---

## Phase 2: Core UI (3 hours)

### Task 2.1: Create UI Module Structure

**File**: `src/ui/mod.rs`

```rust
//! UI rendering modules

mod hosts;
mod details;
mod events;
mod help;
mod statusbar;
mod layout;

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
```

### Task 2.2: Create Layout Calculator

**File**: `src/ui/layout.rs`

```rust
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
            Constraint::Min(10),      // Content
            Constraint::Length(1),    // Status bar
        ])
        .split(area);

    let content_area = vertical[0];
    let statusbar = vertical[1];

    // Content: hosts list (left) + details/events (right)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // Host list
            Constraint::Percentage(50),  // Details + events
        ])
        .split(content_area);

    let hosts = horizontal[0];

    // Right panel: details (top) + events (bottom)
    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),  // Details
            Constraint::Percentage(40),  // Events
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
```

### Task 2.3: Create Host List Widget

**File**: `src/ui/hosts.rs`

```rust
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

            let cells = vec![
                Cell::from(host.name.clone()),
                Cell::from(format!("{} {}", state_symbol, host.state))
                    .style(Style::default().fg(state_color)),
                Cell::from(host.os.clone()),
                Cell::from(
                    host.packages
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "--".to_string()),
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
        format!(" Hosts (/{}) ", app.search_query)
    } else {
        format!(" Hosts ({}) ", hosts.len())
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .row_highlight_style(config::selected_style())
        .highlight_symbol("▸ ");

    // Render with state for selection
    let mut state = TableState::default();
    state.select(Some(app.selected_host));

    frame.render_stateful_widget(table, area, &mut state);
}
```

### Task 2.4: Create Details Panel

**File**: `src/ui/details.rs`

```rust
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
        Some(name) => format!(" Host: {} ", name),
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
        lines.push(format!("Name: {}", name));
    }
    if let Some(state) = details.get("state").and_then(|v| v.as_str()) {
        lines.push(format!("State: {}", state));
    }
    if let Some(addr) = details.get("addr").and_then(|v| v.as_str()) {
        lines.push(format!("Address: {}", addr));
    }

    lines.push(String::new());

    // System info
    if let Some(system) = details.get("system") {
        if let Some(os) = system.get("os_name").and_then(|v| v.as_str()) {
            let version = system.get("os_version").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("OS: {} {}", os, version));
        }
        if let Some(hostname) = system.get("hostname").and_then(|v| v.as_str()) {
            lines.push(format!("Hostname: {}", hostname));
        }
        if let Some(uptime) = system.get("uptime_seconds").and_then(|v| v.as_u64()) {
            lines.push(format!("Uptime: {}", format_uptime(uptime)));
        }
    }

    lines.push(String::new());

    // Upgradable packages
    if let Some(packages) = details.get("upgradable_packages").and_then(|v| v.as_array()) {
        lines.push(format!("Upgradable Packages: {}", packages.len()));
        for (i, pkg) in packages.iter().take(10).enumerate() {
            if let Some(name) = pkg.get("name").and_then(|v| v.as_str()) {
                let from = pkg.get("current_version").and_then(|v| v.as_str()).unwrap_or("?");
                let to = pkg.get("new_version").and_then(|v| v.as_str()).unwrap_or("?");
                let prefix = if i == packages.len().min(10) - 1 { "└──" } else { "├──" };
                lines.push(format!("  {} {} ({} → {})", prefix, name, from, to));
            }
        }
        if packages.len() > 10 {
            lines.push(format!("  ... ({} more)", packages.len() - 10));
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
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
```

### Task 2.5: Create Events Panel

**File**: `src/ui/events.rs`

```rust
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

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Events ")
                .borders(Borders::ALL)
                .border_style(border_style),
        );

    frame.render_widget(list, area);
}
```

### Task 2.6: Create Status Bar

**File**: `src/ui/statusbar.rs`

```rust
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
        ConnectionState::Reconnecting { attempt } => {
            (format!("◐ Reconnecting ({})", attempt).leak(), Color::Yellow)
        }
    };

    let keybindings = "[j/k] Navigate  [Enter] Details  [u] Update  [r] Reboot  [?] Help  [q] Quit";

    let status_line = Line::from(vec![
        Span::styled(connection_status.0, Style::default().fg(connection_status.1)),
        Span::raw("  │  "),
        Span::styled(keybindings, Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(status_line);
    frame.render_widget(paragraph, area);
}
```

### Task 2.7: Create Help Popup

**File**: `src/ui/help.rs`

```rust
//! Help popup widget

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Render the help popup
pub fn render(frame: &mut Frame) {
    let help_text = r#"
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
"#;

    // Calculate popup area (centered, 50x20)
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
```

**Acceptance criteria**:
- [ ] Host list table renders correctly
- [ ] Details panel shows selected host info
- [ ] Event log displays recent events
- [ ] Status bar shows connection state and keybindings
- [ ] Help popup displays on '?'
- [ ] Colors and styling work

---

## Phase 3: WebSocket Integration (2 hours)

### Task 3.1: Improve WebSocket Handling in App

**Update `src/app.rs`** to use tendhost-client properly with async channels:

```rust
// Add to App struct:
/// WebSocket task handle
ws_task: Option<tokio::task::JoinHandle<()>>,
/// Channel for receiving WS events
ws_rx: Option<tokio::sync::mpsc::UnboundedReceiver<WsEvent>>,

// Update connect method to spawn WS task:
pub async fn connect(&mut self) -> Result<()> {
    self.connection_state = ConnectionState::Connecting;

    // Create HTTP client
    let http_client = HttpClient::new(&self.server_url)?;
    self.http_client = Some(http_client.clone());

    // Load initial host list
    self.load_hosts().await?;

    // Spawn WebSocket task
    let ws_url = self.server_url.replace("http", "ws") + "/ws/events";
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    self.ws_rx = Some(rx);

    let ws_task = tokio::spawn(async move {
        loop {
            match WsClient::connect(&ws_url).await {
                Ok(mut client) => {
                    while let Some(event) = client.recv().await {
                        if tx.send(event).is_err() {
                            return; // Receiver dropped
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("WebSocket connection failed: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    self.ws_task = Some(ws_task);
    self.connection_state = ConnectionState::Connected;
    self.log_event("Connected to daemon", EventLevel::Success);

    Ok(())
}

// Update process_ws_events:
pub async fn process_ws_events(&mut self) -> Result<()> {
    if let Some(rx) = &mut self.ws_rx {
        while let Ok(event) = rx.try_recv() {
            self.handle_ws_event(event);
        }
    }
    Ok(())
}
```

### Task 3.2: Add Reconnection Logic

**Add to `src/app.rs`**:

```rust
impl App {
    /// Check connection health and reconnect if needed
    pub async fn check_connection(&mut self) -> Result<()> {
        // Check if WS task is still running
        if let Some(task) = &self.ws_task {
            if task.is_finished() {
                self.connection_state = ConnectionState::Reconnecting { attempt: 1 };
                self.log_event("WebSocket disconnected, reconnecting...", EventLevel::Warning);
                
                // Restart connection
                self.ws_task = None;
                self.connect().await?;
            }
        }
        Ok(())
    }
}
```

### Task 3.3: Update Main Loop for Connection Checks

**Update `src/main.rs`**:

```rust
// In run_app function, add periodic connection check:
let mut check_connection_interval = tokio::time::interval(std::time::Duration::from_secs(10));

loop {
    tokio::select! {
        // Handle terminal events
        event = events.next() => {
            if let Some(event) = event {
                let action = match event {
                    event::Event::Key(key) => event::key_to_action(key),
                    event::Event::Resize(_, _) => action::Action::Render,
                    event::Event::Tick => action::Action::Tick,
                };
                app.handle_action(action).await?;
            }
        }
        // Check connection periodically
        _ = check_connection_interval.tick() => {
            app.check_connection().await?;
        }
    }

    // Process WebSocket events
    app.process_ws_events().await?;

    // Draw UI
    terminal.draw(|frame| ui::render(frame, app))?;

    if app.should_quit() {
        break;
    }
}
```

**Acceptance criteria**:
- [ ] WebSocket connects on startup
- [ ] Events update host states in real-time
- [ ] Event log shows incoming events
- [ ] Reconnection works on disconnect

---

## Phase 4: Navigation & Selection (1.5 hours)

### Task 4.1: Improve Selection Handling

Already covered in Phase 1 App implementation. Enhance with:
- Page up/down support
- Better bounds checking
- Scroll state for long lists

### Task 4.2: Add Search Functionality

**Update `src/event.rs`** for search mode:

```rust
/// Convert key to action, considering current mode
pub fn key_to_action(key: KeyEvent, search_active: bool) -> Action {
    if search_active {
        match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::Back, // Confirm search
            KeyCode::Backspace => Action::SearchBackspace,
            KeyCode::Char(c) => Action::SearchInput(c),
            _ => Action::None,
        }
    } else {
        // Normal mode handling (existing code)
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            // ... rest of existing code
        }
    }
}
```

**Update main.rs to pass search state:**

```rust
let action = match event {
    event::Event::Key(key) => event::key_to_action(key, app.search_active),
    // ...
};
```

**Acceptance criteria**:
- [ ] j/k navigation works
- [ ] g/G jump to first/last
- [ ] Search filters host list
- [ ] Tab switches focus between panels

---

## Phase 5: Host Details Panel (1.5 hours)

### Task 5.1: Enhanced Details Display

Already covered in Phase 2. Add:
- Memory/CPU info display
- Docker container list
- Scrolling for long content

### Task 5.2: Inventory Refresh

**Add to `src/app.rs`**:

```rust
/// Refresh inventory for selected host
async fn refresh_selected_inventory(&mut self) -> Result<()> {
    if let (Some(client), Some(name)) = (&self.http_client, self.selected_host_name()) {
        let name = name.to_string();
        self.log_event(&format!("Refreshing inventory for {name}"), EventLevel::Info);
        match client.get_host_inventory(&name).await {
            Ok(inventory) => {
                // Merge inventory into host details
                self.log_event(&format!("Inventory refreshed for {name}"), EventLevel::Success);
                // Update host_details with new inventory data
            }
            Err(e) => {
                self.log_event(&format!("Inventory refresh failed: {e}"), EventLevel::Error);
            }
        }
    }
    Ok(())
}
```

**Acceptance criteria**:
- [ ] Enter loads full host details
- [ ] Details show system info, packages
- [ ] 'i' refreshes inventory

---

## Phase 6: Actions (1.5 hours)

### Task 6.1: Confirmation Dialogs

**Create `src/ui/confirm.rs`**:

```rust
//! Confirmation dialog widget

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Confirmation dialog state
pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Update(String),
    Reboot(String),
    FleetUpdate,
}

/// Render confirmation dialog
pub fn render(frame: &mut Frame, dialog: &ConfirmDialog) {
    let area = frame.area();
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 7;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let text = format!(
        "{}\n\n[y] Confirm  [n] Cancel",
        dialog.message
    );

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(format!(" {} ", dialog.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}
```

### Task 6.2: Fleet Update Handling

**Add to `src/app.rs`**:

```rust
/// Trigger fleet update
async fn trigger_fleet_update(&mut self) -> Result<()> {
    if let Some(client) = &self.http_client {
        self.log_event("Triggering fleet update", EventLevel::Info);
        let request = tendhost_api::requests::FleetUpdateRequest {
            batch_size: 2,
            delay_ms: 30000,
            filter: None,
        };
        match client.update_fleet(request).await {
            Ok(_) => {
                self.log_event("Fleet update started", EventLevel::Success);
            }
            Err(e) => {
                self.log_event(&format!("Fleet update failed: {e}"), EventLevel::Error);
            }
        }
    }
    Ok(())
}
```

**Acceptance criteria**:
- [ ] 'u' triggers update on selected host
- [ ] 'U' triggers fleet update
- [ ] 'r' triggers reboot
- [ ] 'R' retries failed host
- [ ] 'a' acknowledges failure
- [ ] Dangerous actions show confirmation

---

## Phase 7: Polish (2 hours)

### Task 7.1: Animation Improvements

Already have tick-based animation. Enhance with:
- Smoother state transitions
- Loading indicators during HTTP calls

### Task 7.2: Error Toast System

**Add to `src/app.rs`**:

```rust
/// Toast notification
pub struct Toast {
    pub message: String,
    pub level: EventLevel,
    pub expires_at: std::time::Instant,
}

impl App {
    /// Show a toast notification
    pub fn show_toast(&mut self, message: &str, level: EventLevel, duration: Duration) {
        self.toast = Some(Toast {
            message: message.to_string(),
            level,
            expires_at: std::time::Instant::now() + duration,
        });
    }

    /// Clear expired toasts
    pub fn clear_expired_toasts(&mut self) {
        if let Some(toast) = &self.toast {
            if std::time::Instant::now() > toast.expires_at {
                self.toast = None;
            }
        }
    }
}
```

**Create `src/ui/toast.rs`**:

```rust
//! Toast notification widget

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{EventLevel, Toast};

/// Render toast notification
pub fn render(frame: &mut Frame, toast: &Toast) {
    let area = frame.area();
    let width = (toast.message.len() + 4).min(area.width as usize - 4) as u16;
    let x = area.width.saturating_sub(width) - 2;
    let y = 1;
    let toast_area = Rect::new(x, y, width, 3);

    frame.render_widget(Clear, toast_area);

    let color = match toast.level {
        EventLevel::Info => Color::Blue,
        EventLevel::Success => Color::Green,
        EventLevel::Warning => Color::Yellow,
        EventLevel::Error => Color::Red,
    };

    let paragraph = Paragraph::new(toast.message.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, toast_area);
}
```

### Task 7.3: Final Cleanup

- Add doc comments to all public items
- Run clippy pedantic
- Run cargo fmt
- Test with real daemon

**Acceptance criteria**:
- [ ] Help popup shows all keybindings
- [ ] Search filters host list interactively
- [ ] Animations are smooth
- [ ] Error toasts appear and disappear
- [ ] No clippy warnings
- [ ] Documentation complete

---

## Summary

### Files Created/Modified

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modified | Add crossterm, clap, chrono, unicode-width |
| `src/main.rs` | Rewritten | Entry point, terminal init, main loop |
| `src/app.rs` | Created | Application state and logic |
| `src/action.rs` | Created | Action type definitions |
| `src/event.rs` | Created | Event handling |
| `src/config.rs` | Created | Colors and styles |
| `src/ui/mod.rs` | Created | UI module exports |
| `src/ui/layout.rs` | Created | Layout calculations |
| `src/ui/hosts.rs` | Created | Host list table |
| `src/ui/details.rs` | Created | Host details panel |
| `src/ui/events.rs` | Created | Event log panel |
| `src/ui/statusbar.rs` | Created | Status bar |
| `src/ui/help.rs` | Created | Help popup |
| `src/ui/toast.rs` | Created | Toast notifications |
| `src/ui/confirm.rs` | Created | Confirmation dialogs |

### Time Breakdown

| Phase | Task | Time |
|-------|------|------|
| Phase 1 | Foundation | 2 hours |
| Phase 2 | Core UI | 3 hours |
| Phase 3 | WebSocket Integration | 2 hours |
| Phase 4 | Navigation & Selection | 1.5 hours |
| Phase 5 | Host Details Panel | 1.5 hours |
| Phase 6 | Actions | 1.5 hours |
| Phase 7 | Polish | 2 hours |
| **Total** | | **~13.5 hours** |

### Testing Plan

1. Manual testing with running daemon
2. Test keyboard navigation
3. Test all actions (update, reboot, etc.)
4. Test connection loss and reconnection
5. Test with many hosts (scrolling)
6. Test terminal resize

### Dependencies

- **Requires**: `tendhost-client` (HTTP + WebSocket)
- **Requires**: `tendhost-api` (event types)
- **Requires**: Running `tendhost` daemon for testing

---

## Next Steps After Completion

1. Add mouse support
2. Add configuration file support
3. Add host grouping view
4. Add fleet update wizard with progress
5. Add export functionality
6. Add themes/color customization

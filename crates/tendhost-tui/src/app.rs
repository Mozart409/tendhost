//! Application state and logic

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use color_eyre::Result;
use tendhost_api::events::WsEvent;
use tendhost_client::{HttpClient, WsClient};

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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct HostDisplay {
    pub name: String,
    pub state: String,
    pub os: String,
    pub packages: Option<u32>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Application state
#[allow(dead_code)]
pub struct App {
    /// Server URL
    server_url: String,
    /// HTTP client
    http_client: Option<HttpClient>,
    /// WebSocket client
    ws_client: Option<WsClient>,
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
        self.http_client = Some(http_client);

        // Load initial host list
        self.load_hosts().await?;

        // Connect WebSocket for event receiving
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
                self.connection_state = ConnectionState::Connected;
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
                        .map(|h| HostDisplay {
                            name: h
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            state: h
                                .get("state")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            os: h
                                .get("os")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            packages: h
                                .get("upgradable_packages")
                                .and_then(serde_json::Value::as_u64)
                                .and_then(|v| u32::try_from(v).ok()),
                            last_updated: None,
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
        // Collect events first to avoid borrow issues
        let mut events = Vec::new();
        if let Some(ws_client) = &mut self.ws_client {
            // Non-blocking check for events
            while let Ok(event) = tokio::time::timeout(std::time::Duration::from_millis(1), async {
                ws_client.recv().await
            })
            .await
            {
                if let Some(event) = event {
                    events.push(event);
                }
            }
        }

        // Handle collected events
        for event in &events {
            self.handle_ws_event(event);
        }
        Ok(())
    }

    /// Handle a WebSocket event
    fn handle_ws_event(&mut self, event: &WsEvent) {
        match event {
            WsEvent::HostStateChanged { host, from, to } => {
                self.log_event(&format!("{host}: {from} -> {to}"), EventLevel::Info);
                // Update host state in list
                if let Some(h) = self.hosts.iter_mut().find(|h| h.name == *host) {
                    h.state.clone_from(to);
                }
            }
            WsEvent::UpdateProgress {
                host,
                package,
                progress,
            } => {
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
            Action::SearchBackspace => {
                if self.search_active {
                    self.search_query.pop();
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
        let client = self.http_client.clone();
        let name = self
            .selected_host_name()
            .map(std::string::ToString::to_string);

        if let (Some(client), Some(name)) = (client, name) {
            match client.get_host(&name).await {
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
        let client = self.http_client.clone();
        let name = self
            .selected_host_name()
            .map(std::string::ToString::to_string);

        if let (Some(client), Some(name)) = (client, name) {
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
        let client = self.http_client.clone();
        let name = self
            .selected_host_name()
            .map(std::string::ToString::to_string);

        if let (Some(client), Some(name)) = (client, name) {
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
        let client = self.http_client.clone();
        let name = self
            .selected_host_name()
            .map(std::string::ToString::to_string);

        if let (Some(client), Some(name)) = (client, name) {
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

    /// Check connection health and reconnect if needed
    #[allow(clippy::unused_self)]
    pub fn check_connection(&mut self) {
        // For now, just a placeholder. Could add reconnection logic later.
    }
}

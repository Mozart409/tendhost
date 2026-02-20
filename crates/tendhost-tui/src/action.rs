//! User actions for the TUI application

/// Actions that can be performed in the application
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    /// Backspace in search
    SearchBackspace,
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

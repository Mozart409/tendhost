//! WebSocket client for tendhost daemon

use tendhost_api::events::WsEvent;

/// WebSocket client for receiving live events from tendhost daemon
pub struct WsClient {
    url: String,
}

impl WsClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

//! WebSocket client for tendhost daemon

/// WebSocket client for receiving live events from tendhost daemon
pub struct WsClient {
    #[allow(dead_code)]
    url: String,
}

impl WsClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

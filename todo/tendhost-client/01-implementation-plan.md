# tendhost-client: Implementation Plan

## Overview

Implement a Rust client library for the tendhost daemon with both HTTP (REST API) and WebSocket (real-time events) support.

**Estimated time**: ~4 hours

## Prerequisites

- ✅ `tendhost-api` types defined
- ✅ Daemon API specification in GOALS.md
- ✅ Skeleton code exists

## Phase 1: Error Types (30 min)

### Task 1.1: Implement `ClientError`

**File**: `src/error.rs`

```rust
//! Error types for the tendhost client

use thiserror::Error;

/// Errors that can occur when using the tendhost client
#[derive(Error, Debug)]
pub enum ClientError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// Request timeout
    #[error("Request timed out")]
    Timeout,

    /// API returned an error status
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    /// Connection closed unexpectedly
    #[error("Connection closed: {0}")]
    ConnectionClosed(String),

    /// Invalid response format
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Result type for client operations
pub type Result<T> = std::result::Result<T, ClientError>;
```

## Phase 2: HTTP Client (2 hours)

### Task 2.1: Core HTTP Client

**File**: `src/http.rs`

```rust
//! HTTP client for tendhost daemon

use reqwest::Client;
use serde::de::DeserializeOwned;
use url::Url;

use tendhost_api::{
    events::WsEvent,
    requests::{FleetUpdateFilter, FleetUpdateRequest, UpdateRequest},
    responses::{HealthResponse, PaginatedResponse, Pagination},
};

use crate::error::{ClientError, Result};

/// HTTP client for communicating with tendhost daemon
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    base_url: Url,
}

impl HttpClient {
    /// Create a new HTTP client
    ///
    /// # Example
    /// ```no_run
    /// use tendhost_client::HttpClient;
    ///
    /// let client = HttpClient::new("http://localhost:8080")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(base_url: impl AsRef<str>) -> Result<Self> {
        let base_url = Url::parse(base_url.as_ref())?;
        Ok(Self {
            client: Client::new(),
            base_url,
        })
    }

    /// Create a new HTTP client with custom `reqwest::Client`
    pub fn with_client(base_url: impl AsRef<str>, client: Client) -> Result<Self> {
        let base_url = Url::parse(base_url.as_ref())?;
        Ok(Self { client, base_url })
    }

    /// Build a full URL from a path
    fn url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|e| ClientError::Url(e))
    }

    /// Perform a GET request and deserialize the response
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path)?;
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ClientError::Api { status, message });
        }

        Ok(response.json().await?)
    }

    /// Perform a POST request with JSON body
    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: impl serde::Serialize,
    ) -> Result<T> {
        let url = self.url(path)?;
        let response = self.client.post(url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ClientError::Api { status, message });
        }

        Ok(response.json().await?)
    }

    /// Perform a PATCH request with JSON body
    async fn patch<T: DeserializeOwned>(
        &self,
        path: &str,
        body: impl serde::Serialize,
    ) -> Result<T> {
        let url = self.url(path)?;
        let response = self.client.patch(url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ClientError::Api { status, message });
        }

        Ok(response.json().await?)
    }

    /// Perform a DELETE request
    async fn delete(&self, path: &str) -> Result<()> {
        let url = self.url(path)?;
        let response = self.client.delete(url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ClientError::Api { status, message });
        }

        Ok(())
    }

    // System endpoints

    /// Get daemon health status
    ///
    /// # Example
    /// ```no_run
    /// # use tendhost_client::HttpClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = HttpClient::new("http://localhost:8080")?;
    /// let health = client.health().await?;
    /// println!("Status: {}", health.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health(&self) -> Result<HealthResponse> {
        self.get("/health").await
    }

    // Host endpoints - will be implemented in Task 2.2
}
```

### Task 2.2: Host Endpoints

**Add to `src/http.rs`:**

```rust
// Add after system endpoints comment

// Note: These types will be defined based on daemon implementation
// For now using placeholder serde_json::Value

use serde_json::Value;

impl HttpClient {
    /// List all hosts with optional filtering and pagination
    ///
    /// Use `ListHostsBuilder` for a more ergonomic API.
    pub fn list_hosts(&self) -> ListHostsBuilder {
        ListHostsBuilder::new(self.clone())
    }

    /// Get a single host by name
    pub async fn get_host(&self, name: &str) -> Result<Value> {
        self.get(&format!("/hosts/{name}")).await
    }

    /// Create a new host
    pub async fn create_host(&self, config: Value) -> Result<Value> {
        self.post("/hosts", config).await
    }

    /// Update host configuration
    pub async fn update_host(&self, name: &str, config: Value) -> Result<Value> {
        self.patch(&format!("/hosts/{name}"), config).await
    }

    /// Delete a host
    pub async fn delete_host(&self, name: &str) -> Result<()> {
        self.delete(&format!("/hosts/{name}")).await
    }

    /// Trigger package update on a host
    pub async fn update_host_packages(&self, name: &str, dry_run: bool) -> Result<Value> {
        let request = UpdateRequest { dry_run };
        self.post(&format!("/hosts/{name}/update"), request).await
    }

    /// Trigger host reboot
    pub async fn reboot_host(&self, name: &str) -> Result<Value> {
        self.post(&format!("/hosts/{name}/reboot"), serde_json::json!({}))
            .await
    }

    /// Retry a failed host
    pub async fn retry_host(&self, name: &str) -> Result<Value> {
        self.post(&format!("/hosts/{name}/retry"), serde_json::json!({}))
            .await
    }

    /// Acknowledge a host failure
    pub async fn acknowledge_host(&self, name: &str) -> Result<Value> {
        self.post(&format!("/hosts/{name}/acknowledge"), serde_json::json!({}))
            .await
    }

    /// Get full osquery inventory for a host
    pub async fn get_host_inventory(&self, name: &str) -> Result<Value> {
        self.get(&format!("/hosts/{name}/inventory")).await
    }

    /// Trigger fleet-wide update
    pub async fn update_fleet(&self, request: FleetUpdateRequest) -> Result<Value> {
        self.post("/fleet/update", request).await
    }
}
```

### Task 2.3: Query Builder for List Hosts

**Add to `src/http.rs`:**

```rust
/// Builder for listing hosts with filters
#[derive(Debug, Clone)]
pub struct ListHostsBuilder {
    client: HttpClient,
    page: Option<u64>,
    per_page: Option<u64>,
    tags: Vec<String>,
    state: Option<String>,
    group: Option<String>,
    search: Option<String>,
}

impl ListHostsBuilder {
    fn new(client: HttpClient) -> Self {
        Self {
            client,
            page: None,
            per_page: None,
            tags: Vec::new(),
            state: None,
            group: None,
            search: None,
        }
    }

    /// Set page number (default: 1)
    pub fn page(mut self, page: u64) -> Self {
        self.page = Some(page);
        self
    }

    /// Set items per page (default: 50, max: 200)
    pub fn per_page(mut self, per_page: u64) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Add a tag filter (repeatable for AND logic)
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Filter by state (idle, updating, etc.)
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Filter by group name
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Search by hostname (prefix match)
    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Execute the request
    pub async fn send(self) -> Result<PaginatedResponse<Value>> {
        let mut url = self.client.url("/hosts")?;

        {
            let mut query = url.query_pairs_mut();
            if let Some(page) = self.page {
                query.append_pair("page", &page.to_string());
            }
            if let Some(per_page) = self.per_page {
                query.append_pair("per_page", &per_page.to_string());
            }
            for tag in &self.tags {
                query.append_pair("tag", tag);
            }
            if let Some(state) = &self.state {
                query.append_pair("state", state);
            }
            if let Some(group) = &self.group {
                query.append_pair("group", group);
            }
            if let Some(search) = &self.search {
                query.append_pair("search", search);
            }
        }

        let response = self.client.client.get(url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ClientError::Api { status, message });
        }

        Ok(response.json().await?)
    }
}
```

## Phase 3: WebSocket Client (1.5 hours)

### Task 3.1: WebSocket Client with Auto-Reconnection

**File**: `src/ws.rs`

```rust
//! WebSocket client for tendhost daemon

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use tendhost_api::events::WsEvent;

use crate::error::{ClientError, Result};

/// WebSocket client for receiving live events from tendhost daemon
#[derive(Debug)]
pub struct WsClient {
    url: Url,
    receiver: mpsc::Receiver<WsEvent>,
    _task_handle: tokio::task::JoinHandle<()>,
}

impl WsClient {
    /// Connect to the WebSocket endpoint
    ///
    /// Automatically reconnects on connection loss with exponential backoff.
    ///
    /// # Example
    /// ```no_run
    /// use tendhost_client::WsClient;
    /// use tendhost_api::events::WsEvent;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = WsClient::connect("ws://localhost:8080/ws/events").await?;
    ///
    /// while let Some(event) = client.recv().await {
    ///     match event {
    ///         WsEvent::HostStateChanged { host, from, to } => {
    ///             println!("{host}: {from} -> {to}");
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(url: impl AsRef<str>) -> Result<Self> {
        let url = Url::parse(url.as_ref())?;
        let (tx, rx) = mpsc::channel(100);

        let task_url = url.clone();
        let task_handle = tokio::spawn(async move {
            Self::connection_loop(task_url, tx).await;
        });

        Ok(Self {
            url,
            receiver: rx,
            _task_handle: task_handle,
        })
    }

    /// Receive the next event from the stream
    ///
    /// Returns `None` when the connection is closed and cannot be reconnected.
    pub async fn recv(&mut self) -> Option<WsEvent> {
        self.receiver.recv().await
    }

    /// Connection loop with auto-reconnection
    async fn connection_loop(url: Url, tx: mpsc::Sender<WsEvent>) {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);

        loop {
            match Self::connect_and_receive(&url, &tx).await {
                Ok(_) => {
                    // Connection closed gracefully
                    tracing::info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    tracing::warn!("WebSocket error: {}, reconnecting in {:?}", e, backoff);
                    sleep(backoff).await;

                    // Exponential backoff
                    backoff = (backoff * 2).min(max_backoff);
                }
            }
        }
    }

    /// Connect and receive messages
    async fn connect_and_receive(url: &Url, tx: &mpsc::Sender<WsEvent>) -> Result<()> {
        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(|e| ClientError::WebSocket(e.to_string()))?;

        tracing::info!("WebSocket connected to {}", url);

        let (mut _write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let msg = msg.map_err(|e| ClientError::WebSocket(e.to_string()))?;

            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WsEvent>(&text) {
                        Ok(event) => {
                            if tx.send(event).await.is_err() {
                                // Receiver dropped, exit
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse event: {}", e);
                        }
                    }
                }
                Message::Close(_) => {
                    return Err(ClientError::ConnectionClosed("Server closed connection".into()));
                }
                Message::Ping(_) | Message::Pong(_) => {
                    // Handled automatically by tungstenite
                }
                _ => {}
            }
        }

        Err(ClientError::ConnectionClosed("Stream ended".into()))
    }
}
```

## Phase 4: Library Exports & Tests (30 min)

### Task 4.1: Update `lib.rs`

**File**: `src/lib.rs`

```rust
//! tendhost-client: HTTP and WebSocket client library
//!
//! Provides both HTTP and WebSocket clients for communicating with the tendhost daemon.
//!
//! # Examples
//!
//! ## HTTP Client
//!
//! ```no_run
//! use tendhost_client::HttpClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = HttpClient::new("http://localhost:8080")?;
//!
//! // Get health
//! let health = client.health().await?;
//! println!("Status: {}", health.status);
//!
//! // List hosts with filters
//! let hosts = client.list_hosts()
//!     .page(1)
//!     .per_page(50)
//!     .tag("production")
//!     .send()
//!     .await?;
//!
//! // Get single host
//! let host = client.get_host("debian-vm").await?;
//!
//! // Trigger update
//! client.update_host_packages("debian-vm", false).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## WebSocket Client
//!
//! ```no_run
//! use tendhost_client::WsClient;
//! use tendhost_api::events::WsEvent;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = WsClient::connect("ws://localhost:8080/ws/events").await?;
//!
//! while let Some(event) = client.recv().await {
//!     match event {
//!         WsEvent::HostStateChanged { host, from, to } => {
//!             println!("{host}: {from} -> {to}");
//!         }
//!         WsEvent::UpdateProgress { host, package, progress } => {
//!             println!("{host}: {package} {progress}%");
//!         }
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod http;
pub mod ws;

pub use error::{ClientError, Result};
pub use http::{HttpClient, ListHostsBuilder};
pub use ws::WsClient;
```

### Task 4.2: Add Dependencies

**Update `Cargo.toml`:**

```toml
[dependencies]
reqwest = { workspace = true }
tokio = { workspace = true }
tokio-tungstenite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tendhost-api = { workspace = true }

# Additional dependencies
url = "2.5"
futures = "0.3"
tracing = "0.1"
```

### Task 4.3: Add Basic Tests

**Add to `src/http.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HttpClient::new("http://localhost:8080");
        assert!(client.is_ok());
    }

    #[test]
    fn test_invalid_url() {
        let client = HttpClient::new("not a url");
        assert!(client.is_err());
    }

    #[test]
    fn test_url_building() {
        let client = HttpClient::new("http://localhost:8080").unwrap();
        let url = client.url("/hosts").unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080/hosts");
    }

    #[test]
    fn test_list_hosts_builder() {
        let client = HttpClient::new("http://localhost:8080").unwrap();
        let builder = client.list_hosts()
            .page(2)
            .per_page(100)
            .tag("production")
            .state("idle");

        assert_eq!(builder.page, Some(2));
        assert_eq!(builder.per_page, Some(100));
        assert_eq!(builder.tags, vec!["production"]);
        assert_eq!(builder.state, Some("idle".to_string()));
    }
}
```

**Add to `src/ws.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing() {
        let url = Url::parse("ws://localhost:8080/ws/events");
        assert!(url.is_ok());
    }

    #[test]
    fn test_invalid_url() {
        let url = Url::parse("not a url");
        assert!(url.is_err());
    }
}
```

## Summary

### Files Modified/Created

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Modified | Add `url`, `futures`, `tracing` dependencies |
| `src/lib.rs` | Modified | Public exports and documentation |
| `src/error.rs` | Created | `ClientError` enum |
| `src/http.rs` | Rewritten | Full HTTP client with all endpoints |
| `src/ws.rs` | Rewritten | WebSocket client with auto-reconnection |

### Testing Plan

1. Unit tests for URL building and builders
2. Integration tests require running daemon (optional for now)
3. Manual testing with real daemon

### Time Breakdown

| Phase | Task | Time |
|-------|------|------|
| Phase 1 | Error types | 30 min |
| Phase 2 | HTTP client core | 45 min |
| Phase 2 | Host endpoints | 45 min |
| Phase 2 | Query builder | 30 min |
| Phase 3 | WebSocket client | 1.5 hours |
| Phase 4 | Library exports & tests | 30 min |
| **Total** | | **~4 hours** |

## Next Steps After Completion

1. Test with running daemon
2. Add more comprehensive error messages
3. Add request/response logging (tracing)
4. Add timeout configuration
5. Add TLS/authentication support (if needed)
6. Create `tendhost-cli` using this client

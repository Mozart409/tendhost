# tendhost-client: Reasoning & Design

## Purpose

The `tendhost-client` crate provides both HTTP and WebSocket client libraries for communicating with the tendhost daemon. It will be used by:
- `tendhost-cli` - Command-line interface
- `tendhost-tui` - Terminal UI with real-time updates
- External integrations - Any Rust program that wants to interact with tendhost

## Design Goals

1. **Type-safe API**: Use `tendhost-api` types for requests/responses
2. **Error handling**: Comprehensive error types with context
3. **Async-first**: Built on `tokio` and `reqwest`
4. **Reconnection**: WebSocket client with automatic reconnection
5. **Ergonomic**: Builder pattern for optional parameters
6. **Testable**: Mock-friendly design for testing

## Architecture

### HTTP Client (`HttpClient`)

The HTTP client wraps `reqwest::Client` and provides type-safe methods for all daemon endpoints:

**System endpoints:**
- `health()` - Get health status

**Host endpoints:**
- `list_hosts(query)` - List hosts with pagination/filtering
- `get_host(name)` - Get single host details
- `create_host(config)` - Add new host
- `update_host(name, config)` - Update host configuration
- `delete_host(name)` - Remove host
- `update_host_packages(name, dry_run)` - Trigger package update
- `reboot_host(name)` - Trigger host reboot
- `retry_host(name)` - Retry failed host
- `acknowledge_host(name)` - Acknowledge host failure
- `get_host_inventory(name)` - Get full osquery inventory

**Fleet endpoints:**
- `update_fleet(request)` - Trigger fleet-wide update

### WebSocket Client (`WsClient`)

The WebSocket client connects to `/ws/events` and provides:
- Event stream via `tokio::sync::mpsc` channel
- Auto-reconnection with exponential backoff
- Graceful shutdown
- Error handling and recovery

**Event types from `tendhost-api::events::WsEvent`:**
- `HostStateChanged` - State transitions
- `UpdateProgress` - Package update progress
- `UpdateCompleted` - Update finished
- `HostConnected` - Host came online
- `HostDisconnected` - Host went offline

### Error Handling

`ClientError` enum covers:
- `Http` - HTTP errors (status codes, network)
- `WebSocket` - WebSocket errors (connection, protocol)
- `Json` - Serialization/deserialization errors
- `Url` - Invalid URL errors
- `Timeout` - Request timeout errors

## Dependencies

Already in `Cargo.toml`:
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket client
- `serde` / `serde_json` - Serialization
- `thiserror` - Error types
- `tendhost-api` - Shared types

Additional needed:
- `url` - URL parsing/building
- `futures` - Stream utilities
- `tracing` - Logging

## API Examples

### HTTP Client

```rust
use tendhost_client::HttpClient;

let client = HttpClient::new("http://localhost:8080")?;

// Get health
let health = client.health().await?;

// List hosts
let hosts = client.list_hosts()
    .page(1)
    .per_page(50)
    .tag("critical")
    .state("idle")
    .send()
    .await?;

// Get single host
let host = client.get_host("debian-vm").await?;

// Trigger update
client.update_host_packages("debian-vm", false).await?;

// Fleet update
let request = FleetUpdateRequest {
    batch_size: 5,
    delay_ms: 5000,
    filter: Some(FleetUpdateFilter {
        tags: Some(vec!["production".into()]),
        groups: None,
        exclude_hosts: None,
    }),
};
client.update_fleet(request).await?;
```

### WebSocket Client

```rust
use tendhost_client::WsClient;
use tendhost_api::events::WsEvent;

let mut client = WsClient::connect("ws://localhost:8080/ws/events").await?;

while let Some(event) = client.recv().await {
    match event {
        WsEvent::HostStateChanged { host, from, to } => {
            println!("{host}: {from} -> {to}");
        }
        WsEvent::UpdateProgress { host, package, progress } => {
            println!("{host}: {package} {progress}%");
        }
        _ => {}
    }
}
```

## File Structure

```
tendhost-client/
├── Cargo.toml
└── src/
    ├── lib.rs          # Public exports
    ├── error.rs        # ClientError type
    ├── http.rs         # HttpClient + builders
    └── ws.rs           # WsClient + reconnection
```

## Implementation Phases

1. **Error types** - `ClientError` enum (30 min)
2. **HTTP client** - All REST endpoints (2 hours)
3. **WebSocket client** - Event streaming + reconnection (1.5 hours)
4. **Tests** - Unit tests and mock support (30 min)

**Total estimated time**: ~4 hours

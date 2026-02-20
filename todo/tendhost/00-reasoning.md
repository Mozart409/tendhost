# Reasoning: tendhost Daemon

## Overview

The `tendhost` daemon is the central binary that wires together all the library crates (core, exec, pkg, inventory) into a running service. It provides:

1. **HTTP API** via axum for REST endpoints
2. **WebSocket** for real-time event streaming
3. **Configuration loading** from `tendhost.toml`
4. **Actor initialization** using kameo framework
5. **OpenAPI documentation** via utoipa and Scalar

## Key Design Decisions

### 1. Application State Architecture

**Decision**: Use axum's `State` extractor with `Arc<AppState>` containing the `OrchestratorActor` reference.

**Rationale**:
- Axum requires `Send + Sync` for shared state
- `ActorRef<OrchestratorActor>` is cloneable and thread-safe
- Single orchestrator manages all host actors
- WebSocket subscribers get event receiver from orchestrator

```rust
pub struct AppState {
    pub orchestrator: ActorRef<OrchestratorActor>,
}
```

### 2. Configuration Strategy

**Decision**: Use a two-tier configuration approach:
1. `DaemonConfig` - server settings (bind address, TLS, auth)
2. `HostConfig` (from tendhost-core) - per-host settings

**Rationale**:
- Separation of concerns between daemon runtime and host management
- Host configs can be reloaded without restarting daemon (future)
- TOML format for human readability
- Environment variable overrides for containerized deployments

### 3. Error Handling Strategy

**Decision**: Use `color-eyre` for the binary, convert library errors at API boundaries.

**Rationale**:
- Binary crates can use color-eyre for rich error context
- API responses use typed error responses (JSON)
- Internal errors don't leak to API clients
- Proper HTTP status codes for different error types

### 4. Route Organization

**Decision**: Split routes into modules by resource (`hosts`, `fleet`, `ws`, `system`).

**Rationale**:
- Matches REST resource model
- Each module handles its own handlers and OpenAPI schemas
- Easy to add middleware per-route group
- Clear separation of concerns

```
api/
├── mod.rs          # Router composition
├── hosts.rs        # /hosts/* endpoints
├── fleet.rs        # /fleet/* endpoints  
├── ws.rs           # /ws/events WebSocket
├── system.rs       # /health, /docs, /openapi.json
└── middleware.rs   # Auth, logging, metrics
```

### 5. WebSocket Design

**Decision**: Use tokio broadcast channel from `OrchestratorActor` for event distribution.

**Rationale**:
- Actors emit events to broadcast channel
- WebSocket handler subscribes to channel
- Multiple WebSocket clients share same event stream
- No polling required - push-based architecture
- Channel capacity prevents slow consumers from blocking

### 6. Tracing Setup

**Decision**: Use `tracing-subscriber` with JSON output for structured logging.

**Rationale**:
- Structured logs for log aggregation (ELK, Loki)
- Span-based tracing for request flow
- Environment-based log level filtering
- Request ID propagation through actors

### 7. Graceful Shutdown

**Decision**: Implement proper shutdown sequence with actor cleanup.

**Rationale**:
- HTTP server stops accepting new connections
- In-flight requests complete (timeout)
- Orchestrator stops all host actors
- Connections drain gracefully
- Exit code indicates shutdown reason

## Dependencies Rationale

| Dependency | Purpose | Why This Choice |
|------------|---------|-----------------|
| `axum` | HTTP server | Tokio-native, type-safe extractors, tower middleware |
| `tokio-tungstenite` | WebSocket | Tokio-native, works with axum upgrade |
| `utoipa` | OpenAPI | Derive macros, integrates with axum |
| `utoipa-scalar` | API docs | Modern UI, better than Swagger UI |
| `color-eyre` | Errors | Rich backtraces, context chaining |
| `toml` | Config | Human-friendly, TOML is standard for Rust |
| `tracing-subscriber` | Logging | Structured logging, spans, env filter |

## API Design Principles

1. **RESTful Resources**: Hosts are resources, updates are actions on hosts
2. **Consistent Responses**: All responses follow same structure (data + pagination)
3. **Query Parameters**: Filtering via query params, not body
4. **WebSocket for Events**: Real-time updates via WebSocket, not polling
5. **OpenAPI First**: All endpoints documented with schemas

## Security Considerations

1. **Token Auth**: Optional bearer token authentication
2. **TLS**: Optional HTTPS/WSS support
3. **Audit Logging**: All operations logged with user context
4. **Input Validation**: All requests validated before processing
5. **Rate Limiting**: Consider adding for public deployments

## Future Considerations

1. **Configuration Reload**: SIGHUP to reload hosts without restart
2. **Metrics**: Prometheus endpoint for observability
3. **Health Checks**: Kubernetes-style liveness/readiness probes
4. **Clustering**: Multiple daemon instances (future, complex)

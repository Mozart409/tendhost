# Skeleton Status: tendhost Daemon

**Status**: ✅ **SKELETON COMPLETE**  
**Completed**: 2026-02-20  
**Type**: Minimal MVP - Runnable daemon with health endpoint

---

## Overview

A minimal, runnable skeleton of the tendhost daemon has been implemented. The daemon compiles, runs, and provides a basic health endpoint. This serves as a foundation for full API implementation.

---

## What's Implemented

### 1. Configuration Loading (`config.rs`)
**Status**: ✅ Minimal  
**Lines**: 93

- `Config` struct with daemon settings and host list
- `DaemonConfig` with bind address and log level
- `load()` method to parse from TOML file
- `load_default()` with environment variable and path fallback
- Default configuration when no file found

**TODO**: Full implementation needs:
- TLS configuration
- Authentication settings
- Default host settings
- Group definitions
- Audit logging config

### 2. Application State (`state.rs`)
**Status**: ✅ Complete for MVP  
**Lines**: 20

- `AppState` struct holding orchestrator and config
- Implements `Clone` for axum state sharing
- Proper `Arc` wrapping for config

### 3. API Error Types (`api/error.rs`)
**Status**: ✅ Minimal  
**Lines**: 49

- `ApiError` struct for JSON error responses
- `AppError` wrapper with HTTP status codes
- `IntoResponse` implementation for axum
- `internal()` helper for generic errors

**TODO**: Full implementation needs:
- All error variants (not_found, bad_request, conflict, etc.)
- Conversion from `CoreError`
- Proper status code mapping
- Error details field

### 4. System Endpoints (`api/system.rs`)
**Status**: ✅ Complete for MVP  
**Lines**: 18

- `health()` endpoint returning service status and version
- `HealthResponse` struct with serde derives
- Returns HTTP 200 with JSON payload

**TODO**: Full implementation needs:
- OpenAPI annotations
- Metrics endpoint
- Readiness/liveness probes
- Version info endpoint

### 5. Router (`router.rs`)
**Status**: ✅ Minimal  
**Lines**: 16

- Creates axum `Router` with health endpoint
- Attaches application state
- Ready for additional routes

**TODO**: Full implementation needs:
- All host endpoints (`/hosts/*`)
- Fleet endpoints (`/fleet/*`)
- WebSocket endpoint (`/ws/events`)
- Groups and tags endpoints
- OpenAPI documentation route (`/docs`, `/openapi.json`)
- Scalar UI integration

### 6. API Module Organization (`api/mod.rs`)
**Status**: ✅ Minimal  
**Lines**: 12

- Exports `error` and `system` modules
- Re-exports error types (marked `#[allow(unused)]`)
- Commented out placeholders for unimplemented modules

**TODO**: Implement:
- `hosts.rs` - Host management endpoints
- `fleet.rs` - Fleet operations endpoints
- `ws.rs` - WebSocket event streaming

### 7. Main Entry Point (`main.rs`)
**Status**: ✅ Complete for MVP  
**Lines**: 110

**Implemented**:
- Error handling setup with `color-eyre`
- Configuration loading
- Tracing initialization with env filter
- Orchestrator actor spawning (default/no-op factory)
- Application state creation
- HTTP server binding and listening
- Graceful shutdown signal handling (SIGTERM, SIGINT)
- Actor cleanup on shutdown

**TODO**: Full implementation needs:
- Host actor factory creation
- Host registration from config
- TLS support
- Authentication middleware
- Request logging middleware

---

## File Structure

```
crates/tendhost/src/
├── main.rs              ✅ Runnable daemon entry point
├── config.rs            ✅ Configuration loading (minimal)
├── state.rs             ✅ Application state
├── router.rs            ✅ HTTP router (minimal)
├── api/
│   ├── mod.rs           ✅ Module exports
│   ├── error.rs         ✅ Error types (minimal)
│   ├── system.rs        ✅ Health endpoint
│   ├── hosts.rs         ❌ NOT IMPLEMENTED
│   ├── fleet.rs         ❌ NOT IMPLEMENTED (empty file)
│   └── ws.rs            ❌ NOT IMPLEMENTED (empty file)
└── factory.rs           ❌ NOT CREATED (needs HostActorFactory impl)
```

---

## Dependencies Added

```toml
# New dependencies for skeleton
dirs = "5"              # Config file path resolution
kameo = "0.19"          # Actor spawning (was missing)
```

**Already present**:
- axum, tokio, tokio-tungstenite
- color-eyre, eyre
- serde, serde_json, toml
- utoipa, utoipa-scalar
- tracing, tracing-subscriber
- All tendhost-* crates

---

## Testing the Skeleton

### Run the daemon:
```bash
cd crates/tendhost
cargo run
```

**Expected output**:
```
2026-02-20T12:00:00.000Z  INFO tendhost: tendhost daemon starting (skeleton mode)...
2026-02-20T12:00:00.001Z  INFO tendhost: configuration loaded bind=127.0.0.1:8080
2026-02-20T12:00:00.002Z  INFO tendhost_core::actor::orchestrator: OrchestratorActor starting id=...
2026-02-20T12:00:00.003Z  INFO tendhost: orchestrator actor started
2026-02-20T12:00:00.004Z  INFO tendhost: HTTP server listening addr=127.0.0.1:8080
2026-02-20T12:00:00.005Z  INFO tendhost: Health endpoint available at http://127.0.0.1:8080/health
```

### Test health endpoint:
```bash
curl http://127.0.0.1:8080/health
```

**Expected response**:
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

### Graceful shutdown:
```bash
# Press Ctrl+C
^C
2026-02-20T12:05:00.000Z  INFO tendhost: shutdown signal received
2026-02-20T12:05:00.001Z  INFO tendhost: shutting down...
2026-02-20T12:05:00.002Z  INFO tendhost_core::actor::orchestrator: OrchestratorActor stopping reason=...
2026-02-20T12:05:00.003Z  INFO tendhost: shutdown complete
```

---

## Code Quality

### Compilation
✅ `cargo check -p tendhost` - **Passes**

### Linting
✅ `cargo clippy -p tendhost -- -W clippy::pedantic -D warnings` - **Clean**

### Formatting
✅ `cargo fmt --all` - **Applied**

---

## What's NOT Implemented

The following from `01-implementation-plan.md` are **not implemented** in this skeleton:

### Missing Components:

1. **`factory.rs`** - `DefaultHostFactory` for creating SSH executors and package managers
2. **`api/hosts.rs`** - All host management endpoints:
   - `GET /hosts` - List hosts with pagination and filtering
   - `GET /hosts/:name` - Get host details
   - `DELETE /hosts/:name` - Remove host
   - `POST /hosts/:name/update` - Trigger update
   - `POST /hosts/:name/reboot` - Trigger reboot
   - `POST /hosts/:name/retry` - Retry failed host
   - `POST /hosts/:name/acknowledge` - Acknowledge failure
   - `GET /hosts/:name/inventory` - Get inventory

3. **`api/fleet.rs`** - Fleet operation endpoints:
   - `POST /fleet/update` - Fleet-wide update
   - `GET /groups` - List groups
   - `GET /groups/:name` - Get group details
   - `GET /tags` - List tags with counts

4. **`api/ws.rs`** - WebSocket event streaming:
   - `GET /ws/events` - WebSocket upgrade handler
   - Event subscription from orchestrator
   - Bidirectional message handling

5. **OpenAPI Documentation**:
   - Route annotations with `utoipa::path`
   - Schema definitions
   - Scalar UI integration at `/docs`
   - OpenAPI JSON at `/openapi.json`

6. **Configuration Enhancements**:
   - TLS support
   - Authentication tokens
   - Default host settings
   - Group definitions
   - Audit logging

7. **Host Registration**:
   - Loading hosts from config file
   - Creating SSH executors
   - Detecting package managers
   - Spawning `HostActor` instances

8. **Middleware**:
   - Request logging
   - Authentication checking
   - CORS headers
   - Rate limiting

---

## Next Steps for Full Implementation

Based on `01-implementation-plan.md`, the recommended implementation order:

### Phase 1: Host Actor Factory (1.5 hours)
- Create `factory.rs` with `DefaultHostFactory`
- Implement executor creation (Local vs SSH)
- Implement package manager detection
- Handle SSH key resolution

### Phase 2: Host API Endpoints (2.5 hours)
- Implement all host handlers in `api/hosts.rs`
- Add response types and pagination
- Add OpenAPI annotations
- Add filtering logic

### Phase 3: Fleet API Endpoints (1 hour)
- Implement fleet handlers in `api/fleet.rs`
- Add batch update logic
- Add group/tag endpoints

### Phase 4: WebSocket Support (1.5 hours)
- Implement `api/ws.rs` with event streaming
- Add `Subscribe` message to `tendhost-core`
- Handle client connections and disconnections

### Phase 5: OpenAPI Documentation (45 min)
- Add all route annotations
- Configure Scalar UI
- Test documentation endpoint

### Phase 6: Configuration & Testing (2 hours)
- Expand config types
- Create example `tendhost.toml`
- Write integration tests
- Add middleware

**Total Remaining Effort**: ~9.5 hours

---

## Usage Notes

### Configuration File

Create `tendhost.toml` in one of these locations:
- `./tendhost.toml` (current directory)
- `/etc/tendhost/tendhost.toml`
- `~/.config/tendhost/tendhost.toml`
- Set `TENDHOST_CONFIG` environment variable

**Example config**:
```toml
[daemon]
bind = "127.0.0.1:8080"
log_level = "info"

# Hosts are not registered in skeleton mode yet
# [[host]]
# name = "example-host"
# addr = "192.168.1.10"
# tags = ["production"]
```

### Environment Variables

- `TENDHOST_CONFIG` - Path to config file
- `RUST_LOG` - Override log level (e.g., `RUST_LOG=debug`)

---

## Summary

The tendhost daemon skeleton is **complete and functional** with:
- ✅ Compiles and runs
- ✅ Loads configuration
- ✅ Spawns orchestrator actor
- ✅ Serves HTTP with graceful shutdown
- ✅ Health endpoint working
- ✅ Clean clippy pedantic
- ✅ Proper error handling

The skeleton provides a **solid foundation** for implementing the full API per the detailed plan in `01-implementation-plan.md`. All major architectural decisions are in place (axum routing, actor system, state management, configuration loading).

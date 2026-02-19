# Reasoning: tendhost-core Implementation Plan

## Current State Analysis

### What Exists
- **Skeleton structure**: `lib.rs`, `actor/mod.rs`, `state.rs`, `message.rs` files exist with minimal content
- **State enum**: `HostState` enum with all 8 states defined (Idle, Querying, PendingUpdates, Updating, WaitingReboot, Rebooting, Verifying, Failed)
- **FailedState struct**: Contains `previous_state`, `error`, `failed_at`, `retry_count`, `acknowledged`
- **Basic messages**: `QueryInventory`, `StartUpdate`, `RebootIfRequired`, `HealthCheck`, `RegisterHost`, `TriggerFleetUpdate`
- **Actor stubs**: Empty `HostActor` and `OrchestratorActor` structs

### Dependencies Available
From `Cargo.toml` (updated):
- `kameo` 0.19 - Actor framework (latest)
- `kameo_actors` 0.4 - Pre-built actor patterns (pubsub, etc.)
- `kameo_macros` 0.19 - Derive macros for actors
- `tokio` with full features - Async runtime
- `chrono` with serde - Timestamps
- `thiserror` - Error types
- `async-trait` - Async trait support
- `tracing` - Structured logging
- `serde`/`serde_json` - Serialization
- `tendhost-api` - Event types (`WsEvent`)
- `tendhost-exec` - Remote executor trait
- `tendhost-pkg` - Package manager trait

### External Crate Interfaces
From reading dependent crates:

1. **tendhost-exec** (`RemoteExecutor` trait):
   ```rust
   async fn run(&self, cmd: &str) -> Result<String, String>;
   async fn run_with_timeout(&self, cmd: &str, timeout: Duration) -> Result<String, String>;
   ```

2. **tendhost-pkg** (`PackageManager` trait):
   ```rust
   async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, String>;
   async fn upgrade_all(&self) -> Result<UpdateResult, String>;
   async fn upgrade_dry_run(&self) -> Result<UpdateResult, String>;
   async fn reboot_required(&self) -> Result<bool, String>;
   ```

3. **tendhost-api** (`WsEvent` enum):
   - `HostStateChanged { host, from, to }`
   - `UpdateProgress { host, package, progress }`
   - `UpdateCompleted { host, result }`
   - `HostConnected { host }`
   - `HostDisconnected { host, reason }`

## Key Design Decisions

### 1. Actor Framework Pattern (kameo 0.19)
Kameo uses a message-passing model. Each actor:
- Has internal state
- Receives messages via handlers
- Can spawn child actors
- Supports supervision hierarchies and linking

Key kameo 0.19 concepts:
- `Actor` trait with `Args` and `Error` associated types
- `on_start(args, actor_ref) -> Result<Self, Self::Error>` for initialization
- `on_stop(&mut self, weak_ref, reason)` for cleanup
- `on_panic(&mut self, weak_ref, err)` for panic recovery
- `Message<M>` trait with `handle(&mut self, msg, ctx) -> Self::Reply`
- `ActorRef<A>` for actor references with `ask()` and `tell()` methods
- `#[derive(Actor)]` macro for simple actors without custom lifecycle
- `spawn()` and `spawn_with_mailbox()` for actor creation
- `kameo_actors` provides pre-built patterns (pubsub, etc.)

### 2. State Machine Design
The `HostActor` manages a state machine per GOALS.md:
```
Idle -> Querying -> PendingUpdates -> Updating -> WaitingReboot -> Rebooting -> Verifying -> Idle
                                   \-> Idle (no reboot needed)
Any state -> Failed (on error)
Failed -> Idle (on retry)
```

**State transitions** should be:
- Explicit (no implicit transitions)
- Logged via tracing
- Broadcast via event channel for WebSocket

### 3. Error Handling Strategy
- Use `thiserror` for `CoreError` enum
- All errors captured in `FailedState`
- Recovery path via `Retry` message
- Acknowledgment path via `Acknowledge` message

### 4. Event Broadcasting
`HostActor` needs to emit events for WebSocket:
- Use `tokio::sync::broadcast` channel
- Orchestrator holds sender, shares with actors
- State changes emit `WsEvent::HostStateChanged`

### 5. Dependency Injection
`HostActor` needs:
- `Box<dyn RemoteExecutor>` - for SSH/local execution
- `Box<dyn PackageManager>` - for apt/dnf/docker
- Event sender - for broadcasting

This allows testing with mock implementations.

## Implementation Phases

### Phase 1: Foundation (error.rs, config types)
Set up error types and configuration structures that actors will use.

### Phase 2: State Machine (state.rs enhancement)
- Add `impl HostState` with transition validation
- Add serialization for API compatibility
- Add display formatting

### Phase 3: Messages (message.rs enhancement)
- Define kameo `Message` implementations
- Add reply types for each message
- Add orchestrator-specific messages

### Phase 4: HostActor (actor/host.rs)
- Implement `Actor` trait
- State machine logic
- Message handlers
- Event emission

### Phase 5: OrchestratorActor (actor/orchestrator.rs)
- Host registry management
- Fleet-wide commands
- Batch scheduling
- Event channel ownership

### Phase 6: Integration & Testing
- Unit tests for state transitions
- Mock-based actor tests
- Integration with tendhost-api events

## Complexity Assessment

| Component | Complexity | Notes |
|-----------|------------|-------|
| Error types | Low | Standard thiserror pattern |
| State transitions | Medium | 8 states, multiple paths, validation |
| Message definitions | Medium | kameo Message trait, reply types |
| HostActor | High | State machine + external deps + async |
| OrchestratorActor | High | Registry + batching + supervision |
| Testing | Medium | Mocking required |

## Dependencies Between Tasks

```
error.rs ─────────────────────────────────┐
                                          │
state.rs (enhanced) ──────────────────────┼──► HostActor
                                          │
message.rs (with Message impls) ──────────┤
                                          │
                                          └──► OrchestratorActor
                                                    │
                                                    ▼
                                               Integration Tests
```

## Risk Areas

1. **kameo 0.19 API**: Verified via Context7 docs - uses `Args`/`Error` associated types, `on_start` returns `Result<Self, Error>`
2. **Trait object constraints**: `PackageManager` and `RemoteExecutor` need `Send + Sync` (already defined)
3. **Async state transitions**: Need careful handling of concurrent messages during state changes
4. **Event ordering**: Broadcast channel may lose events if subscriber is slow
5. **Actor error handling**: kameo 0.19 uses `SendError::HandlerError` for message handler errors

## Next Steps

See `01-implementation-plan.md` for detailed task breakdown.

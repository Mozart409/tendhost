# Reasoning: tendhost-exec Implementation Plan

## Current State Analysis

### What Exists
- **Skeleton structure**: `lib.rs`, `traits.rs`, `ssh.rs`, `local.rs` files exist with minimal content
- **Trait definition**: `RemoteExecutor` trait with basic methods (`run`, `run_with_timeout`)
- **Module structure**: Traits and implementations modules set up

### Dependencies Available
From `Cargo.toml`:
- `tokio` - Async runtime (for I/O operations)
- `async-trait` - Async trait support
- `thiserror` - Error types
- `serde/serde_json` - Serialization (for structured command results)
- `tracing` - Structured logging

### Design Decisions

#### 1. SSH Implementation Strategy
Need to choose between SSH libraries:
- **`openssh`** (high-level, `ssh` binary wrapper) - Simpler, but requires binary
- **`russh`** (pure Rust) - More complex, but fully Rust-native
- **`libssh`** (bindings) - Good middle ground

**Decision**: Start with `openssh` for MVP (simpler, battle-tested), add `russh` later for pure Rust deployments.

#### 2. Error Handling
Use `thiserror` with specific error variants:
- Connection failures
- Authentication errors
- Command execution errors
- Timeout errors

#### 3. Timeout Handling
- `run_with_timeout` should use `tokio::time::timeout`
- Cleanup spawned SSH connections on timeout
- Return structured error indicating timeout vs failure

#### 4. SSH Key Management
Support multiple key sources (from GOALS.md):
1. Explicit path in config
2. SSH agent (default)
3. Environment variable `TENDHOST_SSH_KEY` (base64-encoded)

#### 5. Local Execution
Even for local execution, wrap to:
- Provide consistent interface
- Add logging
- Handle timeouts the same way
- Support both sync and async subprocess

## Implementation Phases

### Phase 1: Error Types
Define comprehensive error enum covering all failure modes.

### Phase 2: Trait Enhancement
Enhance `RemoteExecutor` trait with:
- Connection lifecycle methods (for SSH)
- Better return types (structured results)

### Phase 3: Local Executor
Implement local command execution using `tokio::process::Command`.

### Phase 4: SSH Executor (openssh)
Implement SSH execution using `openssh` crate.

### Phase 5: SSH Key Management
Implement key resolution logic.

### Phase 6: Testing
- Unit tests with mock commands
- Integration tests (optional, requires SSH server)

## Risk Areas

1. **SSH library choice** - May need to switch if openssh doesn't meet needs
2. **Connection pooling** - Not in scope for MVP, but needed for scale
3. **Key permissions** - Must validate key file permissions (600)
4. **Timeout handling** - Must properly kill processes on timeout

## Next Steps

See `01-implementation-plan.md` for detailed task breakdown.

# Reasoning: tendhost-inventory Implementation Plan

## Current State Analysis

### What Exists
- **Skeleton structure**: `lib.rs`, `osquery.rs`, `types.rs` files exist with minimal content
- **No concrete implementations** yet

### Dependencies Available
From `Cargo.toml`:
- `tokio` - Async runtime
- `async-trait` - Async trait support
- `thiserror` - Error types
- `serde/serde_json` - Serialization
- `tracing` - Structured logging

### Design Decisions

#### 1. osquery Integration Strategy
osquery provides unified SQL interface across distros:
- **Local execution**: osqueryi (interactive shell)
- **Daemon mode**: osqueryd with extensions
- **Remote execution**: SSH to host, run osqueryi

**Decision**: Use `osqueryi` via `RemoteExecutor` for flexibility (works over SSH).

#### 2. Query Categories
From GOALS.md, need to query:
- **System info**: OS version, hostname, uptime
- **Packages**: deb_packages, rpm_packages (distro-specific)
- **Docker**: containers, images, volumes
- **Hardware**: cpu, memory, disk
- **Network**: interfaces, listening ports

#### 3. Error Handling Strategy
- Connection failures (SSH)
- osquery not installed
- Query syntax errors
- JSON parsing errors

#### 4. Caching Strategy
Inventory queries can be expensive:
- Cache results for configurable duration
- Provide force-refresh option
- Cache invalidation on demand

#### 5. SQL Query Builder
Build type-safe queries instead of raw SQL strings:
- Prevent SQL injection
- Query validation
- Reusable query patterns

## Implementation Phases

### Phase 1: Error Types
Define comprehensive error enum for inventory operations.

### Phase 2: Type Definitions
Define structs for all inventory data types.

### Phase 3: SQL Query Builder
Type-safe query construction.

### Phase 4: OsqueryClient
Main client for executing queries.

### Phase 5: High-level Inventory API
Convenience methods for common queries.

### Phase 6: Testing
- Unit tests with mock executor
- Integration tests (optional)

## Risk Areas

1. **osquery availability** - May not be installed on all hosts
2. **Query performance** - Large package lists can be slow
3. **JSON parsing** - osquery output format changes
4. **Distro differences** - Table availability varies

## Next Steps

See `01-implementation-plan.md` for detailed task breakdown.

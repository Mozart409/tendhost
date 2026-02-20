# Implementation Plans

This folder contains implementation plans and reasoning documents organized by crate.

## Structure

```
todo/
├── README.md                        # This file (status overview)
├── tendhost/                        # ⏳ PLANNED (daemon binary)
│   ├── 00-reasoning.md
│   └── 01-implementation-plan.md
├── tendhost-core/                   # ✅ COMPLETE
│   ├── 00-reasoning.md
│   ├── 01-implementation-plan.md
│   └── 02-quick-reference.md
├── tendhost-exec/                   # ✅ COMPLETE
│   ├── 00-reasoning.md
│   └── 01-implementation-plan.md
├── tendhost-pkg/                    # ✅ COMPLETE
│   ├── 00-reasoning.md
│   └── 01-implementation-plan.md
└── tendhost-inventory/              # ✅ COMPLETE
    ├── 00-reasoning.md
    ├── 01-implementation-plan.md
    └── 02-completion-summary.md
```

## Implementation Status (Updated: 2026-02-20 21:00)

### ✅ Completed Crates

#### 1. **tendhost-core** - Actor framework
**Status**: ✅ **COMPLETE** (2026-02-19)
- ✅ `HostActor` with state machine (6 states)
- ✅ `OrchestratorActor` for fleet coordination
- ✅ Message types with kameo 0.19 integration
- ✅ Event broadcasting for WebSocket
- ✅ Error handling (`CoreError`)
- ✅ All tests passing (6/6)
- ✅ Clean clippy run

**Files**: `actor/`, `message.rs`, `state.rs`, `event.rs`, `error.rs`

#### 2. **tendhost-exec** - Remote execution
**Status**: ✅ **COMPLETE** (2026-02-19)
- ✅ `RemoteExecutor` trait with `RemoteExecutorExt`
- ✅ `LocalExecutor` - tokio process execution
- ✅ `SshExecutor` - SSH execution via openssh crate
- ✅ `CommandResult` with status, stdout, stderr, duration
- ✅ SSH key management (`KeySource`, `ResolvedKey`)
- ✅ Error handling (`ExecError`) with retryable detection
- ✅ Connection info tracking
- ✅ Tests passing

**Files**: `error.rs`, `traits.rs`, `local.rs`, `ssh.rs`, `keys.rs`, `result.rs`

#### 3. **tendhost-pkg** - Package manager abstraction
**Status**: ✅ **COMPLETE** (2026-02-20)
- ✅ `PackageManager` trait with `PackageManagerExt`
- ✅ `AptManager` - Debian/Ubuntu (apt)
- ✅ `DnfManager` - Fedora/RHEL (dnf/yum with auto-detection)
- ✅ `DockerComposeManager` - Docker Compose stacks (v1/v2)
- ✅ Error handling (`PackageError`)
- ✅ Type system (`UpgradablePackage`, `UpdateResult`, `PackageManagerType`)
- ✅ Command output parsing with tests
- ✅ All tests passing (4/4)
- ✅ Clean clippy run

**Files**: `error.rs`, `types.rs`, `traits.rs`, `apt.rs`, `dnf.rs`, `docker.rs`

#### 4. **tendhost-inventory** - osquery integration
**Status**: ✅ **COMPLETE** (2026-02-20)
- ✅ `OsqueryClient` - SQL query execution via osqueryi
- ✅ `InventoryCollector` - High-level inventory API
- ✅ Query builder - Type-safe SQL construction with injection prevention
- ✅ Type definitions (`SystemInfo`, `HardwareInfo`, `Package`, `Container`, etc.)
- ✅ Error handling (`InventoryError`)
- ✅ Query caching with TTL support
- ✅ All tests passing (7/7, 1 ignored)
- ✅ Clean clippy pedantic run

**Files**: `error.rs`, `types.rs`, `query.rs`, `osquery.rs`, `collector.rs`

#### 5. **tendhost-api** - Shared API types
**Status**: ✅ **MOSTLY COMPLETE**
- ✅ Request/response types
- ✅ WebSocket event types
- ✅ Serde derives for JSON
- ⏳ May need minor additions as features expand

**Files**: `lib.rs` (types)

---

### ⏳ Pending Crates

#### 6. **tendhost-client** - HTTP + WebSocket client
**Status**: ⏳ **NOT STARTED**
- ⏳ HTTP client for REST API
- ⏳ WebSocket client for events
- ⏳ Auto-reconnection logic
- 📋 No plan yet

**Estimated effort**: ~4 hours

#### 7. **tendhost** - Daemon binary
**Status**: ⏳ **SKELETON** (2026-02-20)
- ✅ Configuration loading from TOML
- ✅ Actor system initialization (OrchestratorActor)
- ✅ Axum HTTP server with graceful shutdown
- ✅ Health endpoint (`/health`)
- ✅ Tracing and error handling
- ⏳ Host API endpoints (pending)
- ⏳ Fleet API endpoints (pending)
- ⏳ WebSocket event streaming (pending)
- ⏳ OpenAPI documentation (pending)
- ⏳ Host actor factory (pending)
- 📋 Plan: `todo/tendhost/01-implementation-plan.md`
- 📋 Skeleton status: `todo/tendhost/02-skeleton-status.md`

**Current files**: Runnable skeleton (main.rs, config.rs, state.rs, router.rs, api/system.rs)
**Remaining effort**: ~9.5 hours for full API

#### 8. **tendhost-cli** - CLI tool
**Status**: ⏳ **SKELETON ONLY**
- ⏳ Clap argument parsing
- ⏳ Command implementations
- ⏳ Output formatting
- 📋 No plan yet

**Estimated effort**: ~4 hours

#### 9. **tendhost-tui** - Terminal UI
**Status**: ⏳ **SKELETON ONLY**
- ⏳ Ratatui dashboard
- ⏳ Event handling
- ⏳ Real-time updates
- 📋 No plan yet

**Estimated effort**: ~12 hours

---

## Recommended Implementation Order

Based on dependencies and current progress:

1. ✅ **tendhost-core** (DONE)
2. ✅ **tendhost-exec** (DONE)
3. ✅ **tendhost-pkg** (DONE)
4. ✅ **tendhost-inventory** (DONE)
5. ⏳ **tendhost** (SKELETON - needs full API implementation)
6. ⏳ **tendhost-client** (needed for CLI/TUI)
7. ⏳ **tendhost-cli** (basic commands)
8. ⏳ **tendhost-tui** (advanced UI)

---

## Summary

- **Completed**: 4 core library crates (core, exec, pkg, inventory)
- **Skeleton**: 1 binary crate (tendhost daemon - MVP runnable)
- **Pending**: 3 crates (client, cli, tui)
- **Total Progress**: ~55% of core functionality complete
- **Next Focus**: Complete `tendhost` daemon API implementation or start client/CLI crates

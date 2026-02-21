# Implementation Plans

This folder contains implementation plans and reasoning documents organized by crate.

## Structure

```
todo/
├── README.md                        # This file (status overview)
├── tendhost/                        # ⏳ SKELETON (daemon binary)
│   ├── 00-reasoning.md
│   ├── 01-implementation-plan.md
│   └── 02-skeleton-status.md
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
├── tendhost-inventory/              # ✅ COMPLETE
│   ├── 00-reasoning.md
│   ├── 01-implementation-plan.md
│   └── 02-completion-summary.md
├── tendhost-client/                 # ✅ COMPLETE
│   ├── 00-reasoning.md
│   ├── 01-implementation-plan.md
│   └── 02-completion-summary.md
└── tendhost-tui/                    # ✅ COMPLETE (TUI binary)
    ├── 00-reasoning.md
    └── 01-implementation-plan.md
```

## Implementation Status (Updated: 2026-02-21 - TUI Complete! 🎉)

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

#### 5. **tendhost-client** - HTTP + WebSocket client
**Status**: ✅ **COMPLETE** (2026-02-20)
- ✅ HTTP client for REST API (all endpoints)
- ✅ WebSocket client for events
- ✅ Auto-reconnection logic with exponential backoff
- ✅ Builder pattern for query construction
- ✅ Error handling (`ClientError`)
- ✅ All tests passing (8/8 unit + 16/16 doc tests)
- ✅ Clean clippy pedantic run
- 📋 Plan: `todo/tendhost-client/01-implementation-plan.md`
- 📋 Summary: `todo/tendhost-client/02-completion-summary.md`

**Files**: `error.rs`, `http.rs`, `ws.rs`, `lib.rs` (830 lines total)

#### 6. **tendhost** - Daemon binary
**Status**: ⏳ **IN PROGRESS** (40% Complete - 2026-02-21)
- ✅ Configuration loading from TOML
- ✅ Actor system initialization (OrchestratorActor)
- ✅ Axum HTTP server with graceful shutdown
- ✅ Health endpoint (`/health`)
- ✅ Tracing and error handling
- ✅ **Host actor factory (DefaultHostFactory)** - NEW!
- ✅ **Host API endpoints (9 endpoints)** - NEW!
  - GET/POST /hosts - List and register
  - GET/DELETE /hosts/:hostname - Details and unregister
  - POST /hosts/:hostname/{update,reboot,retry,acknowledge}
  - GET /hosts/:hostname/inventory
- ⏳ Fleet API endpoints (4 endpoints pending)
- ⏳ WebSocket event streaming (pending)
- ⏳ OpenAPI documentation (pending)
- 📋 Plan: `todo/tendhost/01-implementation-plan.md`
- 📋 Skeleton: `todo/tendhost/02-skeleton-status.md`
- 📋 **Progress: `todo/tendhost/03-api-implementation-progress.md`** - NEW!

**Current files**: factory.rs, api/hosts.rs (340 lines), router (10/17 endpoints)
**Completed**: Phases 1-2 of 6 (4 hours)
**Remaining effort**: ~5.5 hours for fleet, WebSocket, docs, testing

#### 7. **tendhost-cli** - CLI tool
**Status**: ⏳ **SKELETON ONLY**
- ⏳ Clap argument parsing
- ⏳ Command implementations
- ⏳ Output formatting
- 📋 No plan yet

**Estimated effort**: ~4 hours

#### 8. **tendhost-tui** - Terminal UI
**Status**: ✅ **COMPLETE** (2026-02-21)
- ✅ Ratatui dashboard with host table
- ✅ Real-time WebSocket event updates
- ✅ Host details panel with inventory
- ✅ Event log panel
- ✅ Keyboard navigation (vim-style)
- ✅ Actions (update, reboot, retry)
- ✅ Search and filtering
- ✅ Help popup with keybindings
- ✅ Status bar with connection state
- ✅ Color-coded host states
- ✅ Clean build with clippy pedantic
- 📋 Reasoning: `todo/tendhost-tui/00-reasoning.md`
- 📋 Plan: `todo/tendhost-tui/01-implementation-plan.md`

**Architecture**: App state + Event loop + UI rendering + WebSocket integration
**Files**: 13 modules (main, action, event, app, config, ui/*)

---

## Recommended Implementation Order

Based on dependencies and current progress:

1. ✅ **tendhost-core** (DONE)
2. ✅ **tendhost-exec** (DONE)
3. ✅ **tendhost-pkg** (DONE)
4. ✅ **tendhost-inventory** (DONE)
5. ✅ **tendhost-client** (DONE)
6. ✅ **tendhost-tui** (DONE)
7. ⏳ **tendhost** (40% DONE - host API + factory complete, fleet/WS pending)
8. ⏳ **tendhost-cli** (not started)

---

## Summary

- **Completed**: 6 crates (core, exec, pkg, inventory, client, tui)
- **In Progress**: 1 binary crate (tendhost daemon - 40% API complete)
- **Pending**: 1 user-facing crate (cli)
- **Total Progress**: ~87% of core functionality complete
- **Next Focus**: Complete daemon API (fleet, WebSocket, docs) OR build CLI

## Recent Completion: tendhost-tui ✨

**Status**: ✅ Fully functional Terminal UI (2026-02-21)

### Implemented Features
- **Host Table**: List view with state, OS, package counts
- **Details Panel**: System info, uptime, upgradable packages
- **Event Log**: Real-time event stream with timestamps
- **WebSocket Integration**: Live updates from daemon
- **Keyboard Navigation**: j/k/g/G + arrow keys + Tab for focus
- **Actions**: Trigger update (u), reboot (r), retry (R), acknowledge (a)
- **Search**: Filter hosts with / key
- **Help Popup**: Complete keybinding reference with ?
- **Color Coding**: Visual states (green=idle, blue=updating, red=failed, etc.)
- **Status Bar**: Connection state + keybinding hints

### Technical Details
- **13 modules**: main, action, event, app (440 lines), config, ui/* (7 widgets)
- **Build Status**: ✅ cargo build, ✅ cargo test, ✅ clippy pedantic
- **Dependencies**: ratatui, crossterm, tokio, tendhost-client
- **Architecture**: Async event loop with tokio::select! for terminal events + WebSocket

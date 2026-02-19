# Implementation Plans

This folder contains implementation plans and reasoning documents organized by crate.

## Structure

```
todo/
├── README.md                 # This file
└── tendhost-core/           # Core actor framework
    ├── 00-reasoning.md      # Design decisions & analysis
    ├── 01-implementation-plan.md  # Detailed task breakdown
    └── 02-quick-reference.md      # kameo 0.19 API reference
```

## Crate Implementation Order

Based on dependency graph:

1. **tendhost-core** - Actors, state machines, messages (in progress)
2. **tendhost-api** - Shared API types (mostly done)
3. **tendhost-exec** - Remote execution (SSH, local)
4. **tendhost-pkg** - Package manager abstraction
5. **tendhost-inventory** - osquery integration
6. **tendhost-client** - HTTP + WebSocket client
7. **tendhost** - Daemon binary (axum, wiring)
8. **tendhost-cli** - CLI tool
9. **tendhost-tui** - Terminal UI

## Current Focus

**tendhost-pkg** - Package manager abstraction (foundation complete):
- ✅ Error types (`PackageError`)
- ✅ Type definitions (`UpgradablePackage`, `UpdateResult`, etc.)
- ✅ Enhanced `PackageManager` trait with `PackageManagerExt`
- ⏳ `AptManager`: Debian/Ubuntu implementation pending
- ⏳ `DnfManager`: CentOS/Fedora implementation pending
- ⏳ `DockerComposeManager`: Docker Compose implementation pending

## Recently Completed

**tendhost-exec** - Remote execution infrastructure:
- `ExecError` with retryable detection
- `CommandResult` with status, stdout, stderr, duration
- `RemoteExecutor` trait with SSH and local support
- SSH key management with `KeyManager`

**tendhost-core** - Actor framework:
- `HostActor` with state machine
- `OrchestratorActor` for fleet coordination
- Event broadcasting for WebSocket
- Integration tests updated for new types

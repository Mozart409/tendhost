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

**tendhost-core** - Implementing the actor framework with kameo 0.19:
- `HostActor`: Per-host state machine
- `OrchestratorActor`: Fleet coordination
- Message types and handlers
- Event broadcasting for WebSocket

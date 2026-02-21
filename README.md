# tendhost

Actor-based homelab orchestration for managing updates across heterogeneous infrastructure.

## What It Does

`tendhost` is a Rust-native tool that uses **osquery for inventory** and an **actor-per-host model** to orchestrate package updates across Debian, CentOS, Fedora VMs/CTs, and Docker Compose stacks.

## Quick Start

```bash
# Enter the Nix development environment
nix develop

# Build the project
cargo build --release

# Run the daemon
cargo run --bin tendhost

# List managed hosts
curl http://localhost:8080/hosts

# Trigger a fleet update
curl -X POST http://localhost:8080/fleet/update \
  -H 'Content-Type: application/json' \
  -d '{"batch_size": 2}'
```

## Architecture

```
┌─────────────────┐
│  Orchestrator   │
│    Actor        │
└────────┬────────┘
         │ spawns & supervises
    ┌────┴────┬────────┐
    ▼         ▼        ▼
┌────────┐ ┌────────┐ ┌────────┐
│ Host   │ │ Host   │ │ Host   │
│ Actor  │ │ Actor  │ │ Actor  │
└────┬───┘ └────┬───┘ └────┬───┘
     │          │          │
     ▼          ▼          ▼
  osquery + SSH/apt/dnf/docker
```

## Workspace Crates

| Crate                | Type | Purpose                          |
| -------------------- | ---- | -------------------------------- |
| `tendhost`           | bin  | Daemon (axum, actors)            |
| `tendhost-cli`       | bin  | CLI tool                         |
| `tendhost-tui`       | bin  | Terminal UI                      |
| `tendhost-core`      | lib  | Actors, messages, state machines |
| `tendhost-api`       | lib  | Shared types (REST + WebSocket)  |
| `tendhost-inventory` | lib  | osquery integration              |
| `tendhost-pkg`       | lib  | Package manager abstraction      |
| `tendhost-exec`      | lib  | Remote execution (SSH, local)    |
| `tendhost-client`    | lib  | HTTP and WebSocket client        |

## Documentation

- [GOALS.md](./GOALS.md) - Architecture, API design, and project vision
- [AGENTS.md](./AGENTS.md) - Development guidelines and coding standards

## License

AGPL

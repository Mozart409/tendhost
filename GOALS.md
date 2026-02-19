# tendhost

> Actor-based homelab orchestration system for managing updates across heterogeneous infrastructure.

## Vision

A Rust-native tool that uses **osquery for inventory** and an **actor-per-host model** for orchestrating updates across Debian, CentOS, Fedora VMs/CTs and Docker Compose stacks. Built with [kameo](https://github.com/tqwewe/kameo) for actor lifecycle management.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      tendhost (binary)                          │
│                 CLI, config, wiring, color_eyre                 │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
┌─────────────────────────────────┴───────────────────────────────┐
│                      tendhost-core                              │
│              OrchestratorActor, HostActor, messages             │
│                    state machines, fleet logic                  │
└───────┬─────────────────────┬─────────────────────┬─────────────┘
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐   ┌─────────────────┐   ┌───────────────────┐
│tendhost-      │   │tendhost-pkg     │   │tendhost-exec      │
│inventory      │   │                 │   │                   │
│               │   │ PackageManager  │   │ RemoteExecutor    │
│ OsqueryClient │   │ ├─ AptManager   │   │ ├─ SshExecutor    │
│ HostInventory │   │ ├─ DnfManager   │   │ └─ LocalExecutor  │
│               │   │ └─ DockerCompose│   │                   │
└───────────────┘   └─────────────────┘   └───────────────────┘
```

## Actor Model

```
┌──────────────────────────────────────────────────────────────┐
│                    OrchestratorActor                         │
│  • Registry of HostActors                                    │
│  • Fleet-wide commands (TriggerFleetUpdate)                  │
│  • Batch scheduling, wave-based rollouts                     │
└────────────────────────────┬─────────────────────────────────┘
                             │ spawns & supervises
          ┌──────────────────┼──────────────────┐
          ▼                  ▼                  ▼
   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
   │ HostActor   │    │ HostActor   │    │ HostActor   │
   │ (debian-1)  │    │ (centos-2)  │    │ (fedora-3)  │
   │             │    │             │    │             │
   │ State:      │    │ State:      │    │ State:      │
   │ ┌─────────┐ │    │ ┌─────────┐ │    │ ┌─────────┐ │
   │ │Idle     │ │    │ │Updating │ │    │ │Rebooting│ │
   │ └─────────┘ │    │ └─────────┘ │    │ └─────────┘ │
   └──────┬──────┘    └──────┬──────┘    └──────┬──────┘
          │                  │                  │
          ▼                  ▼                  ▼
      osquery +          osquery +          osquery +
      SSH/apt            SSH/dnf            SSH/dnf
```

## Host State Machine

```
                    ┌──────────┐
                    │   Idle   │◄─────────────────────────┐
                    └────┬─────┘                          │
                         │ QueryInventory                 │
                         ▼                                │
                    ┌──────────┐                          │
                    │ Querying │                          │
                    └────┬─────┘                          │
                         │                                │
           ┌─────────────┴─────────────┐                  │
           ▼                           ▼                  │
    ┌──────────────┐            ┌──────────┐              │
    │PendingUpdates│            │   Idle   │──────────────┘
    │    (n=42)    │            └──────────┘
    └──────┬───────┘
           │ StartUpdate
           ▼
    ┌──────────────┐
    │   Updating   │
    └──────┬───────┘
           │
     ┌─────┴─────┐
     ▼           ▼
┌─────────┐ ┌──────────────┐
│  Idle   │ │WaitingReboot │
└─────────┘ └──────┬───────┘
                   │ RebootIfRequired
                   ▼
            ┌──────────┐
            │Rebooting │
            └────┬─────┘
                 │
                 ▼
            ┌──────────┐
            │Verifying │───────► Idle / Failed
            └──────────┘
```

## Workspace Structure

```
tendhost/
├── Cargo.toml                  # workspace root
├── GOALS.md
├── crates/
│   ├── tendhost/               # [[bin]] daemon
│   │   └── src/
│   │       ├── main.rs
│   │       ├── api/            # axum routes
│   │       │   ├── mod.rs
│   │       │   ├── hosts.rs
│   │       │   ├── fleet.rs
│   │       │   └── ws.rs       # WebSocket handler
│   │       └── config.rs
│   ├── tendhost-cli/           # [[bin]] CLI
│   │   └── src/
│   │       └── main.rs
│   ├── tendhost-tui/           # [[bin]] TUI (ratatui + WebSocket)
│   │   └── src/
│   │       └── main.rs
│   ├── tendhost-api/           # [lib] shared types (OpenAPI schema)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── requests.rs
│   │       ├── responses.rs
│   │       └── events.rs       # WsEvent types
│   ├── tendhost-client/        # [lib] HTTP + WebSocket client
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── http.rs
│   │       └── ws.rs
│   ├── tendhost-core/          # [lib] actors, messages, state machines
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── actor/
│   │       │   ├── mod.rs
│   │       │   ├── orchestrator.rs
│   │       │   └── host.rs
│   │       ├── message.rs
│   │       └── state.rs
│   ├── tendhost-inventory/     # [lib] osquery integration
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── osquery.rs
│   │       └── types.rs
│   ├── tendhost-pkg/           # [lib] package manager abstraction
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs
│   │       ├── apt.rs
│   │       ├── dnf.rs
│   │       └── docker.rs
│   └── tendhost-exec/          # [lib] remote execution
│       └── src/
│           ├── lib.rs
│           ├── traits.rs
│           ├── ssh.rs
│           └── local.rs
```

## Key Dependencies

| Crate                  | Purpose                           |
| ---------------------- | --------------------------------- |
| `kameo`                | Actor framework                   |
| `tokio`                | Async runtime                     |
| `axum`                 | HTTP server + WebSocket           |
| `utoipa`               | OpenAPI spec generation           |
| `utoipa-scalar`        | Scalar API docs UI                |
| `eyre` / `color-eyre`  | Error handling in bins)           |
| `thiserror`            | Error handling in libs            |
| `serde` / `serde_json` | Serialization                     |
| `reqwest`              | HTTP client (for tendhost-client) |
| `tokio-tungstenite`    | WebSocket client (for CLI/TUI)    |
| `russh` or `openssh`   | SSH remote execution              |
| `clap`                 | CLI argument parsing              |
| `ratatui`              | TUI framework                     |
| `tracing`              | Structured logging                |
| `chrono`               | Timestamps                        |

## Core Traits

### RemoteExecutor

```rust
#[async_trait]
pub trait RemoteExecutor: Send + Sync {
    async fn run(&self, cmd: &str) -> Result<String>;
    async fn run_with_timeout(&self, cmd: &str, timeout: Duration) -> Result<String>;
}
```

### PackageManager

```rust
#[async_trait]
pub trait PackageManager: Send + Sync {
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>>;
    async fn upgrade_all(&self) -> Result<UpdateResult>;
    async fn upgrade_dry_run(&self) -> Result<UpdateResult>;
    async fn reboot_required(&self) -> Result<bool>;
}
```

## Message Types

```rust
// Inventory
struct QueryInventory;

// Updates
struct StartUpdate { dry_run: bool }
struct RebootIfRequired;
struct HealthCheck;

// Orchestrator
struct RegisterHost { hostname: String, config: HostConfig }
struct TriggerFleetUpdate { batch_size: usize, delay_between_batches: Duration }
```

## osquery Integration

Used for **read-only inventory** across all distros:

```sql
-- System info
SELECT name, version FROM os_version;

-- Packages (distro-specific)
SELECT name, version FROM deb_packages;  -- Debian/Ubuntu
SELECT name, version FROM rpm_packages;  -- CentOS/Fedora

-- Docker
SELECT id, name, image, state FROM docker_containers;
SELECT id, tags FROM docker_images;
```

osquery provides unified querying; actual updates go through SSH + native package managers.

## NixOS Note

osquery is available in nixpkgs:

```nix
services.osquery = {
  enable = true;
  flags = {
    config_path = "/etc/osquery/osquery.conf";
  };
};
```

## Configuration

`tendhost.toml`:

```toml
[defaults]
user = "root"
ssh_key = "~/.ssh/id_ed25519"  # optional, falls back to ssh-agent

[[host]]
name = "proxmox-1"
addr = "192.168.1.10"

[[host]]
name = "debian-vm"
addr = "192.168.1.20"
user = "admin"  # override default

[[host]]
name = "fedora-ct"
addr = "192.168.1.30"
ssh_key = "~/.ssh/fedora_key"  # override default

[[host]]
name = "centos-docker"
addr = "192.168.1.40"
compose_paths = ["/opt/stacks/monitoring", "/opt/stacks/media"]
```

### Host Fields

| Field           | Required | Description                                                  |
| --------------- | -------- | ------------------------------------------------------------ |
| `name`          | yes      | Unique identifier                                            |
| `addr`          | yes      | IP or hostname                                               |
| `user`          | no       | SSH user (default from `[defaults]`)                         |
| `ssh_key`       | no       | Path to private key (default from `[defaults]` or ssh-agent) |
| `compose_paths` | no       | Directories containing docker-compose.yml to manage          |

## Architecture: Daemon / CLI / TUI

```
┌─────────────────────────────────────────────────────────────────┐
│                       tendhost daemon                           │
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌──────────────────┐    │
│  │ Orchestrator│◄──►│  HostActors │◄──►│ WebSocket Hub    │    │
│  │   Actor     │    │             │    │ (broadcast state)│    │
│  └─────────────┘    └─────────────┘    └────────┬─────────┘    │
│                                                  │              │
│  ┌───────────────────────────────────────────────┴───────────┐ │
│  │                      axum router                          │ │
│  │  REST: /hosts, /fleet/update       WS: /ws/events         │ │
│  │  Docs: /docs (Scalar UI)                                  │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
         │                                    │
         ▼                                    ▼
    ┌─────────┐                         ┌─────────┐
    │ CLI     │  HTTP requests          │ TUI     │  WebSocket subscription
    │ curl    │                         │ Web UI  │
    └─────────┘                         └─────────┘
```

## API

OpenAPI spec via `utoipa`, documentation via Scalar.

### REST Endpoints

```
GET  /hosts                     # list all hosts with status
GET  /hosts/:name               # single host details + inventory
GET  /hosts/:name/inventory     # full osquery inventory
POST /hosts/:name/update        # trigger update { dry_run: bool }
POST /hosts/:name/reboot        # trigger reboot if required
POST /fleet/update              # batch update { batch_size, delay_ms }
GET  /health                    # orchestrator health
GET  /docs                      # Scalar API documentation
GET  /openapi.json              # OpenAPI spec
```

### WebSocket: `/ws/events`

Live stream of actor state changes. Clients subscribe once, receive all events.

```rust
#[derive(Serialize, ToSchema)]
#[serde(tag = "type")]
pub enum WsEvent {
    HostStateChanged { host: String, from: HostState, to: HostState },
    UpdateProgress { host: String, package: String, progress: u8 },
    UpdateCompleted { host: String, result: UpdateResult },
    HostConnected { host: String },
    HostDisconnected { host: String, reason: String },
}
```

### Example Usage

```bash
# REST
curl http://localhost:8080/hosts
curl http://localhost:8080/hosts/debian-vm
curl -X POST http://localhost:8080/hosts/debian-vm/update -H 'Content-Type: application/json' -d '{"dry_run": true}'
curl -X POST http://localhost:8080/fleet/update -H 'Content-Type: application/json' -d '{"batch_size": 2}'

# WebSocket (websocat)
websocat ws://localhost:8080/ws/events
```

## Future Considerations

- [ ] Web UI for status dashboard
- [ ] Fleet/Kolide integration for centralized osquery
- [ ] Prometheus metrics export (`/metrics` endpoint)
- [ ] Rollback support
- [ ] Maintenance windows / scheduling
- [ ] Notifications (Slack, email, ntfy, etc.)

## Non-Goals (for now)

- Replacing Ansible/Salt for general configuration management
- Full GitOps workflow (but compose files could be git-managed separately)
- Windows support

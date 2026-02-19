# tendhost

> Actor-based homelab orchestration system for managing updates across heterogeneous infrastructure.

## Vision

A Rust-native tool that uses **osquery for inventory** and an **actor-per-host model** for orchestrating updates across Debian, CentOS, Fedora VMs/CTs and Docker Compose stacks. Built with [kameo](https://github.com/tqwewe/kameo) for actor lifecycle management.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        tendhost (binary)                        │
│                  CLI, config, wiring, color_eyre                │
└────────────────────────────────┬────────────────────────────────┘
                                 │
┌────────────────────────────────┴────────────────────────────────┐
│                         tendhost-core                           │
│               OrchestratorActor, HostActor, messages            │
│                     state machines, fleet logic                 │
└───────────┬─────────────────────┬─────────────────┬─────────────┘
            │                     │                 │
            ▼                     ▼                 ▼
┌───────────────────┐   ┌─────────────────┐   ┌───────────────────┐
│ tendhost-inventory│   │  tendhost-pkg   │   │  tendhost-exec    │
│                   │   │                 │   │                   │
│   OsqueryClient   │   │ PackageManager  │   │  RemoteExecutor   │
│   HostInventory   │   │ ├─ AptManager   │   │  ├─ SshExecutor   │
│                   │   │ ├─ DnfManager   │   │  └─ LocalExecutor │
│                   │   │ └─ DockerCompose│   │                   │
└───────────────────┘   └─────────────────┘   └───────────────────┘
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
        ┌──────────►│   Idle   │◄─────────────────────────┐
        │           └────┬─────┘                          │
        │                │ QueryInventory                 │
        │                ▼                                │
        │           ┌──────────┐                          │
        │           │ Querying │──────────┐               │
        │           └────┬─────┘          │               │
        │                │                │ error         │
        │  ┌─────────────┴─────────────┐  │               │
        │  ▼                           ▼  ▼               │
    ┌──────────────┐            ┌──────────┐              │
    │PendingUpdates│            │   Idle   │──────────────┘
    │    (n=42)    │            └──────────┘
    └──────┬───────┘
           │ StartUpdate
           ▼
    ┌──────────────┐
    │   Updating   │─────────────────┐
    └──────┬───────┘                 │ error
           │                         ▼
     ┌─────┴─────┐              ┌──────────┐
     ▼           ▼              │  Failed  │◄────────┐
┌─────────┐ ┌──────────────┐    └────┬─────┘         │
│  Idle   │ │WaitingReboot │         │               │
└─────────┘ └──────┬───────┘         │ Retry /       │
                   │                 │ Acknowledge   │
                   │ RebootIfRequired│               │
                   ▼                 ▼               │
            ┌──────────┐        ┌──────────┐         │
            │Rebooting │───────►│   Idle   │         │
            └────┬─────┘ error  └──────────┘         │
                 │   │                               │
                 │   └───────────────────────────────┘
                 ▼
            ┌──────────┐
            │Verifying │───────► Idle / Failed
            └──────────┘
```

### Error Recovery

| From State  | Error Type           | Recovery Action                              |
| ----------- | -------------------- | -------------------------------------------- |
| `Querying`  | SSH/osquery failure  | Log error, return to `Idle`, retry on demand |
| `Updating`  | Package manager fail | Transition to `Failed`, preserve error state |
| `Rebooting` | SSH timeout          | Transition to `Failed`, manual intervention  |
| `Verifying` | Health check fail    | Transition to `Failed`, alert operator       |
| `Failed`    | Retry message        | Transition to `Idle`, clear error state      |
| `Failed`    | Acknowledge message  | Clear alert, remain in state for inspection  |

**Failed State Data:**

```rust
struct FailedState {
    previous_state: HostState,
    error: String,
    failed_at: DateTime<Utc>,
    retry_count: u32,
    acknowledged: bool,
}
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
| `eyre` / `color-eyre`  | Error handling (in bins)          |
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
[daemon]
bind = "127.0.0.1:8080"
log_level = "info"  # trace, debug, info, warn, error

[daemon.tls]
enabled = false
cert_path = "/etc/tendhost/cert.pem"
key_path = "/etc/tendhost/key.pem"

[daemon.auth]
enabled = false
tokens = [
    { name = "cli", token_hash = "sha256:..." },
]

[defaults]
user = "root"
ssh_key = "~/.ssh/id_ed25519"  # optional, falls back to ssh-agent

# Host groups for fleet operations
[groups]
production = ["proxmox-1", "debian-vm"]
development = ["fedora-ct"]
docker-hosts = ["centos-docker"]

[[host]]
name = "proxmox-1"
addr = "192.168.1.10"
tags = ["hypervisor", "critical"]

[[host]]
name = "debian-vm"
addr = "192.168.1.20"
user = "admin"  # override default
tags = ["web", "production"]

[host.policy]
auto_reboot = false
maintenance_window = { start = "02:00", end = "06:00", days = ["Sat", "Sun"] }

[[host]]
name = "fedora-ct"
addr = "192.168.1.30"
ssh_key = "~/.ssh/fedora_key"  # override default
tags = ["development"]

[[host]]
name = "centos-docker"
addr = "192.168.1.40"
compose_paths = ["/opt/stacks/monitoring", "/opt/stacks/media"]
tags = ["docker", "monitoring"]

[host.docker]
compose_version = "v2"  # "v1" for docker-compose, "v2" for docker compose
pull_before_update = true
```

### Daemon Fields

| Field                 | Default          | Description                   |
| --------------------- | ---------------- | ----------------------------- |
| `daemon.bind`         | `127.0.0.1:8080` | Address and port to listen on |
| `daemon.log_level`    | `info`           | Minimum log level             |
| `daemon.tls.enabled`  | `false`          | Enable HTTPS/WSS              |
| `daemon.auth.enabled` | `false`          | Require authentication        |

### Host Fields

| Field           | Required | Description                                                  |
| --------------- | -------- | ------------------------------------------------------------ |
| `name`          | yes      | Unique identifier                                            |
| `addr`          | yes      | IP or hostname                                               |
| `user`          | no       | SSH user (default from `[defaults]`)                         |
| `ssh_key`       | no       | Path to private key (default from `[defaults]` or ssh-agent) |
| `compose_paths` | no       | Directories containing docker-compose.yml to manage          |
| `tags`          | no       | List of tags for filtering and grouping                      |

### Host Policy Fields

| Field                | Default | Description                          |
| -------------------- | ------- | ------------------------------------ |
| `auto_reboot`        | `true`  | Automatically reboot when required   |
| `maintenance_window` | `null`  | Time window when updates are allowed |

### Docker Fields

| Field                | Default | Description                                          |
| -------------------- | ------- | ---------------------------------------------------- |
| `compose_version`    | `v2`    | `v1` for `docker-compose`, `v2` for `docker compose` |
| `pull_before_update` | `true`  | Pull images before running compose up                |

## Architecture: Daemon / CLI / TUI

```
┌─────────────────────────────────────────────────────────────────┐
│                        tendhost daemon                          │
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌──────────────────┐    │
│  │ Orchestrator│◄──►│  HostActors │◄──►│  WebSocket Hub   │    │
│  │    Actor    │    │             │    │ (broadcast state)│    │
│  └─────────────┘    └─────────────┘    └────────┬─────────┘    │
│                                                 │              │
│  ┌──────────────────────────────────────────────┴────────────┐ │
│  │                       axum router                         │ │
│  │  REST: /hosts, /fleet/update        WS: /ws/events        │ │
│  │  Docs: /docs (Scalar UI)                                  │ │
│  └───────────────────────────────────────────────────────────┘ │
└───────────────────────────┬─────────────────────┬───────────────┘
                            │                     │
                            ▼                     ▼
                     ┌────────────┐        ┌────────────┐
                     │    CLI     │        │    TUI     │
                     │   curl     │        │   Web UI   │
                     └────────────┘        └────────────┘
                      HTTP requests      WebSocket subscription
```

## API

OpenAPI spec via `utoipa`, documentation via Scalar.

### REST Endpoints

```
# Host management
GET    /hosts                     # list all hosts with status (paginated)
GET    /hosts/:name               # single host details + inventory
DELETE /hosts/:name               # remove host from management
POST   /hosts/:name/retry         # retry failed host
POST   /hosts/:name/acknowledge   # acknowledge failure

# Inventory
GET    /hosts/:name/inventory     # full osquery inventory

# Update operations
POST   /hosts/:name/update        # trigger update { dry_run: bool }
POST   /hosts/:name/reboot        # trigger reboot if required
POST   /fleet/update              # batch update { batch_size, delay_ms, filter }

# Groups and tags
GET    /groups                    # list all groups
GET    /groups/:name              # list hosts in group
GET    /tags                      # list all tags
GET    /hosts?tag=critical        # filter hosts by tag

# System
GET    /health                    # orchestrator health
GET    /docs                      # Scalar API documentation
GET    /openapi.json              # OpenAPI spec
```

### Query Parameters

| Endpoint     | Parameter  | Description                                |
| ------------ | ---------- | ------------------------------------------ |
| `GET /hosts` | `page`     | Page number (default: 1)                   |
| `GET /hosts` | `per_page` | Items per page (default: 50, max: 200)     |
| `GET /hosts` | `tag`      | Filter by tag (repeatable for AND logic)   |
| `GET /hosts` | `state`    | Filter by state (`idle`, `updating`, etc.) |
| `GET /hosts` | `group`    | Filter by group name                       |
| `GET /hosts` | `search`   | Search by hostname (prefix match)          |

### Pagination Response

```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "per_page": 50,
    "total_items": 127,
    "total_pages": 3
  }
}
```

### Fleet Update Filter

```json
{
  "batch_size": 2,
  "delay_ms": 30000,
  "filter": {
    "tags": ["production"],
    "groups": ["web-servers"],
    "exclude_hosts": ["critical-db"]
  }
}
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

## Security

### API Authentication

The daemon API supports token-based authentication:

```toml
[daemon.auth]
enabled = true
tokens = [
    { name = "cli", token_hash = "sha256:..." },
    { name = "tui", token_hash = "sha256:..." },
]
```

Clients include the token in requests:

```
Authorization: Bearer <token>
```

### TLS Configuration

```toml
[daemon.tls]
enabled = true
cert_path = "/etc/tendhost/cert.pem"
key_path = "/etc/tendhost/key.pem"
```

When TLS is enabled, WebSocket connections use `wss://`.

### SSH Key Management

SSH keys can be loaded from:

1. Explicit path in config (`ssh_key = "~/.ssh/id_ed25519"`)
2. SSH agent (default fallback)
3. Environment variable `TENDHOST_SSH_KEY` (base64-encoded)

**Best practices:**

- Use dedicated SSH keys for tendhost
- Restrict key permissions on managed hosts (limit to update commands via `authorized_keys` command restriction)
- Rotate keys periodically

### Audit Logging

All operations are logged with structured metadata:

```rust
#[derive(Serialize)]
struct AuditEvent {
    timestamp: DateTime<Utc>,
    action: String,           // "update_started", "reboot_triggered", etc.
    host: Option<String>,
    user: Option<String>,     // from auth token
    source_ip: IpAddr,
    result: AuditResult,
}
```

Audit logs can be sent to:

- File (JSON lines format)
- Syslog
- External webhook

```toml
[audit]
enabled = true
file = "/var/log/tendhost/audit.jsonl"
syslog = false
webhook = "https://logging.example.com/ingest"
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

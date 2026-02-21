# Implementation Plan: tendhost Daemon

## Overview

This plan implements the tendhost daemon binary, the central service that:

- Loads configuration from `tendhost.toml`
- Initializes the actor system (`OrchestratorActor` + `HostActor`s)
- Serves REST API via axum
- Streams events via WebSocket
- Provides OpenAPI documentation via Scalar

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        tendhost daemon                          │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                      Configuration                          ││
│  │  tendhost.toml → DaemonConfig + Vec<HostConfig>             ││
│  └──────────────────────────────┬──────────────────────────────┘│
│                                 │                               │
│  ┌──────────────────────────────┴──────────────────────────────┐│
│  │                    Actor System                             ││
│  │  ┌─────────────────┐    ┌──────────────────────────────┐    ││
│  │  │ OrchestratorActor│◄──│  HostActorFactory (impl)     │    ││
│  │  │ (event_tx)      │    │  • creates SshExecutor       │    ││
│  │  └────────┬────────┘    │  • creates PackageManager    │    ││
│  │           │              └──────────────────────────────┘    ││
│  │      spawns│                                                 ││
│  │           ▼                                                  ││
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            ││
│  │  │ HostActor   │ │ HostActor   │ │ HostActor   │            ││
│  │  └─────────────┘ └─────────────┘ └─────────────┘            ││
│  └──────────────────────────────────────────────────────────────┘│
│                                 │                               │
│  ┌──────────────────────────────┴──────────────────────────────┐│
│  │                      HTTP Server (axum)                     ││
│  │                                                             ││
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            ││
│  │  │ /hosts/*    │ │ /fleet/*    │ │ /ws/events  │            ││
│  │  └─────────────┘ └─────────────┘ └─────────────┘            ││
│  │                                                             ││
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            ││
│  │  │ /health     │ │ /docs       │ │ /openapi.json│           ││
│  │  └─────────────┘ └─────────────┘ └─────────────┘            ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Current State

The daemon has skeleton files:

- `src/main.rs` - Basic entry point with TODOs
- `src/config.rs` - Empty configuration module
- `src/api/mod.rs` - Module structure
- `src/api/hosts.rs` - Empty
- `src/api/fleet.rs` - Empty
- `src/api/ws.rs` - Empty

---

## Phase 1: Configuration Loading

### Task 1.1: Define Daemon Configuration Types (`config.rs`)

**Priority**: High  
**Estimated effort**: 45 min

```rust
//! Configuration loading and types
//!
//! Parses `tendhost.toml` and provides typed configuration for the daemon.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tendhost_core::HostConfig;

/// Top-level configuration for tendhost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Daemon settings
    #[serde(default)]
    pub daemon: DaemonConfig,
    /// Default settings for hosts
    #[serde(default)]
    pub defaults: DefaultsConfig,
    /// Host groups for fleet operations
    #[serde(default)]
    pub groups: HashMap<String, Vec<String>>,
    /// Individual host configurations
    #[serde(default)]
    pub host: Vec<HostConfig>,
    /// Audit logging settings
    #[serde(default)]
    pub audit: Option<AuditConfig>,
}

/// Daemon server settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Address and port to bind to
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            log_level: default_log_level(),
            tls: None,
            auth: None,
        }
    }
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Whether TLS is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Path to certificate file
    pub cert_path: PathBuf,
    /// Path to private key file
    pub key_path: PathBuf,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Whether authentication is enabled
    #[serde(default)]
    pub enabled: bool,
    /// API tokens
    #[serde(default)]
    pub tokens: Vec<TokenConfig>,
}

/// API token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    /// Token name (for logging)
    pub name: String,
    /// Token hash (sha256:...)
    pub token_hash: String,
}

/// Default settings applied to all hosts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default SSH user
    #[serde(default = "default_user")]
    pub user: String,
    /// Default SSH key path
    pub ssh_key: Option<String>,
}

fn default_user() -> String {
    "root".to_string()
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Path to audit log file
    pub file: Option<PathBuf>,
    /// Whether to send to syslog
    #[serde(default)]
    pub syslog: bool,
    /// Webhook URL for audit events
    pub webhook: Option<String>,
}

impl Config {
    /// Load configuration from file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    pub fn load(path: &PathBuf) -> eyre::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| eyre::eyre!("failed to read config file: {}", e))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| eyre::eyre!("failed to parse config file: {}", e))?;

        Ok(config)
    }

    /// Load from default paths or environment
    pub fn load_default() -> eyre::Result<Self> {
        // Check environment variable
        if let Ok(path) = std::env::var("TENDHOST_CONFIG") {
            return Self::load(&PathBuf::from(path));
        }

        // Try common paths
        let paths = [
            PathBuf::from("tendhost.toml"),
            PathBuf::from("/etc/tendhost/tendhost.toml"),
            dirs::config_dir()
                .map(|p| p.join("tendhost/tendhost.toml"))
                .unwrap_or_default(),
        ];

        for path in paths {
            if path.exists() {
                return Self::load(&path);
            }
        }

        // Return default config if no file found
        tracing::warn!("no config file found, using defaults");
        Ok(Config::default())
    }

    /// Apply defaults to all hosts
    pub fn apply_defaults(&mut self) {
        for host in &mut self.host {
            if host.user == "root" && self.defaults.user != "root" {
                host.user = self.defaults.user.clone();
            }
            if host.ssh_key.is_none() {
                host.ssh_key = self.defaults.ssh_key.clone();
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig::default(),
            defaults: DefaultsConfig::default(),
            groups: HashMap::new(),
            host: Vec::new(),
            audit: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [daemon]
            bind = "0.0.0.0:8080"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.daemon.bind, "0.0.0.0:8080");
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            [daemon]
            bind = "127.0.0.1:8080"
            log_level = "debug"

            [defaults]
            user = "admin"
            ssh_key = "~/.ssh/id_ed25519"

            [groups]
            production = ["host-1", "host-2"]

            [[host]]
            name = "host-1"
            addr = "192.168.1.10"
            tags = ["web", "production"]
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.daemon.log_level, "debug");
        assert_eq!(config.defaults.user, "admin");
        assert_eq!(config.groups.get("production").unwrap().len(), 2);
        assert_eq!(config.host.len(), 1);
    }
}
```

**Acceptance criteria**:

- [ ] All config types from GOALS.md represented
- [ ] Default values applied correctly
- [ ] Multiple config file paths supported
- [ ] Environment variable override
- [ ] Unit tests for parsing

---

### Task 1.2: Add `dirs` Dependency

**Priority**: High  
**Estimated effort**: 5 min

Update `crates/tendhost/Cargo.toml`:

```toml
# Add to [dependencies]
dirs = "5"
```

---

## Phase 2: Application State & Actor Factory

### Task 2.1: Create Application State (`state.rs`)

**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost/src/state.rs`:

```rust
//! Application state shared across HTTP handlers

use std::sync::Arc;

use kameo::actor::ActorRef;
use tendhost_core::OrchestratorActor;

use crate::config::Config;

/// Application state shared across all handlers
pub struct AppState {
    /// Reference to the orchestrator actor
    pub orchestrator: ActorRef<OrchestratorActor>,
    /// Application configuration
    pub config: Arc<Config>,
}

impl AppState {
    /// Create new application state
    pub fn new(orchestrator: ActorRef<OrchestratorActor>, config: Config) -> Self {
        Self {
            orchestrator,
            config: Arc::new(config),
        }
    }
}
```

---

### Task 2.2: Implement `HostActorFactory` (`factory.rs`)

**Priority**: High  
**Estimated effort**: 1 hour

Create `crates/tendhost/src/factory.rs`:

```rust
//! Factory for creating HostActor dependencies

use std::sync::Arc;

use async_trait::async_trait;
use tendhost_core::{HostActorFactory, HostConfig};
use tendhost_exec::{LocalExecutor, RemoteExecutor, SshExecutor, SshConfig};
use tendhost_pkg::{AptManager, DnfManager, PackageManager};

/// Default factory for creating host dependencies
pub struct DefaultHostFactory {
    /// Default SSH user
    default_user: String,
    /// Default SSH key path
    default_ssh_key: Option<String>,
}

impl DefaultHostFactory {
    /// Create a new factory with defaults
    pub fn new(default_user: String, default_ssh_key: Option<String>) -> Self {
        Self {
            default_user,
            default_ssh_key,
        }
    }

    /// Detect package manager type based on remote system
    async fn detect_package_manager(
        &self,
        executor: &Arc<dyn RemoteExecutor>,
    ) -> PackageManagerType {
        // Check for apt (Debian/Ubuntu)
        if executor.command_exists("apt").await.unwrap_or(false) {
            return PackageManagerType::Apt;
        }
        // Check for dnf (Fedora/RHEL 8+)
        if executor.command_exists("dnf").await.unwrap_or(false) {
            return PackageManagerType::Dnf;
        }
        // Check for yum (older RHEL/CentOS)
        if executor.command_exists("yum").await.unwrap_or(false) {
            return PackageManagerType::Dnf; // DnfManager handles yum fallback
        }
        // Default to apt
        PackageManagerType::Apt
    }
}

#[derive(Debug, Clone, Copy)]
enum PackageManagerType {
    Apt,
    Dnf,
}

#[async_trait]
impl HostActorFactory for DefaultHostFactory {
    async fn create_executor(&self, config: &HostConfig) -> Arc<dyn RemoteExecutor> {
        // Check if this is localhost
        let is_local = config.addr == "localhost"
            || config.addr == "127.0.0.1"
            || config.addr.starts_with("local:");

        if is_local {
            return Arc::new(LocalExecutor::new());
        }

        // Create SSH executor
        let user = if config.user.is_empty() {
            &self.default_user
        } else {
            &config.user
        };

        let ssh_key = config.ssh_key.as_ref()
            .or(self.default_ssh_key.as_ref())
            .cloned();

        let ssh_config = SshConfig {
            host: config.addr.clone(),
            port: 22, // TODO: make configurable
            user: user.to_string(),
            key_path: ssh_key,
            connect_timeout: std::time::Duration::from_secs(30),
        };

        match SshExecutor::new(ssh_config).await {
            Ok(executor) => Arc::new(executor),
            Err(e) => {
                tracing::error!(
                    host = %config.name,
                    error = %e,
                    "failed to create SSH executor, falling back to local"
                );
                Arc::new(LocalExecutor::new())
            }
        }
    }

    async fn create_package_manager(
        &self,
        config: &HostConfig,
        executor: Arc<dyn RemoteExecutor>,
    ) -> Arc<dyn PackageManager> {
        let pkg_type = self.detect_package_manager(&executor).await;
        let use_sudo = config.user != "root";

        match pkg_type {
            PackageManagerType::Apt => {
                Arc::new(AptManager::new(executor, use_sudo))
            }
            PackageManagerType::Dnf => {
                Arc::new(DnfManager::new(executor, use_sudo))
            }
        }
    }
}
```

**Acceptance criteria**:

- [ ] Detects localhost vs remote hosts
- [ ] Creates appropriate executor (Local vs SSH)
- [ ] Detects package manager from remote system
- [ ] Handles SSH key fallback
- [ ] Uses sudo when not root

---

## Phase 3: HTTP Server Setup

### Task 3.1: Create Router (`router.rs`)

**Priority**: High  
**Estimated effort**: 45 min

Create `crates/tendhost/src/router.rs`:

```rust
//! HTTP router configuration

use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::api::{fleet, hosts, system, ws};
use crate::state::AppState;

/// Create the application router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Host endpoints
        .route("/hosts", get(hosts::list_hosts))
        .route("/hosts/:name", get(hosts::get_host))
        .route("/hosts/:name", delete(hosts::delete_host))
        .route("/hosts/:name/update", post(hosts::update_host))
        .route("/hosts/:name/reboot", post(hosts::reboot_host))
        .route("/hosts/:name/retry", post(hosts::retry_host))
        .route("/hosts/:name/acknowledge", post(hosts::acknowledge_host))
        .route("/hosts/:name/inventory", get(hosts::get_inventory))
        // Fleet endpoints
        .route("/fleet/update", post(fleet::fleet_update))
        // Groups and tags
        .route("/groups", get(fleet::list_groups))
        .route("/groups/:name", get(fleet::get_group))
        .route("/tags", get(fleet::list_tags))
        // WebSocket
        .route("/ws/events", get(ws::ws_handler))
        // System endpoints
        .route("/health", get(system::health))
        .route("/openapi.json", get(system::openapi_json))
        // OpenAPI documentation
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        // State
        .with_state(state)
}

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "tendhost API",
        description = "Actor-based homelab orchestration system",
        version = "0.1.0",
        license(name = "AGPL"),
    ),
    paths(
        hosts::list_hosts,
        hosts::get_host,
        hosts::delete_host,
        hosts::update_host,
        hosts::reboot_host,
        hosts::retry_host,
        hosts::acknowledge_host,
        hosts::get_inventory,
        fleet::fleet_update,
        fleet::list_groups,
        fleet::get_group,
        fleet::list_tags,
        system::health,
    ),
    components(
        schemas(
            tendhost_api::requests::UpdateRequest,
            tendhost_api::requests::FleetUpdateRequest,
            tendhost_api::requests::FleetUpdateFilter,
            tendhost_api::responses::PaginatedResponse<HostStatusResponse>,
            tendhost_api::responses::Pagination,
            tendhost_api::responses::HealthResponse,
            tendhost_api::events::WsEvent,
            HostStatusResponse,
            HostDetailResponse,
            InventoryResponse,
            UpdateResponse,
            FleetUpdateResponse,
            GroupResponse,
            TagResponse,
            ApiError,
        )
    ),
    tags(
        (name = "hosts", description = "Host management endpoints"),
        (name = "fleet", description = "Fleet-wide operations"),
        (name = "system", description = "System endpoints"),
    )
)]
struct ApiDoc;

// Response types for OpenAPI (defined in api modules, re-exported here)
use crate::api::hosts::{HostStatusResponse, HostDetailResponse, InventoryResponse, UpdateResponse};
use crate::api::fleet::{FleetUpdateResponse, GroupResponse, TagResponse};
use crate::api::ApiError;
```

**Acceptance criteria**:

- [ ] All routes from GOALS.md implemented
- [ ] OpenAPI documentation complete
- [ ] Scalar UI at `/docs`
- [ ] State properly shared

---

### Task 3.2: Create API Error Type (`api/error.rs`)

**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost/src/api/error.rs`:

```rust
//! API error types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Additional details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ApiError {
    /// Create a not found error
    pub fn not_found(resource: &str) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message: format!("{} not found", resource),
            details: None,
        }
    }

    /// Create a bad request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "BAD_REQUEST".to_string(),
            message: message.into(),
            details: None,
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "CONFLICT".to_string(),
            message: message.into(),
            details: None,
        }
    }

    /// Create an invalid state error
    pub fn invalid_state(from: &str, to: &str) -> Self {
        Self {
            code: "INVALID_STATE".to_string(),
            message: format!("cannot transition from {} to {}", from, to),
            details: None,
        }
    }
}

/// Wrapper for API errors with status codes
pub struct AppError {
    pub status: StatusCode,
    pub error: ApiError,
}

impl AppError {
    pub fn not_found(resource: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: ApiError::not_found(resource),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: ApiError::bad_request(message),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: ApiError::internal(message),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            error: ApiError::conflict(message),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(self.error)).into_response()
    }
}

impl From<tendhost_core::CoreError> for AppError {
    fn from(err: tendhost_core::CoreError) -> Self {
        match err {
            tendhost_core::CoreError::HostNotFound(name) => {
                AppError::not_found(&format!("host '{}'", name))
            }
            tendhost_core::CoreError::HostAlreadyExists(name) => {
                AppError::conflict(format!("host '{}' already exists", name))
            }
            tendhost_core::CoreError::InvalidTransition { from, to } => {
                Self {
                    status: StatusCode::CONFLICT,
                    error: ApiError::invalid_state(&from.to_string(), &to.to_string()),
                }
            }
            tendhost_core::CoreError::HostFailed(msg) => {
                AppError::conflict(format!("host is in failed state: {}", msg))
            }
            _ => AppError::internal(err.to_string()),
        }
    }
}
```

---

## Phase 4: Host API Handlers

### Task 4.1: Implement Host Handlers (`api/hosts.rs`)

**Priority**: High  
**Estimated effort**: 2 hours

Rewrite `crates/tendhost/src/api/hosts.rs`:

```rust
//! Host API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use tendhost_api::requests::UpdateRequest;
use tendhost_core::{
    GetHostStatus, HostState, HostStatus, ListHosts, QueryHostInventory,
    RetryHost, AcknowledgeHost, TriggerHostUpdate, UnregisterHost,
};

use crate::api::error::AppError;
use crate::state::AppState;

/// Host status response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HostStatusResponse {
    /// Host name
    pub name: String,
    /// Current state
    pub state: String,
    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    /// Number of pending updates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_updates: Option<u32>,
    /// Error message if in failed state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Tags assigned to host
    pub tags: Vec<String>,
}

impl From<HostStatus> for HostStatusResponse {
    fn from(status: HostStatus) -> Self {
        Self {
            name: status.name,
            state: status.state.to_string(),
            last_updated: status.last_updated.map(|t| t.to_rfc3339()),
            pending_updates: status.pending_updates,
            error: status.error,
            tags: status.tags,
        }
    }
}

/// Host detail response (includes inventory)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HostDetailResponse {
    /// Host status
    #[serde(flatten)]
    pub status: HostStatusResponse,
    /// Host address
    pub addr: String,
    /// SSH user
    pub user: String,
}

/// Inventory response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InventoryResponse {
    /// Number of pending updates
    pub pending_updates: u32,
    /// List of package names with updates
    pub packages: Vec<String>,
}

/// Update operation response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateResponse {
    /// Whether update succeeded
    pub success: bool,
    /// Number of packages upgraded
    pub upgraded_count: u32,
    /// Whether reboot is required
    pub reboot_required: bool,
}

/// Query parameters for listing hosts
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListHostsQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u64,
    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: u64,
    /// Filter by tag
    pub tag: Option<String>,
    /// Filter by state
    pub state: Option<String>,
    /// Filter by group
    pub group: Option<String>,
    /// Search by hostname prefix
    pub search: Option<String>,
}

fn default_page() -> u64 { 1 }
fn default_per_page() -> u64 { 50 }

/// List all hosts with filtering and pagination
#[utoipa::path(
    get,
    path = "/hosts",
    params(ListHostsQuery),
    responses(
        (status = 200, description = "List of hosts", body = PaginatedResponse<HostStatusResponse>),
    ),
    tag = "hosts"
)]
pub async fn list_hosts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListHostsQuery>,
) -> Result<Json<tendhost_api::responses::PaginatedResponse<HostStatusResponse>>, AppError> {
    let hosts = state.orchestrator.ask(ListHosts).await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let mut hosts: Vec<_> = hosts.into_iter().map(HostStatusResponse::from).collect();

    // Apply filters
    if let Some(ref tag) = params.tag {
        hosts.retain(|h| h.tags.contains(tag));
    }
    if let Some(ref state_filter) = params.state {
        hosts.retain(|h| h.state == *state_filter);
    }
    if let Some(ref search) = params.search {
        hosts.retain(|h| h.name.starts_with(search));
    }
    if let Some(ref group) = params.group {
        if let Some(group_hosts) = state.config.groups.get(group) {
            hosts.retain(|h| group_hosts.contains(&h.name));
        }
    }

    // Pagination
    let total_items = hosts.len() as u64;
    let total_pages = (total_items + params.per_page - 1) / params.per_page;
    let start = ((params.page - 1) * params.per_page) as usize;
    let end = (start + params.per_page as usize).min(hosts.len());
    let page_hosts = hosts[start..end].to_vec();

    Ok(Json(tendhost_api::responses::PaginatedResponse {
        data: page_hosts,
        pagination: tendhost_api::responses::Pagination {
            page: params.page,
            per_page: params.per_page,
            total_items,
            total_pages,
        },
    }))
}

/// Get a single host's details
#[utoipa::path(
    get,
    path = "/hosts/{name}",
    params(
        ("name" = String, Path, description = "Host name")
    ),
    responses(
        (status = 200, description = "Host details", body = HostDetailResponse),
        (status = 404, description = "Host not found", body = ApiError),
    ),
    tag = "hosts"
)]
pub async fn get_host(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<HostDetailResponse>, AppError> {
    let status = state.orchestrator.ask(GetHostStatus { hostname: name.clone() }).await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::from)?;

    let config = state.config.host.iter()
        .find(|h| h.name == name)
        .ok_or_else(|| AppError::not_found(&format!("host '{}'", name)))?;

    Ok(Json(HostDetailResponse {
        status: HostStatusResponse::from(status),
        addr: config.addr.clone(),
        user: config.user.clone(),
    }))
}

/// Remove a host from management
#[utoipa::path(
    delete,
    path = "/hosts/{name}",
    params(
        ("name" = String, Path, description = "Host name")
    ),
    responses(
        (status = 204, description = "Host removed"),
        (status = 404, description = "Host not found", body = ApiError),
    ),
    tag = "hosts"
)]
pub async fn delete_host(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<(), AppError> {
    state.orchestrator.ask(UnregisterHost { hostname: name }).await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::from)?;

    Ok(())
}

/// Trigger package update for a host
#[utoipa::path(
    post,
    path = "/hosts/{name}/update",
    params(
        ("name" = String, Path, description = "Host name")
    ),
    request_body = UpdateRequest,
    responses(
        (status = 200, description = "Update result", body = UpdateResponse),
        (status = 404, description = "Host not found", body = ApiError),
        (status = 409, description = "Invalid state", body = ApiError),
    ),
    tag = "hosts"
)]
pub async fn update_host(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<UpdateResponse>, AppError> {
    let result = state.orchestrator.ask(TriggerHostUpdate {
        hostname: name,
        dry_run: req.dry_run,
    }).await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::from)?;

    Ok(Json(UpdateResponse {
        success: result.success,
        upgraded_count: result.upgraded_count,
        reboot_required: result.reboot_required,
    }))
}

/// Trigger reboot for a host if required
#[utoipa::path(
    post,
    path = "/hosts/{name}/reboot",
    params(
        ("name" = String, Path, description = "Host name")
    ),
    responses(
        (status = 200, description = "Reboot triggered", body = bool),
        (status = 404, description = "Host not found", body = ApiError),
        (status = 409, description = "Invalid state", body = ApiError),
    ),
    tag = "hosts"
)]
pub async fn reboot_host(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<bool>, AppError> {
    // TODO: Implement RebootHost message
    // For now, just return false
    Ok(Json(false))
}

/// Retry a failed host
#[utoipa::path(
    post,
    path = "/hosts/{name}/retry",
    params(
        ("name" = String, Path, description = "Host name")
    ),
    responses(
        (status = 204, description = "Retry triggered"),
        (status = 404, description = "Host not found", body = ApiError),
        (status = 409, description = "Host not in failed state", body = ApiError),
    ),
    tag = "hosts"
)]
pub async fn retry_host(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<(), AppError> {
    state.orchestrator.ask(RetryHost { hostname: name }).await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::from)?;

    Ok(())
}

/// Acknowledge a failed host
#[utoipa::path(
    post,
    path = "/hosts/{name}/acknowledge",
    params(
        ("name" = String, Path, description = "Host name")
    ),
    responses(
        (status = 204, description = "Failure acknowledged"),
        (status = 404, description = "Host not found", body = ApiError),
        (status = 409, description = "Host not in failed state", body = ApiError),
    ),
    tag = "hosts"
)]
pub async fn acknowledge_host(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<(), AppError> {
    state.orchestrator.ask(AcknowledgeHost { hostname: name }).await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::from)?;

    Ok(())
}

/// Get host inventory
#[utoipa::path(
    get,
    path = "/hosts/{name}/inventory",
    params(
        ("name" = String, Path, description = "Host name")
    ),
    responses(
        (status = 200, description = "Host inventory", body = InventoryResponse),
        (status = 404, description = "Host not found", body = ApiError),
    ),
    tag = "hosts"
)]
pub async fn get_inventory(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<InventoryResponse>, AppError> {
    let result = state.orchestrator.ask(QueryHostInventory { hostname: name }).await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::from)?;

    Ok(Json(InventoryResponse {
        pending_updates: result.pending_updates,
        packages: result.packages,
    }))
}
```

**Acceptance criteria**:

- [ ] All host endpoints implemented
- [ ] Proper error handling
- [ ] Pagination working
- [ ] Filtering working
- [ ] OpenAPI annotations complete

---

## Phase 5: Fleet API Handlers

### Task 5.1: Implement Fleet Handlers (`api/fleet.rs`)

**Priority**: High  
**Estimated effort**: 1 hour

Rewrite `crates/tendhost/src/api/fleet.rs`:

```rust
//! Fleet API handlers

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use tendhost_api::requests::FleetUpdateRequest;
use tendhost_core::{FleetFilter, FleetUpdateConfig, ListHosts, TriggerFleetUpdate};

use crate::api::error::AppError;
use crate::state::AppState;

/// Fleet update response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FleetUpdateResponse {
    /// Total hosts in update batch
    pub total_hosts: usize,
    /// Hosts that completed successfully
    pub completed: usize,
    /// Hosts that failed
    pub failed: usize,
}

/// Group response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GroupResponse {
    /// Group name
    pub name: String,
    /// Hosts in the group
    pub hosts: Vec<String>,
}

/// Tag response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TagResponse {
    /// Tag name
    pub name: String,
    /// Number of hosts with this tag
    pub count: usize,
}

/// Trigger fleet-wide update
#[utoipa::path(
    post,
    path = "/fleet/update",
    request_body = FleetUpdateRequest,
    responses(
        (status = 200, description = "Fleet update result", body = FleetUpdateResponse),
    ),
    tag = "fleet"
)]
pub async fn fleet_update(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FleetUpdateRequest>,
) -> Result<Json<FleetUpdateResponse>, AppError> {
    let filter = req.filter.map(|f| FleetFilter {
        tags: f.tags.unwrap_or_default(),
        groups: f.groups.unwrap_or_default(),
        exclude_hosts: f.exclude_hosts.unwrap_or_default(),
    });

    let config = FleetUpdateConfig {
        batch_size: req.batch_size,
        delay_between_batches: Duration::from_millis(req.delay_ms),
        filter,
        dry_run: false,
    };

    let progress = state.orchestrator.ask(TriggerFleetUpdate { config }).await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::from)?;

    Ok(Json(FleetUpdateResponse {
        total_hosts: progress.total_hosts,
        completed: progress.completed,
        failed: progress.failed,
    }))
}

/// List all groups
#[utoipa::path(
    get,
    path = "/groups",
    responses(
        (status = 200, description = "List of groups", body = Vec<GroupResponse>),
    ),
    tag = "fleet"
)]
pub async fn list_groups(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<GroupResponse>> {
    let groups: Vec<_> = state.config.groups.iter()
        .map(|(name, hosts)| GroupResponse {
            name: name.clone(),
            hosts: hosts.clone(),
        })
        .collect();

    Json(groups)
}

/// Get hosts in a group
#[utoipa::path(
    get,
    path = "/groups/{name}",
    params(
        ("name" = String, Path, description = "Group name")
    ),
    responses(
        (status = 200, description = "Group details", body = GroupResponse),
        (status = 404, description = "Group not found", body = ApiError),
    ),
    tag = "fleet"
)]
pub async fn get_group(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<GroupResponse>, AppError> {
    let hosts = state.config.groups.get(&name)
        .ok_or_else(|| AppError::not_found(&format!("group '{}'", name)))?;

    Ok(Json(GroupResponse {
        name,
        hosts: hosts.clone(),
    }))
}

/// List all tags
#[utoipa::path(
    get,
    path = "/tags",
    responses(
        (status = 200, description = "List of tags with counts", body = Vec<TagResponse>),
    ),
    tag = "fleet"
)]
pub async fn list_tags(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TagResponse>>, AppError> {
    let hosts = state.orchestrator.ask(ListHosts).await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // Count tags across all hosts
    let mut tag_counts = std::collections::HashMap::new();
    for host in hosts {
        for tag in host.tags {
            *tag_counts.entry(tag).or_insert(0) += 1;
        }
    }

    let tags: Vec<_> = tag_counts.into_iter()
        .map(|(name, count)| TagResponse { name, count })
        .collect();

    Ok(Json(tags))
}
```

---

## Phase 6: WebSocket Handler

### Task 6.1: Implement WebSocket Handler (`api/ws.rs`)

**Priority**: High  
**Estimated effort**: 1.5 hours

Rewrite `crates/tendhost/src/api/ws.rs`:

```rust
//! WebSocket handler for real-time events

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tracing::{error, info};

use tendhost_api::events::WsEvent;
use tendhost_core::OrchestratorActor;

use crate::state::AppState;

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to events from orchestrator
    let mut event_rx = state.orchestrator.ask(Subscribe).await
        .expect("failed to subscribe to events");

    info!("WebSocket client connected");

    // Task to forward events to WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = match serde_json::to_string(&event) {
                Ok(json) => Message::Text(json),
                Err(e) => {
                    error!("failed to serialize event: {}", e);
                    continue;
                }
            };

            if sender.send(msg).await.is_err() {
                // Client disconnected
                break;
            }
        }
    });

    // Task to receive messages from WebSocket (for future bidirectional support)
    let recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(data)) => {
                    // Pong is handled automatically by axum
                }
                Ok(_) => {
                    // Ignore other messages for now
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    info!("WebSocket client disconnected");
}

// Message to subscribe to events
// This needs to be added to tendhost-core
#[derive(Debug)]
pub struct Subscribe;

// Implementation in tendhost-core would look like:
// impl Message<Subscribe> for OrchestratorActor {
//     type Reply = broadcast::Receiver<WsEvent>;
//
//     async fn handle(&mut self, _msg: Subscribe, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
//         self.event_tx.subscribe()
//     }
// }
```

**Note**: This requires adding a `Subscribe` message to `tendhost-core` OrchestratorActor.

---

## Phase 7: System Endpoints

### Task 7.1: Implement System Handlers (`api/system.rs`)

**Priority**: Medium  
**Estimated effort**: 30 min

Create `crates/tendhost/src/api/system.rs`:

```rust
//! System endpoints (health, docs)

use axum::Json;
use utoipa::OpenApi;

use tendhost_api::responses::HealthResponse;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    tag = "system"
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
    })
}

/// OpenAPI JSON endpoint
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(crate::router::ApiDoc::openapi())
}
```

---

## Phase 8: Main Entry Point

### Task 8.1: Implement Main Function (`main.rs`)

**Priority**: High  
**Estimated effort**: 1 hour

Rewrite `crates/tendhost/src/main.rs`:

````rust
//! tendhost daemon
//!
//! Actor-based homelab orchestration system using axum HTTP server and kameo actors.
//!
//! # Usage
//! ```bash
//! # Run with default config
//! tendhost
//!
//! # Run with specific config file
//! TENDHOST_CONFIG=/path/to/tendhost.toml tendhost
//! ```

use std::sync::Arc;

use color_eyre::Result;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use tendhost_core::{OrchestratorActor, OrchestratorActorArgs, RegisterHost};

mod api;
mod config;
mod factory;
mod router;
mod state;

use config::Config;
use factory::DefaultHostFactory;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Load configuration
    let mut config = Config::load_default()?;
    config.apply_defaults();

    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.daemon.log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    info!("tendhost daemon starting...");
    info!(bind = %config.daemon.bind, "configuration loaded");

    // Create host factory
    let host_factory = Arc::new(DefaultHostFactory::new(
        config.defaults.user.clone(),
        config.defaults.ssh_key.clone(),
    ));

    // Spawn orchestrator actor
    let orchestrator_args = OrchestratorActorArgs {
        event_channel_capacity: 1024,
        host_factory,
    };
    let orchestrator = OrchestratorActor::spawn(orchestrator_args);

    info!("orchestrator actor started");

    // Register hosts from config
    for host_config in &config.host {
        match orchestrator.ask(RegisterHost { config: host_config.clone() }).await {
            Ok(Ok(())) => {
                info!(host = %host_config.name, "registered host");
            }
            Ok(Err(e)) => {
                error!(host = %host_config.name, error = %e, "failed to register host");
            }
            Err(e) => {
                error!(host = %host_config.name, error = %e, "actor error registering host");
            }
        }
    }

    // Create application state
    let state = Arc::new(AppState::new(orchestrator.clone(), config.clone()));

    // Create router
    let app = router::create_router(state);

    // Create listener
    let listener = tokio::net::TcpListener::bind(&config.daemon.bind).await?;
    info!(addr = %config.daemon.bind, "listening for connections");

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("shutting down...");

    // Stop orchestrator (which stops all host actors)
    orchestrator.stop_gracefully().await.ok();

    info!("shutdown complete");
    Ok(())
}

/// Wait for shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("shutdown signal received");
}
````

---

## Phase 9: Module Organization

### Task 9.1: Update Module Structure

**Priority**: Medium  
**Estimated effort**: 15 min

Update `crates/tendhost/src/api/mod.rs`:

```rust
//! API route handlers

pub mod error;
pub mod fleet;
pub mod hosts;
pub mod system;
pub mod ws;

pub use error::{ApiError, AppError};
```

---

## Phase 10: Dependencies Update

### Task 10.1: Update Cargo.toml

**Priority**: High  
**Estimated effort**: 10 min

Update `crates/tendhost/Cargo.toml`:

```toml
[package]
name = "tendhost"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "tendhost"
path = "src/main.rs"

[dependencies]
# Web server
axum = { workspace = true, features = ["ws"] }
tokio = { workspace = true }
tokio-tungstenite = { workspace = true }
futures-util = "0.3"

# Error handling
color-eyre = { workspace = true }
eyre = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }

# OpenAPI
utoipa = { workspace = true }
utoipa-scalar = { workspace = true }

# Logging
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

# Utilities
chrono = { workspace = true }
dirs = "5"

# Internal crates
tendhost-api = { workspace = true }
tendhost-core = { workspace = true }
tendhost-inventory = { workspace = true }
tendhost-pkg = { workspace = true }
tendhost-exec = { workspace = true }
```

---

## Phase 11: Integration Testing

### Task 11.1: Create Integration Tests

**Priority**: Medium  
**Estimated effort**: 2 hours

Create `crates/tendhost/tests/api_integration.rs`:

```rust
//! API integration tests

use axum::http::StatusCode;
use axum_test::TestServer;

// Test helpers and fixtures would go here
// Integration tests require setting up mock actors

#[tokio::test]
async fn test_health_endpoint() {
    // TODO: Set up test server with mock state
    // let app = create_test_app();
    // let server = TestServer::new(app).unwrap();

    // let response = server.get("/health").await;
    // assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_hosts_empty() {
    // TODO: Test empty host list
}

#[tokio::test]
async fn test_list_hosts_with_pagination() {
    // TODO: Test pagination
}

#[tokio::test]
async fn test_get_host_not_found() {
    // TODO: Test 404 response
}
```

---

## Summary

### File Changes Required

| File                       | Action  | Description                                   |
| -------------------------- | ------- | --------------------------------------------- |
| `Cargo.toml`               | Modify  | Add `dirs`, `futures-util`, axum `ws` feature |
| `src/main.rs`              | Rewrite | Full daemon implementation                    |
| `src/config.rs`            | Rewrite | Configuration loading                         |
| `src/state.rs`             | Create  | Application state                             |
| `src/factory.rs`           | Create  | `HostActorFactory` implementation             |
| `src/router.rs`            | Create  | Router and `OpenAPI` setup                    |
| `src/api/mod.rs`           | Modify  | Add error module                              |
| `src/api/error.rs`         | Create  | API error types                               |
| `src/api/hosts.rs`         | Rewrite | Host handlers                                 |
| `src/api/fleet.rs`         | Rewrite | Fleet handlers                                |
| `src/api/ws.rs`            | Rewrite | WebSocket handler                             |
| `src/api/system.rs`        | Create  | Health and docs handlers                      |
| `tests/api_integration.rs` | Create  | Integration tests                             |

### Required Changes to tendhost-core

1. **Add `Subscribe` message** to `OrchestratorActor`:

```rust
pub struct Subscribe;

impl Message<Subscribe> for OrchestratorActor {
    type Reply = broadcast::Receiver<WsEvent>;

    async fn handle(&mut self, _msg: Subscribe, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.event_tx.subscribe()
    }
}
```

2. **Export `event_tx` method** or provide subscription mechanism.

### Estimated Total Effort

| Phase                     | Effort        |
| ------------------------- | ------------- |
| Phase 1: Configuration    | 50 min        |
| Phase 2: State & Factory  | 1.5 hours     |
| Phase 3: Router           | 45 min        |
| Phase 4: Host API         | 2.5 hours     |
| Phase 5: Fleet API        | 1 hour        |
| Phase 6: WebSocket        | 1.5 hours     |
| Phase 7: System Endpoints | 30 min        |
| Phase 8: Main Entry       | 1 hour        |
| Phase 9: Module Org       | 15 min        |
| Phase 10: Dependencies    | 10 min        |
| Phase 11: Testing         | 2 hours       |
| **Total**                 | **~12 hours** |

### Priority Order

1. **Phase 1**: Configuration (blocks everything)
2. **Phase 2**: State & Factory (blocks HTTP server)
3. **Phase 10**: Dependencies (needed early)
4. **Phase 3**: Router (framework)
5. **Phase 8**: Main Entry (basic runnable daemon)
6. **Phase 4**: Host API (core functionality)
7. **Phase 5**: Fleet API (fleet operations)
8. **Phase 6**: WebSocket (real-time)
9. **Phase 7**: System Endpoints (docs, health)
10. **Phase 9**: Module Organization (cleanup)
11. **Phase 11**: Testing (validation)

### Dependencies

- **Blocks**: `tendhost-cli`, `tendhost-tui` (need running daemon)
- **Blocked by**:
  - `tendhost-core` (actors, messages) ✅ COMPLETE
  - `tendhost-exec` (SSH execution) ✅ COMPLETE
  - `tendhost-pkg` (package managers) ✅ COMPLETE
  - `tendhost-inventory` (osquery) ✅ COMPLETE

### Notes

- WebSocket implementation requires adding `Subscribe` message to core
- Consider adding `axum-test` for integration testing
- TLS support can be added later (requires `axum-server` with rustls)
- Authentication middleware can be added as Phase 12

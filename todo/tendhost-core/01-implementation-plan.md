# Implementation Plan: tendhost-core

## Overview

This plan implements the core actor framework for tendhost, including:
- `HostActor`: Per-host state machine and update orchestration
- `OrchestratorActor`: Fleet-wide coordination and host registry
- Message types with kameo 0.19 integration
- Error handling and event broadcasting

## Dependencies

Using kameo 0.19.2 with companion crates:
- `kameo = "0.19"` - Core actor framework
- `kameo_actors = "0.4"` - Pre-built actor patterns (pubsub, etc.)
- `kameo_macros = "0.19"` - Derive macros for actors

### kameo 0.19 API Key Points

```rust
// Actor trait (0.19 style)
impl Actor for MyActor {
    type Args = InitArgs;  // Arguments passed to on_start
    type Error = MyError;  // Error type for lifecycle hooks

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(MyActor { /* ... */ })
    }

    async fn on_stop(&mut self, actor_ref: WeakActorRef<Self>, reason: ActorStopReason) -> Result<(), Self::Error> {
        Ok(())
    }
}

// Message trait (0.19 style)
impl Message<MyMessage> for MyActor {
    type Reply = ResultType;

    async fn handle(&mut self, msg: MyMessage, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // handle message
    }
}

// Spawning
let actor_ref = MyActor::spawn(args);
let actor_ref = MyActor::spawn_with_mailbox(args, kameo::mailbox::unbounded());

// Derive macro for simple actors
#[derive(Actor)]
struct SimpleActor { /* fields */ }
```

---

## Phase 1: Foundation

### Task 1.1: Create Error Types (`error.rs`)
**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost-core/src/error.rs`:

```rust
//! Core error types for tendhost-core

use thiserror::Error;

use crate::state::HostState;

/// Errors that can occur in core actor operations
#[derive(Error, Debug, Clone)]
pub enum CoreError {
    /// Host not found in registry
    #[error("host not found: {0}")]
    HostNotFound(String),

    /// Host already exists in registry
    #[error("host already exists: {0}")]
    HostAlreadyExists(String),

    /// Invalid state transition attempted
    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// Current state
        from: HostState,
        /// Attempted target state
        to: HostState,
    },

    /// SSH execution failed
    #[error("SSH execution failed: {0}")]
    SshError(String),

    /// Package manager operation failed
    #[error("package manager error: {0}")]
    PackageError(String),

    /// Inventory query failed
    #[error("inventory query failed: {0}")]
    InventoryError(String),

    /// Host is in failed state and cannot process request
    #[error("host is in failed state: {0}")]
    HostFailed(String),

    /// Operation timed out
    #[error("operation timeout")]
    Timeout,

    /// Actor communication error
    #[error("actor communication error: {0}")]
    ActorError(String),

    /// Configuration error
    #[error("configuration error: {0}")]
    ConfigError(String),
}
```

**Acceptance criteria**:
- [ ] Error enum covers all failure modes from GOALS.md
- [ ] Implements `std::error::Error` via thiserror
- [ ] `Clone` derive for use in actor messages
- [ ] Public in lib.rs

---

### Task 1.2: Create Host Configuration Types (`config.rs`)
**Priority**: High  
**Estimated effort**: 45 min

Create `crates/tendhost-core/src/config.rs`:

```rust
//! Configuration types for hosts and fleet operations

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Configuration for a single managed host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    /// Unique hostname identifier
    pub name: String,
    /// IP address or hostname for SSH connection
    pub addr: String,
    /// SSH user (defaults to root)
    #[serde(default = "default_user")]
    pub user: String,
    /// Path to SSH private key (optional, falls back to ssh-agent)
    pub ssh_key: Option<String>,
    /// Docker compose directories to manage
    #[serde(default)]
    pub compose_paths: Vec<String>,
    /// Tags for filtering and grouping
    #[serde(default)]
    pub tags: Vec<String>,
    /// Host-specific policy settings
    #[serde(default)]
    pub policy: HostPolicy,
}

fn default_user() -> String {
    "root".to_string()
}

/// Policy settings for host operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostPolicy {
    /// Automatically reboot when kernel updates require it
    #[serde(default = "default_auto_reboot")]
    pub auto_reboot: bool,
    /// Time window when updates are allowed
    pub maintenance_window: Option<MaintenanceWindow>,
}

fn default_auto_reboot() -> bool {
    true
}

/// Time window for maintenance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    /// Start time in HH:MM format
    pub start: String,
    /// End time in HH:MM format
    pub end: String,
    /// Days of week when window is active
    pub days: Vec<String>,
}

/// Fleet update configuration
#[derive(Debug, Clone)]
pub struct FleetUpdateConfig {
    /// Number of hosts to update in parallel
    pub batch_size: usize,
    /// Delay between batches
    pub delay_between_batches: Duration,
    /// Optional filter for selecting hosts
    pub filter: Option<FleetFilter>,
    /// Whether to perform a dry run
    pub dry_run: bool,
}

impl Default for FleetUpdateConfig {
    fn default() -> Self {
        Self {
            batch_size: 2,
            delay_between_batches: Duration::from_secs(30),
            filter: None,
            dry_run: false,
        }
    }
}

/// Filter for fleet operations
#[derive(Debug, Clone, Default)]
pub struct FleetFilter {
    /// Only include hosts with these tags (AND logic)
    pub tags: Vec<String>,
    /// Only include hosts in these groups
    pub groups: Vec<String>,
    /// Exclude these specific hosts
    pub exclude_hosts: Vec<String>,
}
```

**Acceptance criteria**:
- [ ] All config types from GOALS.md represented
- [ ] Serde derives for serialization
- [ ] `Default` implemented where sensible
- [ ] Documentation on all public items
- [ ] Public in lib.rs

---

## Phase 2: Enhanced State Machine

### Task 2.1: Enhance State Types (`state.rs`)
**Priority**: High  
**Estimated effort**: 1 hour

Enhance existing `state.rs`:

```rust
//! Host state machine types

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// States for a `HostActor` state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostState {
    /// Host is idle and ready for operations
    Idle,
    /// Querying inventory via osquery
    Querying,
    /// Updates available, waiting for trigger
    PendingUpdates,
    /// Performing package updates
    Updating,
    /// Updates complete, reboot required
    WaitingReboot,
    /// Rebooting host
    Rebooting,
    /// Verifying host health after reboot
    Verifying,
    /// Host is in failed state
    Failed,
}

impl HostState {
    /// Check if transition to target state is valid
    ///
    /// Validates against the state machine defined in GOALS.md.
    pub fn can_transition_to(&self, target: HostState) -> bool {
        use HostState::*;
        matches!(
            (self, target),
            // Normal flow
            (Idle, Querying)
                | (Querying, Idle) // no updates or error recovery
                | (Querying, PendingUpdates)
                | (PendingUpdates, Updating)
                | (Updating, Idle) // no reboot needed
                | (Updating, WaitingReboot)
                | (WaitingReboot, Rebooting)
                | (Rebooting, Verifying)
                | (Rebooting, Idle) // error recovery
                | (Verifying, Idle)
                | (Verifying, Failed)
                // Error transitions (any state can fail)
                | (Querying, Failed)
                | (Updating, Failed)
                | (Rebooting, Failed)
                // Recovery from failed
                | (Failed, Idle)
        )
    }

    /// Whether this state represents an active operation
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            HostState::Querying
                | HostState::Updating
                | HostState::Rebooting
                | HostState::Verifying
        )
    }

    /// Whether operations can be started from this state
    pub fn can_start_operation(&self) -> bool {
        matches!(self, HostState::Idle | HostState::PendingUpdates)
    }
}

impl fmt::Display for HostState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Idle => "idle",
            Self::Querying => "querying",
            Self::PendingUpdates => "pending_updates",
            Self::Updating => "updating",
            Self::WaitingReboot => "waiting_reboot",
            Self::Rebooting => "rebooting",
            Self::Verifying => "verifying",
            Self::Failed => "failed",
        };
        write!(f, "{s}")
    }
}

impl Default for HostState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Detailed context when host is in `PendingUpdates` state
#[derive(Debug, Clone)]
pub struct PendingUpdatesContext {
    /// Number of packages with available updates
    pub package_count: u32,
    /// Names of packages with updates
    pub packages: Vec<String>,
    /// When the inventory was queried
    pub queried_at: DateTime<Utc>,
}

/// Failed state details with recovery information
#[derive(Debug, Clone)]
pub struct FailedStateContext {
    /// State before failure occurred
    pub previous_state: HostState,
    /// Error message describing the failure
    pub error: String,
    /// When the failure occurred
    pub failed_at: DateTime<Utc>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Whether operator has acknowledged the failure
    pub acknowledged: bool,
}

impl FailedStateContext {
    /// Create a new failed state context
    pub fn new(previous_state: HostState, error: impl Into<String>) -> Self {
        Self {
            previous_state,
            error: error.into(),
            failed_at: Utc::now(),
            retry_count: 0,
            acknowledged: false,
        }
    }

    /// Increment the retry counter
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Mark the failure as acknowledged
    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        use HostState::*;

        // Normal flow
        assert!(Idle.can_transition_to(Querying));
        assert!(Querying.can_transition_to(PendingUpdates));
        assert!(PendingUpdates.can_transition_to(Updating));
        assert!(Updating.can_transition_to(WaitingReboot));
        assert!(Updating.can_transition_to(Idle));
        assert!(WaitingReboot.can_transition_to(Rebooting));
        assert!(Rebooting.can_transition_to(Verifying));
        assert!(Verifying.can_transition_to(Idle));

        // Error recovery
        assert!(Querying.can_transition_to(Idle));
        assert!(Rebooting.can_transition_to(Idle));

        // Error transitions
        assert!(Querying.can_transition_to(Failed));
        assert!(Updating.can_transition_to(Failed));
        assert!(Rebooting.can_transition_to(Failed));

        // Recovery from failed
        assert!(Failed.can_transition_to(Idle));
    }

    #[test]
    fn test_invalid_transitions() {
        use HostState::*;

        assert!(!Idle.can_transition_to(Updating)); // must query first
        assert!(!Querying.can_transition_to(Rebooting));
        assert!(!PendingUpdates.can_transition_to(Verifying));
        assert!(!Idle.can_transition_to(Idle)); // no self-transition
    }

    #[test]
    fn test_is_busy() {
        use HostState::*;

        assert!(!Idle.is_busy());
        assert!(!Failed.is_busy());
        assert!(!PendingUpdates.is_busy());
        assert!(!WaitingReboot.is_busy());

        assert!(Querying.is_busy());
        assert!(Updating.is_busy());
        assert!(Rebooting.is_busy());
        assert!(Verifying.is_busy());
    }

    #[test]
    fn test_display() {
        assert_eq!(HostState::Idle.to_string(), "idle");
        assert_eq!(HostState::PendingUpdates.to_string(), "pending_updates");
    }
}
```

**Acceptance criteria**:
- [ ] `can_transition_to()` validates all transitions from GOALS.md diagram
- [ ] Serde serialization for API compatibility
- [ ] `Display` impl for logging
- [ ] Context types for stateful states
- [ ] Unit tests for transition validation

---

## Phase 3: Message Definitions

### Task 3.1: Define Message Types (`message.rs`)
**Priority**: High  
**Estimated effort**: 1 hour

Rewrite `message.rs` with message types (handlers in actor files):

```rust
//! Message types for actor communication
//!
//! Message handlers are implemented in their respective actor modules.

use chrono::{DateTime, Utc};

use crate::config::{FleetUpdateConfig, HostConfig};
use crate::state::HostState;

// ============================================================================
// HostActor Messages
// ============================================================================

/// Query host inventory via osquery
#[derive(Debug)]
pub struct QueryInventory;

/// Inventory query result
#[derive(Debug, Clone)]
pub struct InventoryResult {
    /// Number of packages with pending updates
    pub pending_updates: u32,
    /// Package names with updates available
    pub packages: Vec<String>,
}

/// Start package update process
#[derive(Debug)]
pub struct StartUpdate {
    /// If true, only simulate the update
    pub dry_run: bool,
}

/// Update operation result
#[derive(Debug, Clone)]
pub struct UpdateResult {
    /// Whether the update succeeded
    pub success: bool,
    /// Number of packages upgraded
    pub upgraded_count: u32,
    /// Whether a reboot is required
    pub reboot_required: bool,
}

/// Trigger reboot if kernel/services require it
#[derive(Debug)]
pub struct RebootIfRequired;

/// Perform health check after operations
#[derive(Debug)]
pub struct HealthCheck;

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Whether the host is healthy
    pub healthy: bool,
    /// Optional message with details
    pub message: Option<String>,
}

/// Retry failed operation (transitions Failed -> Idle)
#[derive(Debug)]
pub struct Retry;

/// Acknowledge failure (clears alert, allows inspection)
#[derive(Debug)]
pub struct Acknowledge;

/// Get current host state
#[derive(Debug)]
pub struct GetState;

/// Get full host status
#[derive(Debug)]
pub struct GetStatus;

// ============================================================================
// OrchestratorActor Messages
// ============================================================================

/// Register a new host with the orchestrator
#[derive(Debug)]
pub struct RegisterHost {
    /// Host configuration
    pub config: HostConfig,
}

/// Unregister a host from the orchestrator
#[derive(Debug)]
pub struct UnregisterHost {
    /// Hostname to remove
    pub hostname: String,
}

/// Get status of a specific host
#[derive(Debug)]
pub struct GetHostStatus {
    /// Hostname to query
    pub hostname: String,
}

/// List all managed hosts
#[derive(Debug)]
pub struct ListHosts;

/// Host status response
#[derive(Debug, Clone)]
pub struct HostStatus {
    /// Host name
    pub name: String,
    /// Current state
    pub state: HostState,
    /// Last successful update timestamp
    pub last_updated: Option<DateTime<Utc>>,
    /// Number of pending updates (if known)
    pub pending_updates: Option<u32>,
    /// Error message if in failed state
    pub error: Option<String>,
    /// Tags assigned to host
    pub tags: Vec<String>,
}

/// Trigger fleet-wide update
#[derive(Debug)]
pub struct TriggerFleetUpdate {
    /// Update configuration
    pub config: FleetUpdateConfig,
}

/// Fleet update progress
#[derive(Debug, Clone)]
pub struct FleetUpdateProgress {
    /// Total hosts in update batch
    pub total_hosts: usize,
    /// Hosts that completed successfully
    pub completed: usize,
    /// Hosts that failed
    pub failed: usize,
    /// Hosts currently updating
    pub in_progress: usize,
}

/// Query inventory for a specific host
#[derive(Debug)]
pub struct QueryHostInventory {
    /// Hostname to query
    pub hostname: String,
}

/// Trigger update for a specific host
#[derive(Debug)]
pub struct TriggerHostUpdate {
    /// Hostname to update
    pub hostname: String,
    /// Whether to perform a dry run
    pub dry_run: bool,
}

/// Retry a failed host
#[derive(Debug)]
pub struct RetryHost {
    /// Hostname to retry
    pub hostname: String,
}

/// Acknowledge a failed host
#[derive(Debug)]
pub struct AcknowledgeHost {
    /// Hostname to acknowledge
    pub hostname: String,
}
```

**Acceptance criteria**:
- [ ] All messages from GOALS.md defined
- [ ] Result types for each query message
- [ ] Debug derives for logging
- [ ] Documentation on all types

---

## Phase 4: HostActor Implementation

### Task 4.1: HostActor Structure and Lifecycle (`actor/host.rs`)
**Priority**: High  
**Estimated effort**: 2 hours

```rust
//! `HostActor`: Per-host orchestration
//!
//! Manages state machine for a single host and handles updates.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::prelude::*;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use tendhost_api::events::WsEvent;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_pkg::traits::PackageManager;

use crate::config::HostConfig;
use crate::error::CoreError;
use crate::state::{FailedStateContext, HostState, PendingUpdatesContext};

/// Arguments for spawning a `HostActor`
pub struct HostActorArgs {
    /// Host configuration
    pub config: HostConfig,
    /// Remote executor (SSH or local)
    pub executor: Arc<dyn RemoteExecutor>,
    /// Package manager implementation
    pub package_manager: Arc<dyn PackageManager>,
    /// Event broadcast sender for WebSocket
    pub event_tx: broadcast::Sender<WsEvent>,
}

/// Per-host actor managing state machine and operations
pub struct HostActor {
    /// Host configuration
    config: HostConfig,
    /// Current state
    state: HostState,
    /// Context for PendingUpdates state
    pending_context: Option<PendingUpdatesContext>,
    /// Context for Failed state
    failed_context: Option<FailedStateContext>,
    /// Remote executor (SSH or local)
    executor: Arc<dyn RemoteExecutor>,
    /// Package manager implementation
    package_manager: Arc<dyn PackageManager>,
    /// Event broadcast sender
    event_tx: broadcast::Sender<WsEvent>,
    /// Last successful update timestamp
    last_updated: Option<DateTime<Utc>>,
}

impl HostActor {
    /// Get the hostname
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get current state
    pub fn state(&self) -> HostState {
        self.state
    }

    /// Transition to a new state with validation and event emission
    fn transition_to(&mut self, new_state: HostState) -> Result<(), CoreError> {
        if !self.state.can_transition_to(new_state) {
            return Err(CoreError::InvalidTransition {
                from: self.state,
                to: new_state,
            });
        }

        let old_state = self.state;
        self.state = new_state;

        info!(
            host = %self.config.name,
            from = %old_state,
            to = %new_state,
            "state transition"
        );

        // Emit WebSocket event
        let event = WsEvent::HostStateChanged {
            host: self.config.name.clone(),
            from: old_state.to_string(),
            to: new_state.to_string(),
        };
        // Ignore send errors (no subscribers is fine)
        let _ = self.event_tx.send(event);

        Ok(())
    }

    /// Transition to Failed state, preserving error context
    fn fail_with_error(&mut self, error: impl Into<String>) {
        let previous = self.state;
        let error_msg = error.into();
        let context = FailedStateContext::new(previous, error_msg.clone());
        self.failed_context = Some(context);
        self.state = HostState::Failed;

        error!(
            host = %self.config.name,
            previous_state = %previous,
            error = %error_msg,
            "host entered failed state"
        );

        let event = WsEvent::HostStateChanged {
            host: self.config.name.clone(),
            from: previous.to_string(),
            to: "failed".to_string(),
        };
        let _ = self.event_tx.send(event);
    }
}

impl Actor for HostActor {
    type Args = HostActorArgs;
    type Error = CoreError;

    async fn on_start(
        args: Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        info!(host = %args.config.name, id = %actor_ref.id(), "HostActor starting");

        let event = WsEvent::HostConnected {
            host: args.config.name.clone(),
        };
        let _ = args.event_tx.send(event);

        Ok(Self {
            config: args.config,
            state: HostState::Idle,
            pending_context: None,
            failed_context: None,
            executor: args.executor,
            package_manager: args.package_manager,
            event_tx: args.event_tx,
            last_updated: None,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        info!(
            host = %self.config.name,
            reason = ?reason,
            "HostActor stopping"
        );

        let event = WsEvent::HostDisconnected {
            host: self.config.name.clone(),
            reason: format!("{reason:?}"),
        };
        let _ = self.event_tx.send(event);

        Ok(())
    }
}
```

---

### Task 4.2: HostActor Message Handlers
**Priority**: High  
**Estimated effort**: 2.5 hours

Add message handler implementations in `actor/host.rs`:

```rust
use kameo::message::{Context, Message};

use crate::message::{
    Acknowledge, GetState, GetStatus, HealthCheck, HealthCheckResult, HostStatus,
    InventoryResult, QueryInventory, RebootIfRequired, Retry, StartUpdate, UpdateResult,
};

impl Message<QueryInventory> for HostActor {
    type Reply = Result<InventoryResult, CoreError>;

    async fn handle(
        &mut self,
        _msg: QueryInventory,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Validate state
        if self.state.is_busy() {
            return Err(CoreError::InvalidTransition {
                from: self.state,
                to: HostState::Querying,
            });
        }

        self.transition_to(HostState::Querying)?;

        // Query upgradable packages
        match self.package_manager.list_upgradable().await {
            Ok(packages) => {
                let count = packages.len() as u32;
                let names: Vec<String> = packages.into_iter().map(|p| p.name).collect();

                if count > 0 {
                    self.pending_context = Some(PendingUpdatesContext {
                        package_count: count,
                        packages: names.clone(),
                        queried_at: Utc::now(),
                    });
                    self.transition_to(HostState::PendingUpdates)?;
                } else {
                    self.transition_to(HostState::Idle)?;
                }

                Ok(InventoryResult {
                    pending_updates: count,
                    packages: names,
                })
            }
            Err(e) => {
                self.fail_with_error(&e);
                Err(CoreError::InventoryError(e))
            }
        }
    }
}

impl Message<StartUpdate> for HostActor {
    type Reply = Result<UpdateResult, CoreError>;

    async fn handle(
        &mut self,
        msg: StartUpdate,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Must be in PendingUpdates or Idle to start update
        if !self.state.can_start_operation() {
            return Err(CoreError::InvalidTransition {
                from: self.state,
                to: HostState::Updating,
            });
        }

        self.transition_to(HostState::Updating)?;

        let result = if msg.dry_run {
            self.package_manager.upgrade_dry_run().await
        } else {
            self.package_manager.upgrade_all().await
        };

        match result {
            Ok(pkg_result) => {
                // Check if reboot is required
                let reboot_required = self
                    .package_manager
                    .reboot_required()
                    .await
                    .unwrap_or(false);

                if reboot_required && !msg.dry_run {
                    self.transition_to(HostState::WaitingReboot)?;
                } else {
                    self.last_updated = Some(Utc::now());
                    self.pending_context = None;
                    self.transition_to(HostState::Idle)?;
                }

                // Emit completion event
                let event = WsEvent::UpdateCompleted {
                    host: self.config.name.clone(),
                    result: format!(
                        "upgraded {} packages, reboot_required={}",
                        pkg_result.upgraded_count, reboot_required
                    ),
                };
                let _ = self.event_tx.send(event);

                Ok(UpdateResult {
                    success: pkg_result.success,
                    upgraded_count: pkg_result.upgraded_count,
                    reboot_required,
                })
            }
            Err(e) => {
                self.fail_with_error(&e);
                Err(CoreError::PackageError(e))
            }
        }
    }
}

impl Message<RebootIfRequired> for HostActor {
    type Reply = Result<bool, CoreError>;

    async fn handle(
        &mut self,
        _msg: RebootIfRequired,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.state != HostState::WaitingReboot {
            return Err(CoreError::InvalidTransition {
                from: self.state,
                to: HostState::Rebooting,
            });
        }

        // Check policy
        if !self.config.policy.auto_reboot {
            warn!(
                host = %self.config.name,
                "auto_reboot disabled, staying in WaitingReboot"
            );
            return Ok(false);
        }

        self.transition_to(HostState::Rebooting)?;

        // Execute reboot command
        match self.executor.run("sudo reboot").await {
            Ok(_) => {
                // After reboot, we need to verify
                // In practice, we'd wait for SSH to come back
                self.transition_to(HostState::Verifying)?;
                Ok(true)
            }
            Err(e) => {
                self.fail_with_error(&e);
                Err(CoreError::SshError(e))
            }
        }
    }
}

impl Message<HealthCheck> for HostActor {
    type Reply = Result<HealthCheckResult, CoreError>;

    async fn handle(
        &mut self,
        _msg: HealthCheck,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Health check can be done from Verifying state or any non-busy state
        let is_verifying = self.state == HostState::Verifying;

        // Simple health check: can we run a command?
        match self.executor.run("echo ok").await {
            Ok(output) => {
                let healthy = output.trim() == "ok";

                if is_verifying {
                    if healthy {
                        self.last_updated = Some(Utc::now());
                        self.pending_context = None;
                        self.transition_to(HostState::Idle)?;
                    } else {
                        self.fail_with_error("health check failed after reboot");
                    }
                }

                Ok(HealthCheckResult {
                    healthy,
                    message: if healthy {
                        None
                    } else {
                        Some("unexpected output".to_string())
                    },
                })
            }
            Err(e) => {
                if is_verifying {
                    self.fail_with_error(&e);
                }
                Err(CoreError::SshError(e))
            }
        }
    }
}

impl Message<Retry> for HostActor {
    type Reply = Result<(), CoreError>;

    async fn handle(
        &mut self,
        _msg: Retry,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.state != HostState::Failed {
            return Err(CoreError::InvalidTransition {
                from: self.state,
                to: HostState::Idle,
            });
        }

        if let Some(ref mut ctx) = self.failed_context {
            ctx.increment_retry();
        }

        self.transition_to(HostState::Idle)?;
        self.failed_context = None;

        info!(host = %self.config.name, "host recovered from failed state");

        Ok(())
    }
}

impl Message<Acknowledge> for HostActor {
    type Reply = Result<(), CoreError>;

    async fn handle(
        &mut self,
        _msg: Acknowledge,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.state != HostState::Failed {
            return Err(CoreError::HostFailed(
                "can only acknowledge hosts in Failed state".to_string(),
            ));
        }

        if let Some(ref mut ctx) = self.failed_context {
            ctx.acknowledge();
            info!(
                host = %self.config.name,
                error = %ctx.error,
                "failure acknowledged"
            );
        }

        Ok(())
    }
}

impl Message<GetState> for HostActor {
    type Reply = HostState;

    async fn handle(
        &mut self,
        _msg: GetState,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state
    }
}

impl Message<GetStatus> for HostActor {
    type Reply = HostStatus;

    async fn handle(
        &mut self,
        _msg: GetStatus,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        HostStatus {
            name: self.config.name.clone(),
            state: self.state,
            last_updated: self.last_updated,
            pending_updates: self.pending_context.as_ref().map(|c| c.package_count),
            error: self.failed_context.as_ref().map(|c| c.error.clone()),
            tags: self.config.tags.clone(),
        }
    }
}
```

**Acceptance criteria**:
- [ ] All message handlers implemented
- [ ] State validation before operations
- [ ] Proper error recovery to Failed state
- [ ] Event emission for progress/completion
- [ ] Unit tests for each handler

---

## Phase 5: OrchestratorActor Implementation

### Task 5.1: OrchestratorActor Structure (`actor/orchestrator.rs`)
**Priority**: High  
**Estimated effort**: 1.5 hours

```rust
//! `OrchestratorActor`: Fleet-wide orchestration
//!
//! Manages registry of `HostActors` and coordinates fleet-wide commands.

use std::collections::HashMap;
use std::sync::Arc;

use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::prelude::*;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use tendhost_api::events::WsEvent;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_pkg::traits::PackageManager;

use crate::actor::host::{HostActor, HostActorArgs};
use crate::config::HostConfig;
use crate::error::CoreError;

/// Factory trait for creating `HostActor` dependencies
///
/// Allows injection of different executors/package managers per host.
#[async_trait::async_trait]
pub trait HostActorFactory: Send + Sync {
    /// Create a remote executor for the given host config
    async fn create_executor(&self, config: &HostConfig) -> Arc<dyn RemoteExecutor>;

    /// Create a package manager for the given host config and executor
    async fn create_package_manager(
        &self,
        config: &HostConfig,
        executor: Arc<dyn RemoteExecutor>,
    ) -> Arc<dyn PackageManager>;
}

/// Arguments for spawning an `OrchestratorActor`
pub struct OrchestratorActorArgs {
    /// Event broadcast channel capacity
    pub event_channel_capacity: usize,
    /// Factory for creating host dependencies
    pub host_factory: Arc<dyn HostActorFactory>,
}

impl Default for OrchestratorActorArgs {
    fn default() -> Self {
        Self {
            event_channel_capacity: 1024,
            host_factory: Arc::new(NoOpHostFactory),
        }
    }
}

/// No-op factory for testing
struct NoOpHostFactory;

#[async_trait::async_trait]
impl HostActorFactory for NoOpHostFactory {
    async fn create_executor(&self, _config: &HostConfig) -> Arc<dyn RemoteExecutor> {
        panic!("NoOpHostFactory should not be used in production")
    }

    async fn create_package_manager(
        &self,
        _config: &HostConfig,
        _executor: Arc<dyn RemoteExecutor>,
    ) -> Arc<dyn PackageManager> {
        panic!("NoOpHostFactory should not be used in production")
    }
}

/// Fleet orchestrator managing all host actors
pub struct OrchestratorActor {
    /// Registry of host actors by hostname
    hosts: HashMap<String, ActorRef<HostActor>>,
    /// Host configurations
    configs: HashMap<String, HostConfig>,
    /// Event broadcast sender
    event_tx: broadcast::Sender<WsEvent>,
    /// Factory for creating host dependencies
    host_factory: Arc<dyn HostActorFactory>,
}

impl OrchestratorActor {
    /// Get event receiver for WebSocket connections
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.event_tx.subscribe()
    }

    /// Get number of managed hosts
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    /// Spawn a `HostActor` for the given config
    async fn spawn_host_actor(
        &mut self,
        config: HostConfig,
    ) -> Result<ActorRef<HostActor>, CoreError> {
        let executor = self.host_factory.create_executor(&config).await;
        let package_manager = self
            .host_factory
            .create_package_manager(&config, executor.clone())
            .await;

        let args = HostActorArgs {
            config: config.clone(),
            executor,
            package_manager,
            event_tx: self.event_tx.clone(),
        };

        let actor_ref = HostActor::spawn(args);

        info!(host = %config.name, "spawned HostActor");

        Ok(actor_ref)
    }
}

impl Actor for OrchestratorActor {
    type Args = OrchestratorActorArgs;
    type Error = CoreError;

    async fn on_start(
        args: Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let (event_tx, _) = broadcast::channel(args.event_channel_capacity);

        info!(id = %actor_ref.id(), "OrchestratorActor starting");

        Ok(Self {
            hosts: HashMap::new(),
            configs: HashMap::new(),
            event_tx,
            host_factory: args.host_factory,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        info!(reason = ?reason, "OrchestratorActor stopping");

        // Stop all host actors
        for (name, actor_ref) in &self.hosts {
            info!(host = %name, "stopping HostActor");
            actor_ref.stop_gracefully().await.ok();
        }

        Ok(())
    }
}
```

---

### Task 5.2: OrchestratorActor Message Handlers
**Priority**: High  
**Estimated effort**: 2 hours

```rust
use kameo::message::{Context, Message};

use crate::message::{
    AcknowledgeHost, FleetUpdateProgress, GetHostStatus, HostStatus, ListHosts,
    QueryHostInventory, QueryInventory, RegisterHost, RetryHost, Retry,
    StartUpdate, TriggerFleetUpdate, TriggerHostUpdate, UnregisterHost,
};

impl Message<RegisterHost> for OrchestratorActor {
    type Reply = Result<(), CoreError>;

    async fn handle(
        &mut self,
        msg: RegisterHost,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let name = msg.config.name.clone();

        if self.hosts.contains_key(&name) {
            return Err(CoreError::HostAlreadyExists(name));
        }

        let actor_ref = self.spawn_host_actor(msg.config.clone()).await?;
        self.hosts.insert(name.clone(), actor_ref);
        self.configs.insert(name, msg.config);

        Ok(())
    }
}

impl Message<UnregisterHost> for OrchestratorActor {
    type Reply = Result<(), CoreError>;

    async fn handle(
        &mut self,
        msg: UnregisterHost,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let name = &msg.hostname;

        if let Some(actor_ref) = self.hosts.remove(name) {
            self.configs.remove(name);
            actor_ref.stop_gracefully().await.ok();
            info!(host = %name, "unregistered host");
            Ok(())
        } else {
            Err(CoreError::HostNotFound(name.clone()))
        }
    }
}

impl Message<GetHostStatus> for OrchestratorActor {
    type Reply = Result<HostStatus, CoreError>;

    async fn handle(
        &mut self,
        msg: GetHostStatus,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let actor_ref = self
            .hosts
            .get(&msg.hostname)
            .ok_or_else(|| CoreError::HostNotFound(msg.hostname.clone()))?;

        actor_ref
            .ask(crate::message::GetStatus)
            .await
            .map_err(|e| CoreError::ActorError(e.to_string()))
    }
}

impl Message<ListHosts> for OrchestratorActor {
    type Reply = Vec<HostStatus>;

    async fn handle(
        &mut self,
        _msg: ListHosts,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut statuses = Vec::with_capacity(self.hosts.len());

        for (name, actor_ref) in &self.hosts {
            match actor_ref.ask(crate::message::GetStatus).await {
                Ok(status) => statuses.push(status),
                Err(e) => {
                    warn!(host = %name, error = %e, "failed to get host status");
                }
            }
        }

        statuses
    }
}

impl Message<QueryHostInventory> for OrchestratorActor {
    type Reply = Result<crate::message::InventoryResult, CoreError>;

    async fn handle(
        &mut self,
        msg: QueryHostInventory,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let actor_ref = self
            .hosts
            .get(&msg.hostname)
            .ok_or_else(|| CoreError::HostNotFound(msg.hostname.clone()))?;

        actor_ref
            .ask(QueryInventory)
            .await
            .map_err(|e| CoreError::ActorError(e.to_string()))?
    }
}

impl Message<TriggerHostUpdate> for OrchestratorActor {
    type Reply = Result<crate::message::UpdateResult, CoreError>;

    async fn handle(
        &mut self,
        msg: TriggerHostUpdate,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let actor_ref = self
            .hosts
            .get(&msg.hostname)
            .ok_or_else(|| CoreError::HostNotFound(msg.hostname.clone()))?;

        actor_ref
            .ask(StartUpdate { dry_run: msg.dry_run })
            .await
            .map_err(|e| CoreError::ActorError(e.to_string()))?
    }
}

impl Message<RetryHost> for OrchestratorActor {
    type Reply = Result<(), CoreError>;

    async fn handle(
        &mut self,
        msg: RetryHost,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let actor_ref = self
            .hosts
            .get(&msg.hostname)
            .ok_or_else(|| CoreError::HostNotFound(msg.hostname.clone()))?;

        actor_ref
            .ask(Retry)
            .await
            .map_err(|e| CoreError::ActorError(e.to_string()))?
    }
}

impl Message<AcknowledgeHost> for OrchestratorActor {
    type Reply = Result<(), CoreError>;

    async fn handle(
        &mut self,
        msg: AcknowledgeHost,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let actor_ref = self
            .hosts
            .get(&msg.hostname)
            .ok_or_else(|| CoreError::HostNotFound(msg.hostname.clone()))?;

        actor_ref
            .ask(crate::message::Acknowledge)
            .await
            .map_err(|e| CoreError::ActorError(e.to_string()))?
    }
}

impl Message<TriggerFleetUpdate> for OrchestratorActor {
    type Reply = Result<FleetUpdateProgress, CoreError>;

    async fn handle(
        &mut self,
        msg: TriggerFleetUpdate,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let config = msg.config;

        // Filter hosts based on config
        let hosts_to_update: Vec<_> = self
            .hosts
            .iter()
            .filter(|(name, _)| {
                if let Some(ref filter) = config.filter {
                    // Check exclusions
                    if filter.exclude_hosts.contains(name) {
                        return false;
                    }
                    // Check tags (if specified, host must have at least one)
                    if !filter.tags.is_empty() {
                        if let Some(hc) = self.configs.get(*name) {
                            if !filter.tags.iter().any(|t| hc.tags.contains(t)) {
                                return false;
                            }
                        }
                    }
                }
                true
            })
            .map(|(name, actor)| (name.clone(), actor.clone()))
            .collect();

        let total = hosts_to_update.len();
        let mut completed = 0;
        let mut failed = 0;

        info!(
            total_hosts = total,
            batch_size = config.batch_size,
            "starting fleet update"
        );

        // Process in batches
        for batch in hosts_to_update.chunks(config.batch_size) {
            let mut handles = Vec::new();

            for (name, actor_ref) in batch {
                let actor = actor_ref.clone();
                let host_name = name.clone();
                let dry_run = config.dry_run;

                let handle = tokio::spawn(async move {
                    // First query inventory, then update
                    let _ = actor.ask(QueryInventory).await;
                    actor.ask(StartUpdate { dry_run }).await
                });

                handles.push((host_name, handle));
            }

            // Wait for batch to complete
            for (name, handle) in handles {
                match handle.await {
                    Ok(Ok(_)) => {
                        completed += 1;
                        info!(host = %name, "update completed");
                    }
                    Ok(Err(e)) => {
                        failed += 1;
                        error!(host = %name, error = %e, "update failed");
                    }
                    Err(e) => {
                        failed += 1;
                        error!(host = %name, error = %e, "task panicked");
                    }
                }
            }

            // Delay between batches (skip for last batch)
            if !config.delay_between_batches.is_zero() && completed + failed < total {
                tokio::time::sleep(config.delay_between_batches).await;
            }
        }

        info!(
            total = total,
            completed = completed,
            failed = failed,
            "fleet update finished"
        );

        Ok(FleetUpdateProgress {
            total_hosts: total,
            completed,
            failed,
            in_progress: 0,
        })
    }
}
```

**Acceptance criteria**:
- [ ] Host registration/unregistration
- [ ] Fleet update with batching
- [ ] Filter support (tags, groups, exclusions)
- [ ] Progress tracking
- [ ] Error handling per host

---

## Phase 6: Integration & Testing

### Task 6.1: Update lib.rs Exports
**Priority**: Medium  
**Estimated effort**: 15 min

```rust
//! tendhost-core: Actor framework and orchestration logic
//!
//! Implements the `OrchestratorActor` and `HostActor` using kameo framework.
//! Contains message types, state machines, and fleet logic.

pub mod actor;
pub mod config;
pub mod error;
pub mod message;
pub mod state;

pub use actor::host::{HostActor, HostActorArgs};
pub use actor::orchestrator::{HostActorFactory, OrchestratorActor, OrchestratorActorArgs};
pub use config::{FleetFilter, FleetUpdateConfig, HostConfig, HostPolicy, MaintenanceWindow};
pub use error::CoreError;
pub use message::{
    Acknowledge, FleetUpdateProgress, GetHostStatus, GetState, GetStatus, HealthCheck,
    HealthCheckResult, HostStatus, InventoryResult, ListHosts, QueryHostInventory,
    QueryInventory, RebootIfRequired, RegisterHost, Retry, RetryHost, StartUpdate,
    TriggerFleetUpdate, TriggerHostUpdate, UnregisterHost, UpdateResult,
};
pub use state::{FailedStateContext, HostState, PendingUpdatesContext};
```

---

### Task 6.2: Update actor/mod.rs
**Priority**: Medium  
**Estimated effort**: 5 min

```rust
//! Actor implementations

pub mod host;
pub mod orchestrator;

pub use host::{HostActor, HostActorArgs};
pub use orchestrator::{HostActorFactory, OrchestratorActor, OrchestratorActorArgs};
```

---

### Task 6.3: Integration Tests with Mocks
**Priority**: Medium  
**Estimated effort**: 2 hours

Create `crates/tendhost-core/tests/actor_integration.rs`:

```rust
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::broadcast;

use tendhost_core::*;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_pkg::traits::{PackageManager, UpdateResult as PkgUpdateResult, UpgradablePackage};

// Mock implementations
struct MockExecutor;

#[async_trait]
impl RemoteExecutor for MockExecutor {
    async fn run(&self, _cmd: &str) -> Result<String, String> {
        Ok("ok".to_string())
    }

    async fn run_with_timeout(
        &self,
        cmd: &str,
        _timeout: Duration,
    ) -> Result<String, String> {
        self.run(cmd).await
    }
}

struct MockPackageManager {
    packages: Vec<String>,
    reboot_required: bool,
}

#[async_trait]
impl PackageManager for MockPackageManager {
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, String> {
        Ok(self
            .packages
            .iter()
            .map(|name| UpgradablePackage {
                name: name.clone(),
                version: "1.0.0".to_string(),
            })
            .collect())
    }

    async fn upgrade_all(&self) -> Result<PkgUpdateResult, String> {
        Ok(PkgUpdateResult {
            success: true,
            upgraded_count: self.packages.len() as u32,
        })
    }

    async fn upgrade_dry_run(&self) -> Result<PkgUpdateResult, String> {
        self.upgrade_all().await
    }

    async fn reboot_required(&self) -> Result<bool, String> {
        Ok(self.reboot_required)
    }
}

struct TestHostFactory;

#[async_trait]
impl HostActorFactory for TestHostFactory {
    async fn create_executor(&self, _config: &HostConfig) -> Arc<dyn RemoteExecutor> {
        Arc::new(MockExecutor)
    }

    async fn create_package_manager(
        &self,
        _config: &HostConfig,
        _executor: Arc<dyn RemoteExecutor>,
    ) -> Arc<dyn PackageManager> {
        Arc::new(MockPackageManager {
            packages: vec!["vim".to_string(), "curl".to_string()],
            reboot_required: false,
        })
    }
}

#[tokio::test]
async fn test_host_actor_query_inventory() {
    let (tx, _rx) = broadcast::channel(100);

    let config = HostConfig {
        name: "test-host".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        ssh_key: None,
        compose_paths: vec![],
        tags: vec![],
        policy: HostPolicy::default(),
    };

    let args = HostActorArgs {
        config,
        executor: Arc::new(MockExecutor),
        package_manager: Arc::new(MockPackageManager {
            packages: vec!["vim".to_string(), "curl".to_string()],
            reboot_required: false,
        }),
        event_tx: tx,
    };

    let actor_ref = HostActor::spawn(args);

    let result = actor_ref.ask(QueryInventory).await.unwrap();

    assert!(result.is_ok());
    let inventory = result.unwrap();
    assert_eq!(inventory.pending_updates, 2);
    assert_eq!(inventory.packages, vec!["vim", "curl"]);

    actor_ref.stop_gracefully().await.unwrap();
}

#[tokio::test]
async fn test_orchestrator_register_host() {
    let args = OrchestratorActorArgs {
        event_channel_capacity: 100,
        host_factory: Arc::new(TestHostFactory),
    };

    let orchestrator = OrchestratorActor::spawn(args);

    let config = HostConfig {
        name: "test-host".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        ssh_key: None,
        compose_paths: vec![],
        tags: vec!["test".to_string()],
        policy: HostPolicy::default(),
    };

    let result = orchestrator.ask(RegisterHost { config }).await.unwrap();
    assert!(result.is_ok());

    let hosts = orchestrator.ask(ListHosts).await.unwrap();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].name, "test-host");

    orchestrator.stop_gracefully().await.unwrap();
}
```

---

## Summary

### File Changes Required

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add kameo_actors, kameo_macros, tendhost-exec, tendhost-pkg |
| `src/error.rs` | Create | Error types |
| `src/config.rs` | Create | Configuration types |
| `src/state.rs` | Modify | Add transitions, serde, display, tests |
| `src/message.rs` | Modify | Message definitions |
| `src/actor/mod.rs` | Modify | Re-exports |
| `src/actor/host.rs` | Modify | Full HostActor implementation |
| `src/actor/orchestrator.rs` | Modify | Full OrchestratorActor |
| `src/lib.rs` | Modify | Re-exports |
| `tests/actor_integration.rs` | Create | Integration tests |

### Estimated Total Effort

| Phase | Effort |
|-------|--------|
| Phase 1: Foundation | 1.25 hours |
| Phase 2: State Machine | 1 hour |
| Phase 3: Messages | 1 hour |
| Phase 4: HostActor | 4.5 hours |
| Phase 5: OrchestratorActor | 3.5 hours |
| Phase 6: Integration | 2.5 hours |
| **Total** | **~14 hours** |

### Priority Order

1. error.rs (blocking)
2. config.rs (blocking)
3. state.rs enhancements
4. message.rs with types
5. HostActor structure + handlers
6. OrchestratorActor structure + handlers
7. Tests

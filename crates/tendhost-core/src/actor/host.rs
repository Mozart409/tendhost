//! `HostActor`: Per-host orchestration
//!
//! Manages state machine for a single host and handles updates.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::message::{Context, Message};
use kameo::prelude::*;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use tendhost_api::events::WsEvent;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_pkg::traits::PackageManager;

use crate::config::HostConfig;
use crate::error::CoreError;
use crate::message::{
    Acknowledge, GetState, GetStatus, HealthCheck, HealthCheckResult, HostStatus, InventoryResult,
    QueryInventory, RebootIfRequired, Retry, StartUpdate, UpdateResult,
};
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
    /// Context for `PendingUpdates` state
    pending_context: Option<PendingUpdatesContext>,
    /// Context for `Failed` state
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
    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get current state
    #[must_use]
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

    /// Transition to `Failed` state, preserving error context
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

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
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

// ============================================================================
// Message Handlers
// ============================================================================

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
                #[allow(clippy::cast_possible_truncation)]
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

    async fn handle(&mut self, _msg: Retry, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
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

//! `OrchestratorActor`: Fleet-wide orchestration
//!
//! Manages registry of `HostActors` and coordinates fleet-wide commands.

use std::collections::HashMap;
use std::sync::Arc;

use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::message::{Context, Message};
use kameo::prelude::*;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use tendhost_api::events::WsEvent;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_pkg::traits::PackageManager;

use crate::actor::host::{HostActor, HostActorArgs};
use crate::config::HostConfig;
use crate::error::CoreError;
use crate::message::{
    Acknowledge, AcknowledgeHost, FleetUpdateProgress, GetHostStatus, HostStatus, InventoryResult,
    ListHosts, QueryHostInventory, QueryInventory, RegisterHost, Retry, RetryHost, StartUpdate,
    TriggerFleetUpdate, TriggerHostUpdate, UnregisterHost,
};

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
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.event_tx.subscribe()
    }

    /// Get number of managed hosts
    #[must_use]
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

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
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

// ============================================================================
// Message Handlers
// ============================================================================

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
    type Reply = Result<InventoryResult, CoreError>;

    async fn handle(
        &mut self,
        msg: QueryHostInventory,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let actor_ref = self
            .hosts
            .get(&msg.hostname)
            .ok_or_else(|| CoreError::HostNotFound(msg.hostname.clone()))?;

        match actor_ref.ask(QueryInventory).await {
            Ok(inner_result) => Ok(inner_result),
            Err(e) => Err(CoreError::ActorError(e.to_string())),
        }
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

        match actor_ref
            .ask(StartUpdate {
                dry_run: msg.dry_run,
            })
            .await
        {
            Ok(inner_result) => Ok(inner_result),
            Err(e) => Err(CoreError::ActorError(e.to_string())),
        }
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

        match actor_ref.ask(Retry).await {
            Ok(inner_result) => Ok(inner_result),
            Err(e) => Err(CoreError::ActorError(e.to_string())),
        }
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

        match actor_ref.ask(Acknowledge).await {
            Ok(inner_result) => Ok(inner_result),
            Err(e) => Err(CoreError::ActorError(e.to_string())),
        }
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
                    if !filter.tags.is_empty()
                        && let Some(hc) = self.configs.get(*name)
                        && !filter.tags.iter().any(|t| hc.tags.contains(t))
                    {
                        return false;
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

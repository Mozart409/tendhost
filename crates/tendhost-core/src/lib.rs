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
    Acknowledge, AcknowledgeHost, FleetUpdateProgress, GetHostStatus, GetState, GetStatus,
    HealthCheck, HealthCheckResult, HostStatus, InventoryResult, ListHosts, QueryHostInventory,
    QueryInventory, RebootIfRequired, RegisterHost, Retry, RetryHost, StartUpdate,
    TriggerFleetUpdate, TriggerHostUpdate, UnregisterHost, UpdateResult,
};
pub use state::{FailedStateContext, HostState, PendingUpdatesContext};

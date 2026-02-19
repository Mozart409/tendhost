//! Message types for actor communication

/// Inventory query message
pub struct QueryInventory;

/// Start update message
pub struct StartUpdate {
    pub dry_run: bool,
}

/// Reboot if required message
pub struct RebootIfRequired;

/// Health check message
pub struct HealthCheck;

/// Register host with orchestrator
pub struct RegisterHost {
    pub hostname: String,
    // config: HostConfig
}

/// Fleet-wide update trigger
pub struct TriggerFleetUpdate {
    pub batch_size: usize,
    // delay_between_batches: Duration
}

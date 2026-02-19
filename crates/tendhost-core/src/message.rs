//! Message types for actor communication
//!
//! Message handlers are implemented in their respective actor modules.

use chrono::{DateTime, Utc};
use kameo_macros::Reply;

use crate::config::{FleetUpdateConfig, HostConfig};
use crate::state::HostState;

// ============================================================================
// HostActor Messages
// ============================================================================

/// Query host inventory via osquery
#[derive(Debug)]
pub struct QueryInventory;

/// Inventory query result
#[derive(Debug, Clone, Reply)]
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
#[derive(Debug, Clone, Reply)]
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
#[derive(Debug, Clone, Reply)]
pub struct HealthCheckResult {
    /// Whether the host is healthy
    pub healthy: bool,
    /// Optional message with details
    pub message: Option<String>,
}

/// Retry failed operation (transitions `Failed` -> `Idle`)
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
#[derive(Debug, Clone, Reply)]
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
#[derive(Debug, Clone, Reply)]
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

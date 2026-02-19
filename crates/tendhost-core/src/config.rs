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
    /// Start time in `HH:MM` format
    pub start: String,
    /// End time in `HH:MM` format
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

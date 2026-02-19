//! Host state machine types

use chrono::{DateTime, Utc};

/// States for a `HostActor` state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostState {
    Idle,
    Querying,
    PendingUpdates,
    Updating,
    WaitingReboot,
    Rebooting,
    Verifying,
    Failed,
}

/// Failed state details
#[derive(Debug, Clone)]
pub struct FailedState {
    pub previous_state: HostState,
    pub error: String,
    pub failed_at: DateTime<Utc>,
    pub retry_count: u32,
    pub acknowledged: bool,
}

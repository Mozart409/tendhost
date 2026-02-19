//! Host state machine types

use std::fmt;

use chrono::{DateTime, Utc};
use kameo_macros::Reply;
use serde::{Deserialize, Serialize};

/// States for a `HostActor` state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reply, Default)]
#[serde(rename_all = "snake_case")]
pub enum HostState {
    /// Host is idle and ready for operations
    #[default]
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
    /// Validates against the state machine defined in `GOALS.md`.
    #[must_use]
    pub fn can_transition_to(&self, target: Self) -> bool {
        use HostState::{
            Failed, Idle, PendingUpdates, Querying, Rebooting, Updating, Verifying, WaitingReboot,
        };
        matches!(
            (self, target),
            // Normal flow
            (Idle, Querying)
                | (Querying | Updating | Rebooting | Verifying | Failed, Idle)
                | (Querying, PendingUpdates | Failed)
                | (PendingUpdates, Updating)
                | (Updating, WaitingReboot | Failed)
                | (WaitingReboot, Rebooting)
                | (Rebooting, Verifying | Failed)
                | (Verifying, Failed)
        )
    }

    /// Whether this state represents an active operation
    #[must_use]
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Querying | Self::Updating | Self::Rebooting | Self::Verifying
        )
    }

    /// Whether operations can be started from this state
    #[must_use]
    pub fn can_start_operation(&self) -> bool {
        matches!(self, Self::Idle | Self::PendingUpdates)
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
    #[must_use]
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
        use HostState::{
            Failed, Idle, PendingUpdates, Querying, Rebooting, Updating, Verifying, WaitingReboot,
        };

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
        use HostState::{Idle, PendingUpdates, Querying, Rebooting, Updating, Verifying};

        assert!(!Idle.can_transition_to(Updating)); // must query first
        assert!(!Querying.can_transition_to(Rebooting));
        assert!(!PendingUpdates.can_transition_to(Verifying));
        assert!(!Idle.can_transition_to(Idle)); // no self-transition
    }

    #[test]
    fn test_is_busy() {
        use HostState::{
            Failed, Idle, PendingUpdates, Querying, Rebooting, Updating, Verifying, WaitingReboot,
        };

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

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

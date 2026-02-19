//! Error types for tendhost-pkg

use thiserror::Error;

/// Errors that can occur during package operations
#[derive(Error, Debug, Clone)]
pub enum PackageError {
    /// Package manager not found on system
    #[error("package manager not found: {0}")]
    ManagerNotFound(String),

    /// Package not found in repositories
    #[error("package not found: {0}")]
    PackageNotFound(String),

    /// Repository is unavailable
    #[error("repository unavailable: {0}")]
    RepositoryUnavailable(String),

    /// Lock file conflict (another process running)
    #[error("lock file conflict: {0}")]
    LockConflict(String),

    /// Insufficient permissions (need sudo)
    #[error("insufficient permissions: {0}")]
    PermissionDenied(String),

    /// Command execution failed
    #[error("command failed: {status} - {message}")]
    CommandFailed {
        /// Exit status
        status: i32,
        /// Error message
        message: String,
    },

    /// Failed to parse command output
    #[error("parse error: {0}")]
    ParseError(String),

    /// Execution error from remote executor
    #[error("execution error: {0}")]
    ExecutionError(String),

    /// Docker compose not found
    #[error("docker compose not found")]
    DockerComposeNotFound,

    /// Compose file not found
    #[error("compose file not found: {0}")]
    ComposeFileNotFound(String),

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    ConfigError(String),
}

impl PackageError {
    /// Check if error is retryable
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            PackageError::LockConflict(_) | PackageError::RepositoryUnavailable(_)
        )
    }

    /// Check if error indicates need for sudo
    #[must_use]
    pub fn needs_sudo(&self) -> bool {
        matches!(self, PackageError::PermissionDenied(_))
    }
}

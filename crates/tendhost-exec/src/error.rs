//! Error types for tendhost-exec

use std::time::Duration;

use thiserror::Error;

/// Errors that can occur during remote execution
#[derive(Error, Debug, Clone)]
pub enum ExecError {
    /// Failed to connect to remote host
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Authentication failed
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Command execution failed
    #[error("command execution failed: {status} - {stderr}")]
    CommandFailed {
        /// Exit status code
        status: i32,
        /// Stderr output
        stderr: String,
    },

    /// Command timed out
    #[error("command timed out after {timeout:?}")]
    Timeout {
        /// Timeout duration that was exceeded
        timeout: Duration,
    },

    /// SSH key error
    #[error("SSH key error: {0}")]
    SshKeyError(String),

    /// Process spawn error
    #[error("failed to spawn process: {0}")]
    SpawnError(String),

    /// I/O error during execution
    #[error("I/O error: {0}")]
    IoError(String),

    /// Connection not established
    #[error("not connected")]
    NotConnected,

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    ConfigError(String),
}

impl ExecError {
    /// Check if error is retryable
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ExecError::ConnectionFailed(_) | ExecError::Timeout { .. }
        )
    }
}

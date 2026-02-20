//! Error types for tendhost-inventory

use thiserror::Error;

/// Errors that can occur during inventory operations
#[derive(Error, Debug, Clone)]
pub enum InventoryError {
    /// osquery is not installed on the target system
    #[error("osquery not found: {0}")]
    OsqueryNotFound(String),

    /// SQL query execution failed
    #[error("query execution failed: {0}")]
    QueryFailed(String),

    /// SQL syntax error
    #[error("SQL syntax error: {0}")]
    SqlSyntax(String),

    /// Failed to parse query results
    #[error("JSON parse error: {0}")]
    ParseError(String),

    /// Remote execution error
    #[error("execution error: {0}")]
    ExecutionError(String),

    /// Table not available on this system
    #[error("table not available: {0}")]
    TableNotAvailable(String),

    /// Query timeout
    #[error("query timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Cache error
    #[error("cache error: {0}")]
    CacheError(String),

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    ConfigError(String),
}

impl InventoryError {
    /// Check if error is retryable
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            InventoryError::ExecutionError(_) | InventoryError::Timeout(_)
        )
    }

    /// Check if osquery needs to be installed
    #[must_use]
    pub fn needs_installation(&self) -> bool {
        matches!(self, InventoryError::OsqueryNotFound(_))
    }
}

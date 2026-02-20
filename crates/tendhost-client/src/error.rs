//! Error types for the tendhost client

use thiserror::Error;

/// Errors that can occur when using the tendhost client
#[derive(Error, Debug)]
pub enum ClientError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// Request timeout
    #[error("Request timed out")]
    Timeout,

    /// API returned an error status
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message from server
        message: String,
    },

    /// Connection closed unexpectedly
    #[error("Connection closed: {0}")]
    ConnectionClosed(String),

    /// Invalid response format
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Result type for client operations
pub type Result<T> = std::result::Result<T, ClientError>;

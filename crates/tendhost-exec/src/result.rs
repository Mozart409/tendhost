//! Result types for command execution

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Result of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// Exit status code (0 for success)
    pub status: i32,
    /// stdout output
    pub stdout: String,
    /// stderr output
    pub stderr: String,
    /// Time taken to execute
    pub duration: Duration,
}

impl CommandResult {
    /// Check if command succeeded (exit code 0)
    #[must_use]
    pub fn success(&self) -> bool {
        self.status == 0
    }

    /// Combine stdout and stderr
    #[must_use]
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Connection information for SSH
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Host address
    pub host: String,
    /// Port (default 22)
    #[serde(default = "default_port")]
    pub port: u16,
    /// Username
    pub user: String,
    /// Optional SSH key path
    pub ssh_key: Option<String>,
}

fn default_port() -> u16 {
    22
}

impl ConnectionInfo {
    /// Create new connection info
    pub fn new(host: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 22,
            user: user.into(),
            ssh_key: None,
        }
    }

    /// Set SSH key path
    #[must_use]
    pub fn with_ssh_key(mut self, path: impl Into<String>) -> Self {
        self.ssh_key = Some(path.into());
        self
    }

    /// Set custom port
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

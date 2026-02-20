//! Configuration loading and types
//!
//! Minimal skeleton - full implementation pending

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tendhost_core::HostConfig;

/// Top-level configuration for tendhost daemon
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Daemon server settings
    #[serde(default)]
    pub daemon: DaemonConfig,
    /// Individual host configurations
    #[serde(default)]
    pub host: Vec<HostConfig>,
}

/// Daemon server settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Address and port to bind to
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            log_level: default_log_level(),
        }
    }
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    /// Load configuration from file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    pub fn load(path: &PathBuf) -> eyre::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load from default paths or use defaults
    pub fn load_default() -> eyre::Result<Self> {
        // Check environment variable
        if let Ok(path) = std::env::var("TENDHOST_CONFIG") {
            return Self::load(&PathBuf::from(path));
        }

        // Try common paths
        let paths = [
            PathBuf::from("tendhost.toml"),
            PathBuf::from("/etc/tendhost/tendhost.toml"),
            dirs::config_dir()
                .map(|p| p.join("tendhost/tendhost.toml"))
                .unwrap_or_default(),
        ];

        for path in paths {
            if path.exists() {
                return Self::load(&path);
            }
        }

        // Return default config if no file found
        tracing::warn!("no config file found, using defaults");
        Ok(Config::default())
    }
}

// TODO: Implement configuration parsing from tendhost.toml

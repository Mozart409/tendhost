# Implementation Plan: tendhost-exec

## Overview

This plan implements the remote execution abstraction for tendhost, providing:
- `RemoteExecutor` trait for command execution
- `LocalExecutor`: Local command execution via `tokio::process`
- `SshExecutor`: Remote execution via SSH (using `openssh` crate)
- Comprehensive error handling with `thiserror`
- SSH key management and authentication

## Architecture

```
┌─────────────────────────────────────────┐
│         RemoteExecutor Trait            │
│  • run(cmd) -> Result<CommandResult>    │
│  • run_with_timeout(cmd, timeout)       │
│  • connect() / disconnect() (SSH)       │
└─────────────────┬───────────────────────┘
                  │
      ┌───────────┴───────────┐
      ▼                       ▼
┌─────────────┐         ┌─────────────┐
│LocalExecutor│         │SshExecutor  │
│             │         │             │
│ tokio::proc │         │ openssh::Session
└─────────────┘         └─────────────┘
```

---

## Phase 1: Foundation

### Task 1.1: Create Error Types (`error.rs`)
**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost-exec/src/error.rs`:

```rust
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
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ExecError::ConnectionFailed(_) | ExecError::Timeout { .. }
        )
    }
}
```

**Acceptance criteria**:
- [ ] Error enum covers all failure modes
- [ ] Implements `std::error::Error` via thiserror
- [ ] `Clone` derive for use in actor messages
- [ ] Helper method `is_retryable()` for retry logic
- [ ] Public in lib.rs

---

### Task 1.2: Create Result Types (`result.rs`)
**Priority**: High  
**Estimated effort**: 20 min

Create `crates/tendhost-exec/src/result.rs`:

```rust
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
    pub fn success(&self) -> bool {
        self.status == 0
    }

    /// Combine stdout and stderr
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
    pub fn with_ssh_key(mut self, path: impl Into<String>) -> Self {
        self.ssh_key = Some(path.into());
        self
    }

    /// Set custom port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}
```

**Acceptance criteria**:
- [ ] `CommandResult` with all fields
- [ ] Helper methods `success()` and `combined_output()`
- [ ] `ConnectionInfo` for SSH configuration
- [ ] Serde derives for serialization
- [ ] Builder-style methods on `ConnectionInfo`

---

## Phase 2: Enhanced Trait

### Task 2.1: Enhance RemoteExecutor Trait (`traits.rs`)
**Priority**: High  
**Estimated effort**: 45 min

Rewrite `crates/tendhost-exec/src/traits.rs`:

```rust
//! Remote executor trait and implementations

use std::time::Duration;

use async_trait::async_trait;

use crate::error::ExecError;
use crate::result::CommandResult;

/// Trait for executing commands locally or remotely
///
/// Implementations must be `Send + Sync` for use across async contexts.
#[async_trait]
pub trait RemoteExecutor: Send + Sync {
    /// Execute a command and return the result
    ///
    /// # Arguments
    /// * `cmd` - The command to execute (shell syntax supported)
    ///
    /// # Returns
    /// * `Ok(CommandResult)` - Command completed, check `result.success()`
    /// * `Err(ExecError)` - Execution failed (connection, spawn, etc.)
    ///
    /// # Example
    /// ```rust
    /// let result = executor.run("echo hello").await?;
    /// assert!(result.success());
    /// ```
    async fn run(&self, cmd: &str) -> Result<CommandResult, ExecError>;

    /// Execute a command with a timeout
    ///
    /// # Arguments
    /// * `cmd` - The command to execute
    /// * `timeout` - Maximum time to wait before aborting
    ///
    /// # Returns
    /// * `Ok(CommandResult)` - Command completed within timeout
    /// * `Err(ExecError::Timeout)` - Command exceeded timeout
    async fn run_with_timeout(
        &self,
        cmd: &str,
        timeout: Duration,
    ) -> Result<CommandResult, ExecError>;

    /// Check if executor is connected (for SSH implementations)
    ///
    /// Local executors always return true.
    fn is_connected(&self) -> bool {
        true
    }

    /// Get executor type name for logging
    fn executor_type(&self) -> &'static str;
}

/// Extension trait for common command patterns
#[async_trait]
pub trait RemoteExecutorExt: RemoteExecutor {
    /// Run command and return stdout only if successful
    async fn run_ok(&self, cmd: &str) -> Result<String, ExecError> {
        let result = self.run(cmd).await?;
        if result.success() {
            Ok(result.stdout.trim().to_string())
        } else {
            Err(ExecError::CommandFailed {
                status: result.status,
                stderr: result.stderr,
            })
        }
    }

    /// Check if a command exists
    async fn command_exists(&self, cmd: &str) -> Result<bool, ExecError> {
        let result = self.run(&format!("which {cmd}")).await?;
        Ok(result.success())
    }

    /// Run multiple commands in sequence, stopping on first failure
    async fn run_sequence(&self, cmds: &[&str]) -> Result<Vec<CommandResult>, ExecError> {
        let mut results = Vec::new();
        for cmd in cmds {
            let result = self.run(cmd).await?;
            if !result.success() {
                return Err(ExecError::CommandFailed {
                    status: result.status,
                    stderr: result.stderr,
                });
            }
            results.push(result);
        }
        Ok(results)
    }
}

#[async_trait]
impl<T: RemoteExecutor> RemoteExecutorExt for T {}
```

**Acceptance criteria**:
- [ ] Enhanced trait with `CommandResult` return type
- [ ] `is_connected()` default implementation
- [ ] `executor_type()` for logging/debugging
- [ ] Extension trait with helper methods
- [ ] Comprehensive doc comments

---

## Phase 3: Local Executor Implementation

### Task 3.1: Implement LocalExecutor (`local.rs`)
**Priority**: High  
**Estimated effort**: 1 hour

Rewrite `crates/tendhost-exec/src/local.rs`:

```rust
//! Local command execution using tokio::process

use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, instrument};

use crate::error::ExecError;
use crate::result::CommandResult;
use crate::traits::RemoteExecutor;

/// Local command executor
///
/// Executes commands on the local machine using `tokio::process::Command`.
#[derive(Debug, Clone)]
pub struct LocalExecutor;

impl LocalExecutor {
    /// Create a new local executor
    pub fn new() -> Self {
        Self
    }

    /// Internal method to execute command
    #[instrument(skip(self), level = "debug")]
    async fn execute(&self, cmd: &str) -> Result<CommandResult, ExecError> {
        let start = Instant::now();

        debug!(command = %cmd, "executing local command");

        // Use shell to support pipes, redirections, etc.
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ExecError::SpawnError(e.to_string()))?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| ExecError::IoError(e.to_string()))?;

        let duration = start.elapsed();

        let status = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        debug!(
            command = %cmd,
            status = status,
            duration = ?duration,
            "command completed"
        );

        if !output.status.success() {
            error!(
                command = %cmd,
                status = status,
                stderr = %stderr,
                "command failed"
            );
        }

        Ok(CommandResult {
            status,
            stdout,
            stderr,
            duration,
        })
    }
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RemoteExecutor for LocalExecutor {
    #[instrument(skip(self), level = "debug")]
    async fn run(&self, cmd: &str) -> Result<CommandResult, ExecError> {
        self.execute(cmd).await
    }

    #[instrument(skip(self), level = "debug")]
    async fn run_with_timeout(
        &self,
        cmd: &str,
        timeout_duration: Duration,
    ) -> Result<CommandResult, ExecError> {
        let start = Instant::now();

        debug!(command = %cmd, timeout = ?timeout_duration, "executing with timeout");

        let result = timeout(timeout_duration, self.execute(cmd)).await;

        match result {
            Ok(Ok(cmd_result)) => Ok(cmd_result),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                error!(
                    command = %cmd,
                    timeout = ?timeout_duration,
                    elapsed = ?start.elapsed(),
                    "command timed out"
                );
                Err(ExecError::Timeout {
                    timeout: timeout_duration,
                })
            }
        }
    }

    fn executor_type(&self) -> &'static str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_success() {
        let executor = LocalExecutor::new();
        let result = executor.run("echo hello").await.unwrap();

        assert!(result.success());
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn test_run_failure() {
        let executor = LocalExecutor::new();
        let result = executor.run("exit 42").await.unwrap();

        assert!(!result.success());
        assert_eq!(result.status, 42);
    }

    #[tokio::test]
    async fn test_run_timeout() {
        let executor = LocalExecutor::new();
        let result = executor.run_with_timeout("sleep 5", Duration::from_millis(100)).await;

        assert!(matches!(result, Err(ExecError::Timeout { .. })));
    }

    #[tokio::test]
    async fn test_run_with_stderr() {
        let executor = LocalExecutor::new();
        let result = executor.run("echo error >&2").await.unwrap();

        assert!(result.success());
        assert_eq!(result.stderr.trim(), "error");
    }
}
```

**Acceptance criteria**:
- [ ] Uses `tokio::process::Command` for async execution
- [ ] Supports shell syntax via `sh -c`
- [ ] Proper stdout/stderr capture
- [ ] Timeout support with `tokio::time::timeout`
- [ ] Structured logging with `tracing`
- [ ] Unit tests for success/failure/timeout cases

---

## Phase 4: SSH Executor Implementation

### Task 4.1: Add openssh Dependency
**Priority**: High  
**Estimated effort**: 5 min

Update `crates/tendhost-exec/Cargo.toml`:

```toml
[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

# SSH support
openssh = "0.11"
```

---

### Task 4.2: Implement SSH Key Resolution (`keys.rs`)
**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost-exec/src/keys.rs`:

```rust
//! SSH key management and resolution

use std::env;
use std::path::PathBuf;

use tracing::{debug, warn};

/// SSH key resolution strategy
#[derive(Debug, Clone)]
pub enum KeySource {
    /// Explicit path to key file
    Path(PathBuf),
    /// Use SSH agent
    Agent,
    /// Base64-encoded key from environment
    Env(String),
}

impl KeySource {
    /// Resolve key source to a path or agent
    ///
    /// For `Env`, decodes base64 and writes to temp file
    pub fn resolve(&self) -> Result<ResolvedKey, KeyError> {
        match self {
            KeySource::Path(path) => {
                validate_key_permissions(path)?;
                Ok(ResolvedKey::Path(path.clone()))
            }
            KeySource::Agent => Ok(ResolvedKey::Agent),
            KeySource::Env(var_name) => {
                let base64_key = env::var(var_name)
                    .map_err(|_| KeyError::EnvNotSet(var_name.clone()))?;
                let key_data = base64::decode(&base64_key)
                    .map_err(|_| KeyError::InvalidBase64)?;

                // Write to temp file
                let temp_path = write_temp_key(&key_data)?;
                Ok(ResolvedKey::Temp(temp_path))
            }
        }
    }
}

/// Resolved key location
#[derive(Debug)]
pub enum ResolvedKey {
    /// Path to key file
    Path(PathBuf),
    /// Use SSH agent
    Agent,
    /// Temporary file (will be deleted on drop)
    Temp(PathBuf),
}

impl ResolvedKey {
    /// Get path for SSH library
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            ResolvedKey::Path(p) | ResolvedKey::Temp(p) => Some(p),
            ResolvedKey::Agent => None,
        }
    }

    /// Whether to use SSH agent
    pub fn use_agent(&self) -> bool {
        matches!(self, ResolvedKey::Agent)
    }
}

/// Key resolution errors
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("environment variable {0} not set")]
    EnvNotSet(String),

    #[error("invalid base64 encoding")]
    InvalidBase64,

    #[error("key file permissions too open: {0} (should be 600)")]
    BadPermissions(String),

    #[error("key file not found: {0}")]
    NotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

fn validate_key_permissions(path: &PathBuf) -> Result<(), KeyError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)
        .map_err(|e| KeyError::Io(e))?;

    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if permissions are 600 (owner read/write only)
    // mode & 0o77 checks group and other permissions
    if mode & 0o77 != 0 {
        return Err(KeyError::BadPermissions(path.display().to_string()));
    }

    Ok(())
}

fn write_temp_key(key_data: &[u8]) -> Result<PathBuf, KeyError> {
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let temp_path = std::env::temp_dir().join(format!(
        "tendhost_ssh_key_{}",
        std::process::id()
    ));

    let mut file = File::create(&temp_path)?;
    file.write_all(key_data)?;

    // Set 600 permissions
    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(&temp_path, permissions)?;

    debug!(path = %temp_path.display(), "wrote temporary SSH key");

    Ok(temp_path)
}

impl Drop for ResolvedKey {
    fn drop(&mut self) {
        if let ResolvedKey::Temp(ref path) = self {
            if let Err(e) = std::fs::remove_file(path) {
                warn!(path = %path.display(), error = %e, "failed to remove temp key");
            }
        }
    }
}
```

**Acceptance criteria**:
- [ ] `KeySource` enum for different sources
- [ ] Permission validation (600)
- [ ] Base64 decoding from environment
- [ ] Temp file cleanup on drop
- [ ] Error handling with `thiserror`

---

### Task 4.3: Implement SshExecutor (`ssh.rs`)
**Priority**: High  
**Estimated effort**: 1.5 hours

Rewrite `crates/tendhost-exec/src/ssh.rs`:

```rust
//! SSH command execution using openssh crate

use std::time::{Duration, Instant};

use async_trait::async_trait;
use openssh::{Session, SessionBuilder, Stdio};
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, warn};

use crate::error::ExecError;
use crate::keys::{KeySource, ResolvedKey};
use crate::result::{CommandResult, ConnectionInfo};
use crate::traits::RemoteExecutor;

/// SSH command executor
///
/// Manages an SSH session for remote command execution.
/// Connections are established on first use.
pub struct SshExecutor {
    /// Connection configuration
    conn_info: ConnectionInfo,
    /// Resolved SSH key
    key: ResolvedKey,
    /// SSH session (initialized on first use)
    session: Option<Session>,
}

impl SshExecutor {
    /// Create a new SSH executor
    ///
    /// # Arguments
    /// * `conn_info` - Connection details (host, user, port, key)
    /// * `key_source` - How to obtain the SSH key
    ///
    /// # Errors
    /// Returns `ExecError::SshKeyError` if key resolution fails
    pub fn new(
        conn_info: ConnectionInfo,
        key_source: KeySource,
    ) -> Result<Self, ExecError> {
        let key = key_source
            .resolve()
            .map_err(|e| ExecError::SshKeyError(e.to_string()))?;

        Ok(Self {
            conn_info,
            key,
            session: None,
        })
    }

    /// Get connection info
    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.conn_info
    }

    /// Connect to the remote host
    #[instrument(skip(self), fields(host = %self.conn_info.host))]
    async fn connect(&mut self) -> Result<(), ExecError> {
        if self.session.is_some() {
            return Ok(());
        }

        info!(
            host = %self.conn_info.host,
            port = self.conn_info.port,
            user = %self.conn_info.user,
            "connecting to SSH"
        );

        let mut builder = SessionBuilder::default();
        builder
            .user(&self.conn_info.user)
            .port(self.conn_info.port)
            .known_hosts_check(openssh::KnownHosts::Accept);

        // Configure authentication
        if self.key.use_agent() {
            builder.keyfile(&self.conn_info.user); // Use agent
        } else if let Some(key_path) = self.key.path() {
            builder.keyfile(key_path);
        }

        let session = builder
            .connect(&self.conn_info.host)
            .await
            .map_err(|e| ExecError::ConnectionFailed(e.to_string()))?;

        info!(host = %self.conn_info.host, "SSH connected");

        self.session = Some(session);
        Ok(())
    }

    /// Execute command on remote host
    #[instrument(skip(self, cmd), fields(host = %self.conn_info.host))]
    async fn execute_remote(&self, cmd: &str) -> Result<CommandResult, ExecError> {
        let session = self
            .session
            .as_ref()
            .ok_or(ExecError::NotConnected)?;

        debug!(command = %cmd, "executing remote command");

        let start = Instant::now();

        let mut remote_cmd = session.command("sh");
        remote_cmd.arg("-c").arg(cmd);

        let output = remote_cmd
            .output()
            .await
            .map_err(|e| ExecError::IoError(e.to_string()))?;

        let duration = start.elapsed();
        let status = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        debug!(
            command = %cmd,
            status = status,
            duration = ?duration,
            "remote command completed"
        );

        Ok(CommandResult {
            status,
            stdout,
            stderr,
            duration,
        })
    }

    /// Disconnect from remote host
    pub async fn disconnect(mut self) -> Result<(), ExecError> {
        if let Some(session) = self.session.take() {
            session
                .close()
                .await
                .map_err(|e| ExecError::IoError(e.to_string()))?;
            info!(host = %self.conn_info.host, "SSH disconnected");
        }
        Ok(())
    }
}

#[async_trait]
impl RemoteExecutor for SshExecutor {
    #[instrument(skip(self), fields(host = %self.conn_info.host))]
    async fn run(&self, cmd: &str) -> Result<CommandResult, ExecError> {
        // Note: This requires &mut self for connect, but trait uses &self
        // We'll need interior mutability (Mutex) for the session
        todo!("Implement with interior mutability")
    }

    #[instrument(skip(self), fields(host = %self.conn_info.host))]
    async fn run_with_timeout(
        &self,
        cmd: &str,
        timeout_duration: Duration,
    ) -> Result<CommandResult, ExecError> {
        todo!("Implement with timeout")
    }

    fn is_connected(&self) -> bool {
        self.session.is_some()
    }

    fn executor_type(&self) -> &'static str {
        "ssh"
    }
}

/// Builder for SshExecutor
pub struct SshExecutorBuilder {
    conn_info: ConnectionInfo,
    key_source: KeySource,
}

impl SshExecutorBuilder {
    /// Create builder with required fields
    pub fn new(host: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            conn_info: ConnectionInfo::new(host, user),
            key_source: KeySource::Agent, // Default to agent
        }
    }

    /// Set SSH key path
    pub fn with_key_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.key_source = KeySource::Path(path.into());
        self
    }

    /// Use SSH agent
    pub fn with_agent(mut self) -> Self {
        self.key_source = KeySource::Agent;
        self
    }

    /// Set key from environment variable (base64)
    pub fn with_env_key(mut self, var_name: impl Into<String>) -> Self {
        self.key_source = KeySource::Env(var_name.into());
        self
    }

    /// Set custom port
    pub fn with_port(mut self, port: u16) -> Self {
        self.conn_info.port = port;
        self
    }

    /// Build the executor
    pub fn build(self) -> Result<SshExecutor, ExecError> {
        SshExecutor::new(self.conn_info, self.key_source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests require an SSH server - marked as ignored
    #[tokio::test]
    #[ignore = "requires SSH server"]
    async fn test_ssh_connection() {
        // This is a placeholder for actual SSH tests
        // Would require a test SSH server or mocking
    }
}
```

**Acceptance criteria**:
- [ ] Uses `openssh` crate for SSH connections
- [ ] Lazy connection establishment
- [ ] Builder pattern for configuration
- [ ] Support for key file, agent, and env key
- [ ] Timeout support
- [ ] Error handling with specific error types

---

## Phase 5: Integration

### Task 5.1: Update lib.rs Exports
**Priority**: Medium  
**Estimated effort**: 15 min

Update `crates/tendhost-exec/src/lib.rs`:

```rust
//! tendhost-exec: Remote execution abstraction
//!
//! Provides traits and implementations for executing commands locally and remotely via SSH.
//!
//! # Example
//! ```rust
//! use tendhost_exec::{LocalExecutor, RemoteExecutor};
//!
//! let executor = LocalExecutor::new();
//! let result = executor.run("echo hello").await.unwrap();
//! assert!(result.success());
//! ```

pub mod error;
pub mod keys;
pub mod local;
pub mod result;
pub mod ssh;
pub mod traits;

pub use error::ExecError;
pub use keys::{KeySource, ResolvedKey};
pub use local::LocalExecutor;
pub use result::{CommandResult, ConnectionInfo};
pub use ssh::{SshExecutor, SshExecutorBuilder};
pub use traits::{RemoteExecutor, RemoteExecutorExt};
```

---

## Summary

### File Changes Required

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add openssh dependency |
| `src/error.rs` | Create | Error types |
| `src/result.rs` | Create | Result and connection types |
| `src/keys.rs` | Create | SSH key management |
| `src/traits.rs` | Modify | Enhanced RemoteExecutor trait |
| `src/local.rs` | Modify | LocalExecutor implementation |
| `src/ssh.rs` | Modify | SshExecutor implementation |
| `src/lib.rs` | Modify | Re-exports |

### Estimated Total Effort

| Phase | Effort |
|-------|--------|
| Phase 1: Foundation | 50 min |
| Phase 2: Trait Enhancement | 45 min |
| Phase 3: Local Executor | 1 hour |
| Phase 4: SSH Executor | 2 hours |
| Phase 5: Integration | 15 min |
| **Total** | **~4.5 hours** |

### Dependencies

- **Blocks**: `tendhost-pkg` (uses this trait)
- **Blocked by**: None

### Notes

- Interior mutability needed for SSH session (Mutex around Option<Session>)
- Consider connection pooling for scale (future enhancement)
- SSH tests require infrastructure (marked as `#[ignore]`)

//! SSH command execution using russh crate

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use russh::keys::ssh_key;
use russh::keys::{PrivateKeyWithHashAlg, load_secret_key};
use russh::{ChannelMsg, Disconnect, client};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument};

use crate::error::ExecError;
use crate::keys::{KeySource, ResolvedKey};
use crate::result::{CommandResult, ConnectionInfo};
use crate::traits::RemoteExecutor;

/// SSH client handler for russh
#[derive(Debug)]
struct SshClientHandler;

impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept all server keys (like StrictHostKeyChecking=no)
        // In production, this should verify against known_hosts
        Ok(true)
    }
}

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
    session: Mutex<Option<client::Handle<SshClientHandler>>>,
}

impl std::fmt::Debug for SshExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshExecutor")
            .field("conn_info", &self.conn_info)
            .field("key", &self.key)
            .field("connected", &self.is_connected())
            .finish_non_exhaustive()
    }
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
    pub fn new(conn_info: ConnectionInfo, key_source: &KeySource) -> Result<Self, ExecError> {
        let key = key_source
            .resolve()
            .map_err(|e| ExecError::SshKeyError(e.to_string()))?;

        Ok(Self {
            conn_info,
            key,
            session: Mutex::new(None),
        })
    }

    /// Get connection info
    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.conn_info
    }

    /// Connect to the remote host
    #[instrument(skip(self), fields(host = %self.conn_info.host))]
    async fn connect(&self) -> Result<(), ExecError> {
        let mut session_lock = self.session.lock().await;

        if session_lock.is_some() {
            return Ok(());
        }

        info!(
            host = %self.conn_info.host,
            port = self.conn_info.port,
            user = %self.conn_info.user,
            "connecting to SSH"
        );

        // Configure client
        let config = client::Config::default();
        let config = Arc::new(config);

        // Create handler
        let handler = SshClientHandler;

        // Connect
        let mut session = client::connect(
            config,
            (&self.conn_info.host[..], self.conn_info.port),
            handler,
        )
        .await
        .map_err(|e| ExecError::ConnectionFailed(e.to_string()))?;

        // Authenticate
        if self.key.use_agent() {
            // SSH agent authentication - try loading keys from agent
            // For now, fall through to try key-based auth or fail
            // TODO: Implement proper SSH agent support with pageant
            return Err(ExecError::AuthenticationFailed(
                "SSH agent authentication not yet implemented".to_string(),
            ));
        } else if let Some(key_path) = self.key.path() {
            // Load private key and authenticate
            let key_pair = load_secret_key(key_path, None)
                .map_err(|e| ExecError::SshKeyError(e.to_string()))?;

            let hash_alg = session
                .best_supported_rsa_hash()
                .await
                .ok()
                .flatten()
                .flatten();
            let auth_res = session
                .authenticate_publickey(
                    &self.conn_info.user,
                    PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg),
                )
                .await
                .map_err(|e| ExecError::AuthenticationFailed(e.to_string()))?;

            if !auth_res.success() {
                return Err(ExecError::AuthenticationFailed(
                    "Public key authentication failed".to_string(),
                ));
            }
        } else {
            return Err(ExecError::AuthenticationFailed(
                "No authentication method available".to_string(),
            ));
        }

        info!(host = %self.conn_info.host, "SSH connected and authenticated");

        *session_lock = Some(session);
        Ok(())
    }

    /// Execute command on remote host
    #[instrument(skip(self, cmd), fields(host = %self.conn_info.host))]
    async fn execute_remote(&self, cmd: &str) -> Result<CommandResult, ExecError> {
        let mut session_lock = self.session.lock().await;

        let session = session_lock.as_mut().ok_or(ExecError::NotConnected)?;

        debug!(command = %cmd, "executing remote command");

        let start = Instant::now();

        // Open session channel
        let mut channel = session
            .channel_open_session()
            .await
            .map_err(|e| ExecError::IoError(e.to_string()))?;

        // Execute command
        channel
            .exec(true, cmd)
            .await
            .map_err(|e| ExecError::IoError(e.to_string()))?;

        // Collect output
        let mut status = -1;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        loop {
            let msg = channel.wait().await;

            match msg {
                Some(ChannelMsg::Data { data }) => {
                    stdout.extend_from_slice(&data);
                }
                Some(ChannelMsg::ExtendedData { data, ext }) => {
                    if ext == 1 {
                        // stderr
                        stderr.extend_from_slice(&data);
                    }
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    status = exit_status.cast_signed();
                }
                Some(ChannelMsg::Eof) | None => break,
                _ => {}
            }
        }

        let duration = start.elapsed();
        let stdout = String::from_utf8_lossy(&stdout).to_string();
        let stderr = String::from_utf8_lossy(&stderr).to_string();

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
    ///
    /// # Errors
    /// Returns `ExecError::IoError` if disconnection fails
    pub async fn disconnect(&self) -> Result<(), ExecError> {
        let mut session_lock = self.session.lock().await;

        if let Some(session) = session_lock.take() {
            session
                .disconnect(Disconnect::ByApplication, "", "English")
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
        self.connect().await?;
        self.execute_remote(cmd).await
    }

    #[instrument(skip(self), fields(host = %self.conn_info.host))]
    async fn run_with_timeout(
        &self,
        cmd: &str,
        timeout_duration: Duration,
    ) -> Result<CommandResult, ExecError> {
        let start = Instant::now();

        debug!(command = %cmd, timeout = ?timeout_duration, "executing with timeout");

        // Ensure connection first (outside of timeout)
        self.connect().await?;

        // Execute with timeout
        let result = timeout(timeout_duration, self.execute_remote(cmd)).await;

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

    fn is_connected(&self) -> bool {
        // Check if session is Some
        // Note: This is a synchronous check, the actual connection
        // state can only be verified by trying to use the connection
        let session_opt = self.session.try_lock();
        session_opt.map(|s| s.is_some()).unwrap_or(false)
    }

    fn executor_type(&self) -> &'static str {
        "ssh"
    }
}

/// Builder for `SshExecutor`
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
    #[must_use]
    pub fn with_key_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.key_source = KeySource::Path(path.into());
        self
    }

    /// Use SSH agent
    #[must_use]
    pub fn with_agent(mut self) -> Self {
        self.key_source = KeySource::Agent;
        self
    }

    /// Set key from environment variable (base64)
    #[must_use]
    pub fn with_env_key(mut self, var_name: impl Into<String>) -> Self {
        self.key_source = KeySource::Env(var_name.into());
        self
    }

    /// Set custom port
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.conn_info.port = port;
        self
    }

    /// Build the executor
    ///
    /// # Errors
    /// Returns `ExecError::SshKeyError` if key resolution fails
    pub fn build(self) -> Result<SshExecutor, ExecError> {
        SshExecutor::new(self.conn_info, &self.key_source)
    }
}

#[cfg(test)]
mod tests {
    // These tests require an SSH server - marked as ignored
    #[tokio::test]
    #[ignore = "requires SSH server"]
    async fn test_ssh_connection() {
        // This is a placeholder for actual SSH tests
        // Would require a test SSH server or mocking
    }
}

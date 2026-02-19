//! Local command execution using `tokio::process`

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
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Internal method to execute command
    #[instrument(skip(self), level = "debug")]
    async fn execute(&self, cmd: &str) -> Result<CommandResult, ExecError> {
        let start = Instant::now();

        debug!(command = %cmd, "executing local command");

        // Use shell to support pipes, redirections, etc.
        let child = Command::new("sh")
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
        let result = executor
            .run_with_timeout("sleep 5", Duration::from_millis(100))
            .await;

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

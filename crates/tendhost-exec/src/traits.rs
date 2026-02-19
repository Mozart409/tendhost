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
    /// ```rust,no_run
    /// use tendhost_exec::{LocalExecutor, RemoteExecutor};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), tendhost_exec::ExecError> {
    /// let executor = LocalExecutor::new();
    /// let result = executor.run("echo hello").await?;
    /// assert!(result.success());
    /// # Ok(())
    /// # }
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

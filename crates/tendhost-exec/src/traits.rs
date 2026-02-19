//! Remote executor trait

use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait RemoteExecutor: Send + Sync {
    async fn run(&self, cmd: &str) -> Result<String, String>;
    async fn run_with_timeout(&self, cmd: &str, timeout: Duration) -> Result<String, String>;
}

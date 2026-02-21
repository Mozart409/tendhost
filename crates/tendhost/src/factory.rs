//! Host actor factory for creating SSH executors and package managers

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use eyre::Result;
use tendhost_core::{HostActorFactory, HostConfig};
use tendhost_exec::{ConnectionInfo, KeySource, LocalExecutor, RemoteExecutor, SshExecutor};
use tendhost_pkg::{AptManager, DnfManager, DockerComposeManager, PackageManager};

/// Default implementation of `HostActorFactory`
pub struct DefaultHostFactory;

impl DefaultHostFactory {
    /// Create a new factory instance
    pub fn new() -> Self {
        Self
    }

    /// Create a remote executor for a host
    fn create_executor_sync(config: &HostConfig) -> Result<Arc<dyn RemoteExecutor>> {
        // For localhost connections, use LocalExecutor
        if config.addr == "localhost" || config.addr == "127.0.0.1" {
            return Ok(Arc::new(LocalExecutor::new()));
        }

        // Otherwise create SSH executor
        let key_source = if let Some(key_path) = &config.ssh_key {
            KeySource::Path(key_path.clone().into())
        } else {
            KeySource::Agent
        };

        let conn_info = ConnectionInfo::new(&config.addr, &config.user);
        let executor = SshExecutor::new(conn_info, &key_source)
            .map_err(|e| eyre::eyre!("failed to create SSH executor: {e}"))?;
        Ok(Arc::new(executor))
    }

    /// Detect package manager by probing the host
    async fn detect_package_manager(
        executor: Arc<dyn RemoteExecutor>,
    ) -> Result<Arc<dyn PackageManager>> {
        // Determine if we need sudo (check if we're root)
        let whoami = executor.run("whoami").await;
        let use_sudo = whoami
            .as_ref()
            .map(|r| !r.stdout.trim().eq("root"))
            .unwrap_or(true);

        // Try apt first (Debian/Ubuntu)
        let apt_check = executor.run("which apt-get").await;
        if apt_check.is_ok() && apt_check.as_ref().unwrap().success() {
            tracing::info!(use_sudo, "detected apt package manager");
            return Ok(Arc::new(AptManager::new(executor, use_sudo)));
        }

        // Try dnf (Fedora/RHEL 8+)
        let dnf_check = executor.run("which dnf").await;
        if dnf_check.is_ok() && dnf_check.as_ref().unwrap().success() {
            tracing::info!(use_sudo, "detected dnf package manager");
            return Ok(Arc::new(DnfManager::new(executor, use_sudo)));
        }

        // Try yum (CentOS 7/RHEL 7)
        let yum_check = executor.run("which yum").await;
        if yum_check.is_ok() && yum_check.as_ref().unwrap().success() {
            tracing::info!(use_sudo, "detected yum package manager (using DnfManager)");
            return Ok(Arc::new(DnfManager::new(executor, use_sudo)));
        }

        eyre::bail!("no supported package manager found (tried apt, dnf, yum)")
    }

    /// Create Docker Compose manager if compose paths are configured
    #[allow(dead_code)]
    fn create_compose_manager(
        config: &HostConfig,
        executor: Arc<dyn RemoteExecutor>,
    ) -> Option<Arc<dyn PackageManager>> {
        if config.compose_paths.is_empty() {
            return None;
        }

        // Convert String paths to PathBuf
        let compose_dirs: Vec<PathBuf> = config.compose_paths.iter().map(PathBuf::from).collect();

        match DockerComposeManager::new(executor, compose_dirs) {
            Ok(manager) => Some(Arc::new(manager)),
            Err(e) => {
                tracing::error!(error = %e, "failed to create docker compose manager");
                None
            }
        }
    }
}

impl Default for DefaultHostFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HostActorFactory for DefaultHostFactory {
    async fn create_executor(&self, config: &HostConfig) -> Arc<dyn RemoteExecutor> {
        Self::create_executor_sync(config).expect("failed to create executor - configuration error")
    }

    async fn create_package_manager(
        &self,
        _config: &HostConfig,
        executor: Arc<dyn RemoteExecutor>,
    ) -> Arc<dyn PackageManager> {
        Self::detect_package_manager(executor)
            .await
            .expect("failed to detect package manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localhost_executor_creation() {
        use tendhost_core::HostPolicy;

        let config = HostConfig {
            name: "localhost".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            ssh_key: None,
            compose_paths: vec![],
            tags: vec![],
            policy: HostPolicy::default(),
        };

        let executor = DefaultHostFactory::create_executor_sync(&config);
        assert!(executor.is_ok());
    }

    #[test]
    fn test_compose_manager_creation() {
        use tendhost_core::HostPolicy;

        let config = HostConfig {
            name: "docker-host".to_string(),
            addr: "localhost".to_string(),
            user: "root".to_string(),
            ssh_key: None,
            compose_paths: vec!["/opt/stacks".to_string()],
            tags: vec![],
            policy: HostPolicy::default(),
        };

        let executor = Arc::new(LocalExecutor::new());
        let compose = DefaultHostFactory::create_compose_manager(&config, executor);
        assert!(compose.is_some());
    }
}

//! Docker Compose stack management

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tendhost_exec::traits::RemoteExecutor;
use tracing::{debug, error, info, instrument};

use crate::error::PackageError;
use crate::traits::PackageManager;
use crate::types::{PackageManagerType, UpdateResult, UpgradablePackage};

/// Docker Compose manager
///
/// Manages Docker Compose stacks by pulling and recreating containers.
pub struct DockerComposeManager {
    executor: Arc<dyn RemoteExecutor>,
    /// Directories containing docker-compose.yml files
    compose_dirs: Vec<PathBuf>,
    /// Whether to use "docker compose" (v2) or "docker-compose" (v1)
    use_v2: bool,
    /// Whether to pull images before updating
    pull_before_update: bool,
}

impl DockerComposeManager {
    /// Create a new Docker Compose manager
    ///
    /// # Arguments
    /// * `executor` - Remote executor
    /// * `compose_dirs` - Directories containing compose files
    ///
    /// # Errors
    /// Returns `PackageError::ConfigError` if no compose directories are specified
    pub fn new(
        executor: Arc<dyn RemoteExecutor>,
        compose_dirs: Vec<PathBuf>,
    ) -> Result<Self, PackageError> {
        if compose_dirs.is_empty() {
            return Err(PackageError::ConfigError(
                "no compose directories specified".to_string(),
            ));
        }

        Ok(Self {
            executor,
            compose_dirs,
            use_v2: true, // Will detect
            pull_before_update: true,
        })
    }

    /// Detect docker compose version
    #[allow(dead_code)]
    async fn detect_version(&mut self) -> Result<(), PackageError> {
        // Check if docker exists
        let has_docker = self
            .executor
            .run("which docker")
            .await
            .map(|r| r.success())
            .unwrap_or(false);

        if has_docker {
            // Try docker compose (v2)
            let result = self.executor.run("docker compose version").await;
            if result.is_ok()
                && result
                    .as_ref()
                    .is_ok_and(tendhost_exec::CommandResult::success)
            {
                self.use_v2 = true;
                return Ok(());
            }
        }

        // Try docker-compose (v1)
        let has_compose = self
            .executor
            .run("which docker-compose")
            .await
            .map(|r| r.success())
            .unwrap_or(false);

        if has_compose {
            self.use_v2 = false;
            return Ok(());
        }

        Err(PackageError::DockerComposeNotFound)
    }

    /// Build docker compose command
    fn compose_cmd(&self, compose_dir: &Path, args: &str) -> String {
        let cmd = if self.use_v2 {
            "docker compose"
        } else {
            "docker-compose"
        };
        let dir = compose_dir.display();
        format!("{cmd} -f {dir}/docker-compose.yml {args}")
    }

    /// Check if compose file exists
    async fn compose_file_exists(&self, compose_dir: &Path) -> Result<bool, PackageError> {
        let path = compose_dir.join("docker-compose.yml");
        let result = self
            .executor
            .run(&format!("test -f {}", path.display()))
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;
        Ok(result.success())
    }
}

#[async_trait]
impl PackageManager for DockerComposeManager {
    #[instrument(skip(self))]
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, PackageError> {
        // Docker Compose doesn't have a direct "list upgradable" concept
        // We check if images have updates available
        debug!("checking for docker image updates");

        let mut upgradable = Vec::new();

        for compose_dir in &self.compose_dirs {
            if !self.compose_file_exists(compose_dir).await? {
                continue;
            }

            // Get list of services
            let cmd = self.compose_cmd(compose_dir, "config --services");
            let result = self
                .executor
                .run(&cmd)
                .await
                .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

            if !result.success() {
                continue;
            }

            // For each service, check if image can be pulled
            for service in result.stdout.lines() {
                let service = service.trim();
                if service.is_empty() {
                    continue;
                }

                // Get current image
                let img_cmd = format!(
                    "docker compose -f {}/docker-compose.yml ps -q {}",
                    compose_dir.display(),
                    service
                );
                let img_result = self
                    .executor
                    .run(&img_cmd)
                    .await
                    .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

                if img_result.success() && !img_result.stdout.trim().is_empty() {
                    // Check if newer image available
                    let check_cmd = format!(
                        "docker compose -f {}/docker-compose.yml pull --dry-run {} 2>&1 || true",
                        compose_dir.display(),
                        service
                    );
                    let check_result = self
                        .executor
                        .run(&check_cmd)
                        .await
                        .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

                    if check_result.stdout.contains("Downloaded newer image") {
                        upgradable.push(UpgradablePackage::new(
                            format!("{}/{}", compose_dir.display(), service),
                            "current",
                            "available",
                        ));
                    }
                }
            }
        }

        info!(count = upgradable.len(), "found upgradable docker services");
        Ok(upgradable)
    }

    #[instrument(skip(self))]
    async fn upgrade_all(&self) -> Result<UpdateResult, PackageError> {
        info!("starting docker compose update");

        let mut total_upgraded = 0u32;
        let mut errors = Vec::new();

        for compose_dir in &self.compose_dirs {
            if !self.compose_file_exists(compose_dir).await? {
                error!(dir = %compose_dir.display(), "compose file not found");
                continue;
            }

            // Pull images if configured
            if self.pull_before_update {
                let pull_cmd = self.compose_cmd(compose_dir, "pull");
                let pull_result = self
                    .executor
                    .run(&pull_cmd)
                    .await
                    .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

                if !pull_result.success() {
                    errors.push(format!("{}: pull failed", compose_dir.display()));
                    continue;
                }
            }

            // Recreate containers with new images
            let up_cmd = self.compose_cmd(compose_dir, "up -d --force-recreate");
            let up_result = self
                .executor
                .run(&up_cmd)
                .await
                .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

            if up_result.success() {
                // Count services in this compose file
                let ps_cmd = self.compose_cmd(compose_dir, "ps -q");
                let ps_result = self
                    .executor
                    .run(&ps_cmd)
                    .await
                    .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

                if ps_result.success() {
                    let count = u32::try_from(ps_result.stdout.lines().count()).unwrap_or(0);
                    total_upgraded += count;
                }
            } else {
                errors.push(format!("{}: up failed", compose_dir.display()));
            }
        }

        let success = errors.is_empty();
        let mut result = UpdateResult::success(total_upgraded);
        result.success = success;
        if !success {
            result.error = Some(errors.join("; "));
        }

        info!(
            upgraded = total_upgraded,
            success = success,
            "docker compose update completed"
        );

        Ok(result)
    }

    #[instrument(skip(self))]
    async fn upgrade_dry_run(&self) -> Result<UpdateResult, PackageError> {
        debug!("starting docker compose dry run");

        let mut total_upgradable = 0u32;

        for compose_dir in &self.compose_dirs {
            if !self.compose_file_exists(compose_dir).await? {
                continue;
            }

            // Just check what would be pulled
            let cmd = self.compose_cmd(compose_dir, "pull --dry-run");
            let result = self
                .executor
                .run(&cmd)
                .await
                .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

            if result.success() {
                // Count images that would be pulled
                let count = u32::try_from(result.stdout.matches("Pulling").count()).unwrap_or(0);
                total_upgradable += count;
            }
        }

        Ok(UpdateResult::success(total_upgradable))
    }

    #[instrument(skip(self))]
    async fn reboot_required(&self) -> Result<bool, PackageError> {
        // Docker containers don't require host reboot
        Ok(false)
    }

    fn manager_type(&self) -> PackageManagerType {
        PackageManagerType::DockerCompose
    }

    async fn is_available(&self) -> bool {
        // Check if docker exists
        match self.executor.run("which docker").await {
            Ok(result) => result.success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tendhost_exec::LocalExecutor;

    #[test]
    fn test_compose_cmd_v2() {
        let manager = DockerComposeManager::new(
            Arc::new(LocalExecutor::new()),
            vec![PathBuf::from("/opt/stacks/monitoring")],
        )
        .unwrap();

        let cmd = manager.compose_cmd(&PathBuf::from("/opt/stacks/monitoring"), "up -d");
        assert!(cmd.contains("docker compose"));
        assert!(cmd.contains("/opt/stacks/monitoring/docker-compose.yml"));
    }
}

//! APT package manager (Debian/Ubuntu)

use std::sync::Arc;

use async_trait::async_trait;
use tendhost_exec::traits::RemoteExecutor;
use tracing::{debug, info, instrument};

use crate::error::PackageError;
use crate::traits::PackageManager;
use crate::types::{PackageManagerType, UpdateResult, UpgradablePackage};

/// APT package manager implementation
pub struct AptManager {
    /// Remote executor for running commands
    executor: Arc<dyn RemoteExecutor>,
    /// Whether to use sudo
    use_sudo: bool,
}

impl AptManager {
    /// Create a new APT manager
    ///
    /// # Arguments
    /// * `executor` - Remote executor for running apt commands
    /// * `use_sudo` - Whether to prefix commands with sudo
    pub fn new(executor: Arc<dyn RemoteExecutor>, use_sudo: bool) -> Self {
        Self { executor, use_sudo }
    }

    /// Build apt command with optional sudo
    fn apt_cmd(&self, args: &str) -> String {
        if self.use_sudo {
            format!("sudo apt {args}")
        } else {
            format!("apt {args}")
        }
    }

    /// Parse apt list --upgradable output
    fn parse_upgradable(output: &str) -> Vec<UpgradablePackage> {
        let mut packages = Vec::new();

        for line in output.lines() {
            // Skip header lines and empty lines
            if line.is_empty() || line.starts_with("Listing") || line.starts_with("WARNING") {
                continue;
            }

            // Parse: package/arch version repository [upgradable from: oldversion]
            // Example: vim/now 2:8.2.2434-3+deb11u1 amd64 [upgradable from: 2:8.2.2434-3]
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name_arch = parts[0];
                let new_version = parts[1];

                // Split name/arch
                let (name, arch) = if let Some(idx) = name_arch.find('/') {
                    (&name_arch[..idx], Some(&name_arch[idx + 1..]))
                } else {
                    (name_arch, None)
                };

                // Extract old version from [...]
                let current_version = if let Some(start) = line.find("[upgradable from: ") {
                    let start = start + "[upgradable from: ".len();
                    if let Some(end) = line[start..].find(']') {
                        line[start..start + end].to_string()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };

                let mut pkg = UpgradablePackage::new(name, current_version, new_version);
                if let Some(a) = arch {
                    pkg = pkg.with_arch(a);
                }
                packages.push(pkg);
            }
        }

        packages
    }

    /// Parse apt upgrade output for results
    fn parse_upgrade_output(_stdout: &str, stderr: &str) -> UpdateResult {
        let mut upgraded = 0u32;
        let mut new_pkgs = 0u32;
        let mut removed = 0u32;

        // Look for summary line in stderr
        for line in stderr.lines() {
            if line.contains("upgraded,") {
                // Parse: "X upgraded, Y newly installed, Z to remove"
                let parts: Vec<&str> = line.split(',').collect();
                for part in parts {
                    let part = part.trim();
                    if let Some(n) = part.find(" upgraded")
                        && let Ok(num) = part[..n].trim().parse::<u32>()
                    {
                        upgraded = num;
                    }
                    if let Some(n) = part.find(" newly installed")
                        && let Ok(num) = part[..n].trim().parse::<u32>()
                    {
                        new_pkgs = num;
                    }
                    if let Some(n) = part.find(" to remove")
                        && let Ok(num) = part[..n].trim().parse::<u32>()
                    {
                        removed = num;
                    }
                }
            }
        }

        UpdateResult {
            success: true,
            upgraded_count: upgraded,
            new_count: new_pkgs,
            removed_count: removed,
            reboot_required: false, // Will check separately
            upgraded_packages: Vec::new(),
            error: None,
        }
    }
}

#[async_trait]
impl PackageManager for AptManager {
    #[instrument(skip(self))]
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, PackageError> {
        debug!("listing upgradable packages");

        // First update package lists
        let update_cmd = self.apt_cmd("update -qq");
        let update_result = self
            .executor
            .run(&update_cmd)
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !update_result.success() {
            return Err(PackageError::RepositoryUnavailable(
                update_result.stderr.clone(),
            ));
        }

        // List upgradable packages
        let cmd = self.apt_cmd("list --upgradable");
        let result = self
            .executor
            .run(&cmd)
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !result.success() {
            return Err(PackageError::CommandFailed {
                status: result.status,
                message: result.stderr,
            });
        }

        let packages = Self::parse_upgradable(&result.stdout);
        info!(count = packages.len(), "found upgradable packages");

        Ok(packages)
    }

    #[instrument(skip(self))]
    async fn upgrade_all(&self) -> Result<UpdateResult, PackageError> {
        info!("starting apt upgrade");

        let cmd = self.apt_cmd("upgrade -y");
        let result = self
            .executor
            .run(&cmd)
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !result.success() {
            // Check for lock conflict
            if result.stderr.contains("Could not get lock") {
                return Err(PackageError::LockConflict(result.stderr));
            }
            // Check for permission denied
            if result.stderr.contains("Permission denied") {
                return Err(PackageError::PermissionDenied(result.stderr));
            }

            return Err(PackageError::CommandFailed {
                status: result.status,
                message: result.stderr.clone(),
            });
        }

        let mut update_result = Self::parse_upgrade_output(&result.stdout, &result.stderr);

        // Check if reboot is required
        update_result.reboot_required = self.reboot_required().await.unwrap_or(false);

        info!(
            upgraded = update_result.upgraded_count,
            reboot_required = update_result.reboot_required,
            "apt upgrade completed"
        );

        Ok(update_result)
    }

    #[instrument(skip(self))]
    async fn upgrade_dry_run(&self) -> Result<UpdateResult, PackageError> {
        debug!("starting apt dry run");

        let cmd = self.apt_cmd("upgrade --simulate");
        let result = self
            .executor
            .run(&cmd)
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !result.success() {
            return Err(PackageError::CommandFailed {
                status: result.status,
                message: result.stderr,
            });
        }

        let update_result = Self::parse_upgrade_output(&result.stdout, &result.stderr);

        Ok(update_result)
    }

    #[instrument(skip(self))]
    async fn reboot_required(&self) -> Result<bool, PackageError> {
        // Check for /var/run/reboot-required (Debian/Ubuntu standard)
        let result = self
            .executor
            .run("test -f /var/run/reboot-required")
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        Ok(result.success())
    }

    fn manager_type(&self) -> PackageManagerType {
        PackageManagerType::Apt
    }

    async fn is_available(&self) -> bool {
        // Check if apt command exists
        match self.executor.run("which apt").await {
            Ok(result) => result.success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_upgradable() {
        let output = r"Listing... Done
vim/now 2:8.2.2434-3+deb11u1 amd64 [upgradable from: 2:8.2.2434-3]
curl/stable 7.74.0-1.3+deb11u14 amd64 [upgradable from: 7.74.0-1.3+deb11u7]";

        let packages = AptManager::parse_upgradable(output);

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "vim");
        assert_eq!(packages[0].new_version, "2:8.2.2434-3+deb11u1");
        assert_eq!(packages[0].current_version, "2:8.2.2434-3");
    }

    #[test]
    fn test_parse_upgrade_output() {
        let stderr = "5 upgraded, 2 newly installed, 1 to remove and 0 not upgraded";

        let result = AptManager::parse_upgrade_output("", stderr);

        assert_eq!(result.upgraded_count, 5);
        assert_eq!(result.new_count, 2);
        assert_eq!(result.removed_count, 1);
    }
}

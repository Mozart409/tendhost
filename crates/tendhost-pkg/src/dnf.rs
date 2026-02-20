//! DNF package manager (Fedora/RHEL/CentOS)

use std::sync::Arc;

use async_trait::async_trait;
use tendhost_exec::traits::RemoteExecutor;
use tracing::{debug, info, instrument};

use crate::error::PackageError;
use crate::traits::PackageManager;
use crate::types::{PackageManagerType, UpdateResult, UpgradablePackage};

/// DNF package manager implementation
///
/// Falls back to `yum` if `dnf` is not available.
pub struct DnfManager {
    executor: Arc<dyn RemoteExecutor>,
    use_sudo: bool,
    /// Whether to use yum instead of dnf
    use_yum: bool,
}

impl DnfManager {
    /// Create a new DNF manager
    pub fn new(executor: Arc<dyn RemoteExecutor>, use_sudo: bool) -> Self {
        Self {
            executor,
            use_sudo,
            use_yum: false,
        }
    }

    /// Detect whether to use dnf or yum
    #[allow(dead_code)]
    async fn detect_tool(&mut self) -> Result<(), PackageError> {
        // Check if dnf exists
        let has_dnf = self
            .executor
            .run("which dnf")
            .await
            .map(|r| r.success())
            .unwrap_or(false);

        // Check if yum exists
        let has_yum = self
            .executor
            .run("which yum")
            .await
            .map(|r| r.success())
            .unwrap_or(false);

        if has_dnf {
            self.use_yum = false;
        } else if has_yum {
            self.use_yum = true;
        } else {
            return Err(PackageError::ManagerNotFound(
                "neither dnf nor yum found".to_string(),
            ));
        }
        Ok(())
    }

    /// Build dnf/yum command with optional sudo
    fn pkg_cmd(&self, args: &str) -> String {
        let tool = if self.use_yum { "yum" } else { "dnf" };
        if self.use_sudo {
            format!("sudo {tool} {args}")
        } else {
            format!("{tool} {args}")
        }
    }

    /// Parse dnf check-update output
    fn parse_upgradable(output: &str) -> Vec<UpgradablePackage> {
        let mut packages = Vec::new();

        for line in output.lines() {
            // Skip empty lines and headers
            if line.is_empty() || line.starts_with("Last metadata") {
                continue;
            }

            // Parse: name.arch version repository
            // Example: vim-enhanced.x86_64 2:8.2.2637-20.el9_1 baseos
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name_arch = parts[0];
                let new_version = parts[1];
                let repository = parts[2];

                // Split name.arch
                let (name, arch) = if let Some(idx) = name_arch.rfind('.') {
                    (&name_arch[..idx], Some(&name_arch[idx + 1..]))
                } else {
                    (name_arch, None)
                };

                // DNF doesn't show current version in check-update
                let mut pkg = UpgradablePackage::new(name, "unknown", new_version);
                if let Some(a) = arch {
                    pkg = pkg.with_arch(a);
                }
                pkg = pkg.with_repository(repository);
                packages.push(pkg);
            }
        }

        packages
    }

    /// Parse update output
    fn parse_update_output(output: &str) -> UpdateResult {
        let mut upgraded = 0u32;

        // Look for "Complete!" or similar success indicator
        let success = output.contains("Complete!") || output.contains("Updated:");

        // Count "Updated:" lines
        for line in output.lines() {
            if line.starts_with("Updated:") || line.starts_with("Upgraded:") {
                upgraded += 1;
            }
        }

        UpdateResult {
            success,
            upgraded_count: upgraded,
            new_count: 0,
            removed_count: 0,
            reboot_required: false,
            upgraded_packages: Vec::new(),
            error: if success {
                None
            } else {
                Some(output.to_string())
            },
        }
    }
}

#[async_trait]
impl PackageManager for DnfManager {
    #[instrument(skip(self))]
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, PackageError> {
        debug!("listing upgradable packages");

        let cmd = self.pkg_cmd("check-update");
        let result = self
            .executor
            .run(&cmd)
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        // dnf check-update returns exit code 100 when updates are available
        // exit code 0 when no updates
        if result.status != 0 && result.status != 100 {
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
        info!("starting dnf update");

        let cmd = self.pkg_cmd("update -y");
        let result = self
            .executor
            .run(&cmd)
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !result.success() {
            if result.stderr.contains("lock") {
                return Err(PackageError::LockConflict(result.stderr));
            }
            return Err(PackageError::CommandFailed {
                status: result.status,
                message: result.stderr,
            });
        }

        let mut update_result = Self::parse_update_output(&result.stdout);
        update_result.reboot_required = self.reboot_required().await.unwrap_or(false);

        info!(
            upgraded = update_result.upgraded_count,
            reboot_required = update_result.reboot_required,
            "dnf update completed"
        );

        Ok(update_result)
    }

    #[instrument(skip(self))]
    async fn upgrade_dry_run(&self) -> Result<UpdateResult, PackageError> {
        debug!("starting dnf dry run");

        // dnf doesn't have a direct simulate flag like apt
        // Use --assumeno to simulate without installing
        let cmd = self.pkg_cmd("update --assumeno");
        let result = self
            .executor
            .run(&cmd)
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        // --assumeno will "fail" but show what would be done
        let update_result = Self::parse_update_output(&result.stdout);

        Ok(update_result)
    }

    #[instrument(skip(self))]
    async fn reboot_required(&self) -> Result<bool, PackageError> {
        // Check if needs-restarting exists and reports reboot needed
        let result = self
            .executor
            .run("needs-restarting -r")
            .await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        // needs-restarting -r exits 1 if reboot required, 0 if not
        Ok(!result.success())
    }

    fn manager_type(&self) -> PackageManagerType {
        PackageManagerType::Dnf
    }

    async fn is_available(&self) -> bool {
        // Check if dnf exists
        let has_dnf = self
            .executor
            .run("which dnf")
            .await
            .map(|r| r.success())
            .unwrap_or(false);

        // Check if yum exists
        let has_yum = self
            .executor
            .run("which yum")
            .await
            .map(|r| r.success())
            .unwrap_or(false);

        has_dnf || has_yum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_upgradable() {
        let output = r"Last metadata expiration check: 0:05:31 ago.
vim-enhanced.x86_64 2:8.2.2637-20.el9_1 baseos
curl.x86_64         7.76.1-26.el9_0 baseos";

        let packages = DnfManager::parse_upgradable(output);

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "vim-enhanced");
        assert_eq!(packages[0].new_version, "2:8.2.2637-20.el9_1");
    }
}

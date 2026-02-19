//! Package manager trait and implementations

use async_trait::async_trait;

use crate::error::PackageError;
use crate::types::{UpdateResult, UpgradablePackage};

/// Trait for package management operations
///
/// Implementations use a `RemoteExecutor` to run commands on the target system.
#[async_trait]
pub trait PackageManager: Send + Sync {
    /// List packages with available upgrades
    ///
    /// # Returns
    /// * `Ok(Vec<UpgradablePackage>)` - List of upgradable packages
    /// * `Err(PackageError)` - Failed to query packages
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, PackageError>;

    /// Upgrade all packages
    ///
    /// # Returns
    /// * `Ok(UpdateResult)` - Update completed successfully
    /// * `Err(PackageError)` - Update failed
    async fn upgrade_all(&self) -> Result<UpdateResult, PackageError>;

    /// Simulate upgrade (dry run)
    ///
    /// Shows what would be upgraded without making changes.
    ///
    /// # Returns
    /// * `Ok(UpdateResult)` - Simulated update result
    /// * `Err(PackageError)` - Failed to simulate
    async fn upgrade_dry_run(&self) -> Result<UpdateResult, PackageError>;

    /// Check if reboot is required after updates
    ///
    /// # Returns
    /// * `Ok(true)` - Reboot required (kernel or critical services updated)
    /// * `Ok(false)` - No reboot required
    /// * `Err(PackageError)` - Failed to check
    async fn reboot_required(&self) -> Result<bool, PackageError>;

    /// Get package manager type
    fn manager_type(&self) -> crate::types::PackageManagerType;

    /// Check if package manager is available on the system
    async fn is_available(&self) -> bool;
}

/// Extension trait for package manager utilities
#[async_trait]
pub trait PackageManagerExt: PackageManager {
    /// Check if any updates are available
    async fn has_updates(&self) -> Result<bool, PackageError> {
        let packages = self.list_upgradable().await?;
        Ok(!packages.is_empty())
    }

    /// Get count of upgradable packages
    async fn upgrade_count(&self) -> Result<u32, PackageError> {
        let packages = self.list_upgradable().await?;
        u32::try_from(packages.len()).map_err(|e| PackageError::ParseError(e.to_string()))
    }

    /// Update package lists (apt update, dnf makecache)
    async fn update_package_lists(&self) -> Result<(), PackageError>;
}

#[async_trait]
impl<T: PackageManager> PackageManagerExt for T {
    async fn update_package_lists(&self) -> Result<(), PackageError> {
        // Default: no-op (override in implementations)
        Ok(())
    }
}

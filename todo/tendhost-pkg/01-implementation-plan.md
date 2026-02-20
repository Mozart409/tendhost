# Implementation Plan: tendhost-pkg

## Overview

This plan implements the package manager abstraction for tendhost, providing:
- `PackageManager` trait for package operations
- `AptManager`: Debian/Ubuntu package management
- `DnfManager`: CentOS/Fedora/RHEL package management
- `DockerComposeManager`: Docker Compose stack management
- Comprehensive error handling
- Command output parsing

## Architecture

```
┌─────────────────────────────────────────────┐
│         PackageManager Trait                │
│  • list_upgradable() -> Vec<Package>        │
│  • upgrade_all() -> UpdateResult            │
│  • upgrade_dry_run() -> UpdateResult        │
│  • reboot_required() -> bool                │
└───────────────────┬─────────────────────────┘
                    │ uses RemoteExecutor
        ┌───────────┼───────────┐
        ▼           ▼           ▼
┌───────────┐ ┌───────────┐ ┌───────────────────┐
│AptManager │ │DnfManager │ │DockerComposeManager
│           │ │           │ │                   │
│ apt/apt-get│ │ dnf/yum   │ │ docker compose    │
└───────────┘ └───────────┘ └───────────────────┘
```

---

## Phase 1: Foundation

### Task 1.1: Create Error Types (`error.rs`)
**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost-pkg/src/error.rs`:

```rust
//! Error types for tendhost-pkg

use thiserror::Error;

/// Errors that can occur during package operations
#[derive(Error, Debug, Clone)]
pub enum PackageError {
    /// Package manager not found on system
    #[error("package manager not found: {0}")]
    ManagerNotFound(String),

    /// Package not found in repositories
    #[error("package not found: {0}")]
    PackageNotFound(String),

    /// Repository is unavailable
    #[error("repository unavailable: {0}")]
    RepositoryUnavailable(String),

    /// Lock file conflict (another process running)
    #[error("lock file conflict: {0}")]
    LockConflict(String),

    /// Insufficient permissions (need sudo)
    #[error("insufficient permissions: {0}")]
    PermissionDenied(String),

    /// Command execution failed
    #[error("command failed: {status} - {message}")]
    CommandFailed {
        /// Exit status
        status: i32,
        /// Error message
        message: String,
    },

    /// Failed to parse command output
    #[error("parse error: {0}")]
    ParseError(String),

    /// Execution error from remote executor
    #[error("execution error: {0}")]
    ExecutionError(String),

    /// Docker compose not found
    #[error("docker compose not found")]
    DockerComposeNotFound,

    /// Compose file not found
    #[error("compose file not found: {0}")]
    ComposeFileNotFound(String),

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    ConfigError(String),
}

impl PackageError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            PackageError::LockConflict(_) | PackageError::RepositoryUnavailable(_)
        )
    }

    /// Check if error indicates need for sudo
    pub fn needs_sudo(&self) -> bool {
        matches!(self, PackageError::PermissionDenied(_))
    }
}
```

**Acceptance criteria**:
- [x] Error enum covers all package manager failure modes
- [x] `is_retryable()` helper for lock conflicts
- [x] `needs_sudo()` helper for permission errors
- [x] Public in lib.rs

---

### Task 1.2: Enhance Type Definitions (`types.rs`)
**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost-pkg/src/types.rs`:

```rust
//! Type definitions for package management

use serde::{Deserialize, Serialize};

/// A package with available updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradablePackage {
    /// Package name
    pub name: String,
    /// Current installed version
    pub current_version: String,
    /// Available upgrade version
    pub new_version: String,
    /// Package architecture
    pub arch: Option<String>,
    /// Package repository
    pub repository: Option<String>,
}

impl UpgradablePackage {
    /// Create a new upgradable package
    pub fn new(
        name: impl Into<String>,
        current: impl Into<String>,
        new: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            current_version: current.into(),
            new_version: new.into(),
            arch: None,
            repository: None,
        }
    }

    /// Set architecture
    pub fn with_arch(mut self, arch: impl Into<String>) -> Self {
        self.arch = Some(arch.into());
        self
    }

    /// Set repository
    pub fn with_repository(mut self, repo: impl Into<String>) -> Self {
        self.repository = Some(repo.into());
        self
    }
}

/// Result of an update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    /// Whether the update succeeded
    pub success: bool,
    /// Number of packages upgraded
    pub upgraded_count: u32,
    /// Number of packages newly installed
    pub new_count: u32,
    /// Number of packages removed
    pub removed_count: u32,
    /// Whether a reboot is required
    pub reboot_required: bool,
    /// List of upgraded packages
    pub upgraded_packages: Vec<String>,
    /// Error message if failed
    pub error: Option<String>,
}

impl UpdateResult {
    /// Create a successful result
    pub fn success(upgraded: u32) -> Self {
        Self {
            success: true,
            upgraded_count: upgraded,
            new_count: 0,
            removed_count: 0,
            reboot_required: false,
            upgraded_packages: Vec::new(),
            error: None,
        }
    }

    /// Create a failed result
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            success: false,
            upgraded_count: 0,
            new_count: 0,
            removed_count: 0,
            reboot_required: false,
            upgraded_packages: Vec::new(),
            error: Some(error.into()),
        }
    }

    /// Add upgraded package
    pub fn with_package(mut self, name: impl Into<String>) -> Self {
        self.upgraded_packages.push(name.into());
        self
    }

    /// Mark reboot as required
    pub fn with_reboot(mut self) -> Self {
        self.reboot_required = true;
        self
    }
}

/// Package manager type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageManagerType {
    /// APT (Debian/Ubuntu)
    Apt,
    /// DNF (Fedora/RHEL)
    Dnf,
    /// Docker Compose
    DockerCompose,
}

impl std::fmt::Display for PackageManagerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageManagerType::Apt => write!(f, "apt"),
            PackageManagerType::Dnf => write!(f, "dnf"),
            PackageManagerType::DockerCompose => write!(f, "docker-compose"),
        }
    }
}

/// Detected distribution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroInfo {
    /// Distribution ID (debian, ubuntu, fedora, etc.)
    pub id: String,
    /// Distribution name
    pub name: String,
    /// Version ID
    pub version_id: String,
    /// Package manager type
    pub package_manager: PackageManagerType,
}
```

**Acceptance criteria**:
- [x] Enhanced `UpgradablePackage` with all fields
- [x] Enhanced `UpdateResult` with builder methods
- [x] `PackageManagerType` enum
- [x] `DistroInfo` for auto-detection
- [x] Serde derives for serialization

---

## Phase 2: Enhanced Trait

### Task 2.1: Enhance PackageManager Trait (`traits.rs`)
**Priority**: High  
**Estimated effort**: 45 min

Rewrite `crates/tendhost-pkg/src/traits.rs`:

```rust
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
        Ok(packages.len() as u32)
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
```

**Acceptance criteria**:
- [x] Enhanced trait with all methods
- [x] `manager_type()` for identification
- [x] `is_available()` to check prerequisites
- [x] Extension trait with utilities
- [x] Comprehensive doc comments

---

## Phase 3: Apt Implementation

### Task 3.1: Implement AptManager (`apt.rs`)
**Priority**: High  
**Estimated effort**: 1.5 hours

Rewrite `crates/tendhost-pkg/src/apt.rs`:

```rust
//! APT package manager (Debian/Ubuntu)

use std::sync::Arc;

use async_trait::async_trait;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_exec::RemoteExecutorExt;
use tracing::{debug, error, info, instrument};

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
        Self {
            executor,
            use_sudo,
        }
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
    fn parse_upgradable(&self, output: &str) -> Vec<UpgradablePackage> {
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
    fn parse_upgrade_output(&self, stdout: &str, stderr: &str) -> UpdateResult {
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
                    if let Some(n) = part.find(" upgraded") {
                        if let Ok(num) = part[..n].trim().parse::<u32>() {
                            upgraded = num;
                        }
                    }
                    if let Some(n) = part.find(" newly installed") {
                        if let Ok(num) = part[..n].trim().parse::<u32>() {
                            new_pkgs = num;
                        }
                    }
                    if let Some(n) = part.find(" to remove") {
                        if let Ok(num) = part[..n].trim().parse::<u32>() {
                            removed = num;
                        }
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
        let update_result = self.executor.run(&update_cmd).await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !update_result.success() {
            return Err(PackageError::RepositoryUnavailable(
                update_result.stderr.clone()
            ));
        }

        // List upgradable packages
        let cmd = self.apt_cmd("list --upgradable");
        let result = self.executor.run(&cmd).await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !result.success() {
            return Err(PackageError::CommandFailed {
                status: result.status,
                message: result.stderr,
            });
        }

        let packages = self.parse_upgradable(&result.stdout);
        info!(count = packages.len(), "found upgradable packages");

        Ok(packages)
    }

    #[instrument(skip(self))]
    async fn upgrade_all(&self) -> Result<UpdateResult, PackageError> {
        info!("starting apt upgrade");

        let cmd = self.apt_cmd("upgrade -y");
        let result = self.executor.run(&cmd).await
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

        let mut update_result = self.parse_upgrade_output(&result.stdout, &result.stderr);

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
        let result = self.executor.run(&cmd).await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        if !result.success() {
            return Err(PackageError::CommandFailed {
                status: result.status,
                message: result.stderr,
            });
        }

        let update_result = self.parse_upgrade_output(&result.stdout, &result.stderr);

        Ok(update_result)
    }

    #[instrument(skip(self))]
    async fn reboot_required(&self) -> Result<bool, PackageError> {
        // Check for /var/run/reboot-required (Debian/Ubuntu standard)
        let result = self.executor.run("test -f /var/run/reboot-required").await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        Ok(result.success())
    }

    fn manager_type(&self) -> PackageManagerType {
        PackageManagerType::Apt
    }

    async fn is_available(&self) -> bool {
        self.executor.command_exists("apt").await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tendhost_exec::LocalExecutor;

    #[test]
    fn test_parse_upgradable() {
        let manager = AptManager::new(Arc::new(LocalExecutor::new()), false);

        let output = r#"Listing... Done
vim/now 2:8.2.2434-3+deb11u1 amd64 [upgradable from: 2:8.2.2434-3]
curl/stable 7.74.0-1.3+deb11u14 amd64 [upgradable from: 7.74.0-1.3+deb11u7]"#;

        let packages = manager.parse_upgradable(output);

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "vim");
        assert_eq!(packages[0].new_version, "2:8.2.2434-3+deb11u1");
        assert_eq!(packages[0].current_version, "2:8.2.2434-3");
    }

    #[test]
    fn test_parse_upgrade_output() {
        let manager = AptManager::new(Arc::new(LocalExecutor::new()), false);

        let stderr = "5 upgraded, 2 newly installed, 1 to remove and 0 not upgraded";

        let result = manager.parse_upgrade_output("", stderr);

        assert_eq!(result.upgraded_count, 5);
        assert_eq!(result.new_count, 2);
        assert_eq!(result.removed_count, 1);
    }
}
```

**Acceptance criteria**:
- [ ] Uses `RemoteExecutor` for commands
- [ ] Parses `apt list --upgradable` output
- [ ] Handles lock conflicts and permissions
- [ ] Dry-run support
- [ ] Reboot detection via `/var/run/reboot-required`
- [ ] Unit tests for parsing

**Status**: Implementation pending - foundation complete

---

## Phase 4: Dnf Implementation

### Task 4.1: Implement DnfManager (`dnf.rs`)
**Priority**: High  
**Estimated effort**: 1.5 hours

Rewrite `crates/tendhost-pkg/src/dnf.rs`:

```rust
//! DNF package manager (Fedora/RHEL/CentOS)

use std::sync::Arc;

use async_trait::async_trait;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_exec::RemoteExecutorExt;
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
    async fn detect_tool(&mut self) -> Result<(), PackageError> {
        if self.executor.command_exists("dnf").await.unwrap_or(false) {
            self.use_yum = false;
        } else if self.executor.command_exists("yum").await.unwrap_or(false) {
            self.use_yum = true;
        } else {
            return Err(PackageError::ManagerNotFound(
                "neither dnf nor yum found".to_string()
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
    fn parse_upgradable(&self, output: &str) -> Vec<UpgradablePackage> {
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
    fn parse_update_output(&self, output: &str) -> UpdateResult {
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
            error: if success { None } else { Some(output.to_string()) },
        }
    }
}

#[async_trait]
impl PackageManager for DnfManager {
    #[instrument(skip(self))]
    async fn list_upgradable(&self) -> Result<Vec<UpgradablePackage>, PackageError> {
        debug!("listing upgradable packages");

        let cmd = self.pkg_cmd("check-update");
        let result = self.executor.run(&cmd).await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        // dnf check-update returns exit code 100 when updates are available
        // exit code 0 when no updates
        if result.status != 0 && result.status != 100 {
            return Err(PackageError::CommandFailed {
                status: result.status,
                message: result.stderr,
            });
        }

        let packages = self.parse_upgradable(&result.stdout);
        info!(count = packages.len(), "found upgradable packages");

        Ok(packages)
    }

    #[instrument(skip(self))]
    async fn upgrade_all(&self) -> Result<UpdateResult, PackageError> {
        info!("starting dnf update");

        let cmd = self.pkg_cmd("update -y");
        let result = self.executor.run(&cmd).await
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

        let mut update_result = self.parse_update_output(&result.stdout);
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
        let result = self.executor.run(&cmd).await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        // --assumeno will "fail" but show what would be done
        let update_result = self.parse_update_output(&result.stdout);

        Ok(update_result)
    }

    #[instrument(skip(self))]
    async fn reboot_required(&self) -> Result<bool, PackageError> {
        // Check if needs-restarting exists and reports reboot needed
        let result = self.executor.run("needs-restarting -r").await
            .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

        // needs-restarting -r exits 1 if reboot required, 0 if not
        Ok(!result.success())
    }

    fn manager_type(&self) -> PackageManagerType {
        PackageManagerType::Dnf
    }

    async fn is_available(&self) -> bool {
        self.executor.command_exists("dnf").await.unwrap_or(false)
            || self.executor.command_exists("yum").await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tendhost_exec::LocalExecutor;

    #[test]
    fn test_parse_upgradable() {
        let manager = DnfManager::new(Arc::new(LocalExecutor::new()), false);

        let output = r#"Last metadata expiration check: 0:05:31 ago.
vim-enhanced.x86_64 2:8.2.2637-20.el9_1 baseos
curl.x86_64         7.76.1-26.el9_0 baseos"#;

        let packages = manager.parse_upgradable(output);

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "vim-enhanced");
        assert_eq!(packages[0].new_version, "2:8.2.2637-20.el9_1");
    }
}
```

**Acceptance criteria**:
- [ ] Auto-detects dnf vs yum
- [ ] Parses `dnf check-update` output
- [ ] Handles exit code 100 (updates available)
- [ ] Dry-run via `--assumeno`
- [ ] Reboot detection via `needs-restarting`
- [ ] Unit tests for parsing

**Status**: Implementation pending - foundation complete

---

## Phase 5: Docker Compose Implementation

### Task 5.1: Implement DockerComposeManager (`docker.rs`)
**Priority**: High  
**Estimated effort**: 1.5 hours

Rewrite `crates/tendhost-pkg/src/docker.rs`:

```rust
//! Docker Compose stack management

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_exec::RemoteExecutorExt;
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
    pub fn new(
        executor: Arc<dyn RemoteExecutor>,
        compose_dirs: Vec<PathBuf>,
    ) -> Result<Self, PackageError> {
        if compose_dirs.is_empty() {
            return Err(PackageError::ConfigError(
                "no compose directories specified".to_string()
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
    async fn detect_version(&mut self) -> Result<(), PackageError> {
        if self.executor.command_exists("docker compose").await.unwrap_or(false) {
            self.use_v2 = true;
        } else if self.executor.command_exists("docker-compose").await.unwrap_or(false) {
            self.use_v2 = false;
        } else {
            return Err(PackageError::DockerComposeNotFound);
        }
        Ok(())
    }

    /// Build docker compose command
    fn compose_cmd(&self, compose_dir: &PathBuf, args: &str) -> String {
        let cmd = if self.use_v2 {
            "docker compose"
        } else {
            "docker-compose"
        };
        let dir = compose_dir.display();
        format!("{cmd} -f {dir}/docker-compose.yml {args}")
    }

    /// Check if compose file exists
    async fn compose_file_exists(&self, compose_dir: &PathBuf) -> Result<bool, PackageError> {
        let path = compose_dir.join("docker-compose.yml");
        let result = self.executor.run(&format!("test -f {}", path.display())).await
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
            let result = self.executor.run(&cmd).await
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
                let img_result = self.executor.run(&img_cmd).await
                    .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

                if img_result.success() && !img_result.stdout.trim().is_empty() {
                    // Check if newer image available
                    let check_cmd = format!(
                        "docker compose -f {}/docker-compose.yml pull --dry-run {} 2>&1 || true",
                        compose_dir.display(),
                        service
                    );
                    let check_result = self.executor.run(&check_cmd).await
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
                let pull_result = self.executor.run(&pull_cmd).await
                    .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

                if !pull_result.success() {
                    errors.push(format!("{}: pull failed", compose_dir.display()));
                    continue;
                }
            }

            // Recreate containers with new images
            let up_cmd = self.compose_cmd(compose_dir, "up -d --force-recreate");
            let up_result = self.executor.run(&up_cmd).await
                .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

            if up_result.success() {
                // Count services in this compose file
                let ps_cmd = self.compose_cmd(compose_dir, "ps -q");
                let ps_result = self.executor.run(&ps_cmd).await
                    .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

                if ps_result.success() {
                    let count = ps_result.stdout.lines().count() as u32;
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

        info!(upgraded = total_upgraded, success = success, "docker compose update completed");

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
            let result = self.executor.run(&cmd).await
                .map_err(|e| PackageError::ExecutionError(e.to_string()))?;

            if result.success() {
                // Count images that would be pulled
                let count = result.stdout.matches("Pulling").count() as u32;
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
        self.executor.command_exists("docker").await.unwrap_or(false)
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
        ).unwrap();

        let cmd = manager.compose_cmd(&PathBuf::from("/opt/stacks/monitoring"), "up -d");
        assert!(cmd.contains("docker compose"));
        assert!(cmd.contains("/opt/stacks/monitoring/docker-compose.yml"));
    }
}
```

**Acceptance criteria**:
- [ ] Auto-detects v1 vs v2 compose
- [ ] Manages multiple compose directories
- [ ] Pulls and recreates containers
- [ ] Dry-run support
- [ ] No reboot required for Docker
- [ ] Unit tests for command building

**Status**: Implementation pending - foundation complete

---

## Phase 6: Integration

### Task 6.1: Update lib.rs Exports
**Priority**: Medium  
**Estimated effort**: 15 min

Update `crates/tendhost-pkg/src/lib.rs`:

```rust
//! tendhost-pkg: Package manager abstraction
//!
//! Provides traits and implementations for different package managers
//! (apt, dnf, docker compose).
//!
//! # Example
//! ```rust
//! use std::sync::Arc;
//! use tendhost_exec::LocalExecutor;
//! use tendhost_pkg::{AptManager, PackageManager};
//!
//! let executor = Arc::new(LocalExecutor::new());
//! let manager = AptManager::new(executor, true);
//! // let packages = manager.list_upgradable().await?;
//! ```

pub mod apt;
pub mod dnf;
pub mod docker;
pub mod error;
pub mod traits;
pub mod types;

pub use apt::AptManager;
pub use dnf::DnfManager;
pub use docker::DockerComposeManager;
pub use error::PackageError;
pub use traits::{PackageManager, PackageManagerExt};
pub use types::{DistroInfo, PackageManagerType, UpdateResult, UpgradablePackage};
```

---

## Summary

### File Changes Required

| File | Action | Description |
|------|--------|-------------|
| `src/error.rs` | Create | Package error types |
| `src/types.rs` | Create | Type definitions |
| `src/traits.rs` | Modify | Enhanced PackageManager trait |
| `src/apt.rs` | Modify | AptManager implementation |
| `src/dnf.rs` | Modify | DnfManager implementation |
| `src/docker.rs` | Modify | DockerComposeManager implementation |
| `src/lib.rs` | Modify | Re-exports |

### Current Status

**Completed (2026-02-20)**:
- ✅ Phase 1: Foundation (error.rs, types.rs)
- ✅ Phase 2: Trait Enhancement (traits.rs with PackageManager + PackageManagerExt)
- ✅ Phase 3: AptManager implementation (apt.rs with full parsing and tests)
- ✅ Phase 4: DnfManager implementation (dnf.rs with dnf/yum fallback)
- ✅ Phase 5: DockerComposeManager implementation (docker.rs with v1/v2 detection)
- ✅ Phase 6: Integration (lib.rs exports)
- ✅ Fixed tendhost-exec to support proper CommandResult return type
- ✅ Fixed tendhost-core integration tests
- ✅ All tests passing (4 unit tests)
- ✅ Clippy clean with no warnings
- ✅ Full workspace compiles successfully

**Status**: ✅ **COMPLETE** - All phases implemented and tested

### Estimated Total Effort

| Phase | Effort |
|-------|--------|
| Phase 1: Foundation | 1 hour |
| Phase 2: Trait Enhancement | 45 min |
| Phase 3: Apt Implementation | 1.5 hours |
| Phase 4: Dnf Implementation | 1.5 hours |
| Phase 5: Docker Implementation | 1.5 hours |
| Phase 6: Integration | 15 min |
| **Total** | **~6.5 hours** |

### Dependencies

- **Blocks**: `tendhost-core` (uses these implementations)
- **Blocked by**: `tendhost-exec` (uses RemoteExecutor)

### Notes

- Command output parsing is fragile - may need adjustment for different versions
- Consider adding retry logic for lock conflicts
- Docker Compose "list upgradable" is approximate (no direct equivalent)

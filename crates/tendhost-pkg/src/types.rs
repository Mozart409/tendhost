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
    #[must_use]
    pub fn with_arch(mut self, arch: impl Into<String>) -> Self {
        self.arch = Some(arch.into());
        self
    }

    /// Set repository
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn with_package(mut self, name: impl Into<String>) -> Self {
        self.upgraded_packages.push(name.into());
        self
    }

    /// Mark reboot as required
    #[must_use]
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

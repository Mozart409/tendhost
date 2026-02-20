//! Inventory type definitions

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// System Information
// ============================================================================

/// Operating system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Hostname
    pub hostname: String,
    /// OS name (Debian, Ubuntu, Fedora, etc.)
    pub os_name: String,
    /// OS version
    pub os_version: String,
    /// OS codename (if applicable)
    pub os_codename: Option<String>,
    /// Platform (Linux, Darwin, Windows)
    pub platform: String,
    /// Architecture (`x86_64`, `arm64`, etc.)
    pub arch: String,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// System UUID
    pub uuid: Option<String>,
    /// Kernel version
    pub kernel_version: String,
    /// When this data was collected
    pub collected_at: DateTime<Utc>,
}

/// Hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// CPU information
    pub cpu: CpuInfo,
    /// Memory information
    pub memory: MemoryInfo,
    /// Disk information
    pub disks: Vec<DiskInfo>,
    /// Network interfaces
    pub network_interfaces: Vec<NetworkInterface>,
}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU model name
    pub model: String,
    /// Number of physical cores
    pub physical_cores: u32,
    /// Number of logical cores (with hyperthreading)
    pub logical_cores: u32,
    /// CPU speed in MHz
    pub speed_mhz: u32,
    /// Vendor
    pub vendor: String,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total memory in bytes
    pub total_bytes: u64,
    /// Free memory in bytes
    pub free_bytes: u64,
    /// Used memory in bytes
    pub used_bytes: u64,
    /// Total swap in bytes
    pub swap_total_bytes: u64,
    /// Free swap in bytes
    pub swap_free_bytes: u64,
}

/// Disk information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// Device name
    pub device: String,
    /// Mount point
    pub mount_point: String,
    /// Filesystem type
    pub filesystem: String,
    /// Total size in bytes
    pub total_bytes: u64,
    /// Free space in bytes
    pub free_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
}

/// Network interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// MAC address
    pub mac: String,
    /// IPv4 addresses
    pub ipv4: Vec<String>,
    /// IPv6 addresses
    pub ipv6: Vec<String>,
}

// ============================================================================
// Packages
// ============================================================================

/// Software package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package architecture
    pub arch: String,
    /// Package source (apt, dnf, etc.)
    pub source: PackageSource,
    /// Install time
    pub install_time: Option<DateTime<Utc>>,
    /// Package size in bytes
    pub size_bytes: Option<u64>,
}

/// Package source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageSource {
    /// Debian/Ubuntu package
    Deb,
    /// RPM package
    Rpm,
    /// Python package
    Python,
    /// Node.js package
    Npm,
    /// Other/unknown
    Other,
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageSource::Deb => write!(f, "deb"),
            PackageSource::Rpm => write!(f, "rpm"),
            PackageSource::Python => write!(f, "python"),
            PackageSource::Npm => write!(f, "npm"),
            PackageSource::Other => write!(f, "other"),
        }
    }
}

// ============================================================================
// Docker
// ============================================================================

/// Docker container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    /// Container ID (short)
    pub id: String,
    /// Container name
    pub name: String,
    /// Image name
    pub image: String,
    /// Container state (running, exited, etc.)
    pub state: String,
    /// Status message
    pub status: String,
    /// Created time
    pub created: DateTime<Utc>,
    /// Exposed ports
    pub ports: Vec<ContainerPort>,
    /// Mounts
    pub mounts: Vec<ContainerMount>,
}

/// Container port mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPort {
    /// Port number
    pub port: u16,
    /// Protocol (tcp/udp)
    pub protocol: String,
    /// Host port (if published)
    pub host_port: Option<u16>,
    /// Host IP (if published)
    pub host_ip: Option<String>,
}

/// Container mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMount {
    /// Source path on host
    pub source: String,
    /// Destination path in container
    pub destination: String,
    /// Read-only
    pub read_only: bool,
}

/// Docker image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// Image ID
    pub id: String,
    /// Repository tags
    pub tags: Vec<String>,
    /// Created time
    pub created: DateTime<Utc>,
    /// Size in bytes
    pub size_bytes: u64,
}

// ============================================================================
// Full Inventory
// ============================================================================

/// Complete host inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInventory {
    /// System information
    pub system: SystemInfo,
    /// Hardware information
    pub hardware: HardwareInfo,
    /// Installed packages
    pub packages: Vec<Package>,
    /// Docker containers (if applicable)
    pub docker_containers: Vec<Container>,
    /// Docker images (if applicable)
    pub docker_images: Vec<Image>,
    /// When inventory was collected
    pub collected_at: DateTime<Utc>,
    /// Inventory version/schema
    pub version: String,
}

impl HostInventory {
    /// Create new empty inventory
    #[must_use]
    pub fn new() -> Self {
        Self {
            system: SystemInfo {
                hostname: String::new(),
                os_name: String::new(),
                os_version: String::new(),
                os_codename: None,
                platform: String::new(),
                arch: String::new(),
                uptime_seconds: 0,
                uuid: None,
                kernel_version: String::new(),
                collected_at: Utc::now(),
            },
            hardware: HardwareInfo {
                cpu: CpuInfo {
                    model: String::new(),
                    physical_cores: 0,
                    logical_cores: 0,
                    speed_mhz: 0,
                    vendor: String::new(),
                },
                memory: MemoryInfo {
                    total_bytes: 0,
                    free_bytes: 0,
                    used_bytes: 0,
                    swap_total_bytes: 0,
                    swap_free_bytes: 0,
                },
                disks: Vec::new(),
                network_interfaces: Vec::new(),
            },
            packages: Vec::new(),
            docker_containers: Vec::new(),
            docker_images: Vec::new(),
            collected_at: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// Get package count by source
    #[must_use]
    pub fn package_count_by_source(&self) -> HashMap<PackageSource, usize> {
        let mut counts = HashMap::new();
        for pkg in &self.packages {
            *counts.entry(pkg.source).or_insert(0) += 1;
        }
        counts
    }

    /// Check if Docker is installed
    #[must_use]
    pub fn has_docker(&self) -> bool {
        !self.docker_containers.is_empty() || !self.docker_images.is_empty()
    }
}

impl Default for HostInventory {
    fn default() -> Self {
        Self::new()
    }
}

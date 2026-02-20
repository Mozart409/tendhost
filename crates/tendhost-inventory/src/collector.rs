//! High-level inventory collection API

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use tendhost_exec::traits::RemoteExecutor;
use tracing::{debug, info, instrument, warn};

use crate::error::InventoryError;
use crate::osquery::OsqueryClient;
use crate::query::queries;
use crate::types::{
    Container, CpuInfo, DiskInfo, HardwareInfo, HostInventory, Image, MemoryInfo, NetworkInterface,
    Package, PackageSource, SystemInfo,
};

/// Inventory collector
///
/// High-level API for collecting host inventory data.
pub struct InventoryCollector {
    client: OsqueryClient,
}

impl InventoryCollector {
    /// Create a new inventory collector
    pub fn new(executor: Arc<dyn RemoteExecutor>, cache_ttl: Duration) -> Self {
        Self {
            client: OsqueryClient::new(executor, cache_ttl),
        }
    }

    /// Set query timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.client = self.client.with_timeout(timeout);
        self
    }

    /// Collect full inventory
    ///
    /// # Errors
    /// Returns an error if inventory collection fails completely. Partial failures
    /// are logged as warnings and the collection continues.
    #[instrument(skip(self))]
    pub async fn collect_full(&self) -> Result<HostInventory, InventoryError> {
        info!("collecting full inventory");

        let mut inventory = HostInventory::new();

        // Collect system info
        match self.get_system_info().await {
            Ok(info) => inventory.system = info,
            Err(e) => warn!(error = %e, "failed to collect system info"),
        }

        // Collect hardware info
        match self.get_hardware_info().await {
            Ok(info) => inventory.hardware = info,
            Err(e) => warn!(error = %e, "failed to collect hardware info"),
        }

        // Collect packages
        match self.get_packages().await {
            Ok(packages) => inventory.packages = packages,
            Err(e) => warn!(error = %e, "failed to collect packages"),
        }

        // Collect Docker info (if available)
        match self.get_docker_containers().await {
            Ok(containers) => inventory.docker_containers = containers,
            Err(e) => debug!(error = %e, "docker containers not available"),
        }

        match self.get_docker_images().await {
            Ok(images) => inventory.docker_images = images,
            Err(e) => debug!(error = %e, "docker images not available"),
        }

        inventory.collected_at = Utc::now();

        info!("inventory collection completed");

        Ok(inventory)
    }

    /// Get system information
    ///
    /// # Errors
    /// Returns an error if osquery queries fail or if required data is missing.
    #[instrument(skip(self))]
    pub async fn get_system_info(&self) -> Result<SystemInfo, InventoryError> {
        debug!("collecting system info");

        // Get OS version
        #[derive(Deserialize)]
        struct OsVersionRow {
            name: String,
            version: String,
            codename: Option<String>,
            platform: String,
            arch: String,
        }

        let os_rows: Vec<OsVersionRow> = self.client.query(&queries::os_version()).await?;
        let os = os_rows
            .into_iter()
            .next()
            .ok_or_else(|| InventoryError::ParseError("no os_version data".to_string()))?;

        // Get hostname
        #[derive(Deserialize)]
        struct SystemInfoRow {
            hostname: String,
        }

        let sys_rows: Vec<SystemInfoRow> = self.client.query(&queries::system_info()).await?;
        let sys = sys_rows
            .into_iter()
            .next()
            .ok_or_else(|| InventoryError::ParseError("no system_info data".to_string()))?;

        // Get uptime
        #[derive(Deserialize)]
        struct UptimeRow {
            total_seconds: String,
        }

        let uptime_rows: Vec<UptimeRow> = self.client.query(&queries::uptime()).await?;
        let uptime_seconds = uptime_rows
            .into_iter()
            .next()
            .and_then(|r| r.total_seconds.parse().ok())
            .unwrap_or(0);

        // Get kernel version
        #[derive(Deserialize)]
        struct KernelRow {
            version: String,
        }

        let kernel_rows: Vec<KernelRow> = self.client.query(&queries::kernel_info()).await?;
        let kernel_version = kernel_rows
            .into_iter()
            .next()
            .map(|r| r.version)
            .unwrap_or_default();

        Ok(SystemInfo {
            hostname: sys.hostname,
            os_name: os.name,
            os_version: os.version,
            os_codename: os.codename,
            platform: os.platform,
            arch: os.arch,
            uptime_seconds,
            uuid: None, // Could get from system_info if available
            kernel_version,
            collected_at: Utc::now(),
        })
    }

    /// Get hardware information
    ///
    /// # Errors
    /// Returns an error if osquery queries fail or if required data is missing.
    #[allow(clippy::too_many_lines)]
    #[instrument(skip(self))]
    pub async fn get_hardware_info(&self) -> Result<HardwareInfo, InventoryError> {
        debug!("collecting hardware info");

        // Get CPU info
        #[derive(Deserialize)]
        struct CpuRow {
            model: String,
            vendor: String,
            physical_cores: String,
            logical_cores: String,
            mhz: String,
        }

        let cpu_rows: Vec<CpuRow> = self.client.query(&queries::cpu_info()).await?;
        let cpu_row = cpu_rows
            .into_iter()
            .next()
            .ok_or_else(|| InventoryError::ParseError("no cpu_info data".to_string()))?;

        let cpu = CpuInfo {
            model: cpu_row.model,
            physical_cores: cpu_row.physical_cores.parse().unwrap_or(0),
            logical_cores: cpu_row.logical_cores.parse().unwrap_or(0),
            speed_mhz: cpu_row.mhz.parse().unwrap_or(0),
            vendor: cpu_row.vendor,
        };

        // Get memory info
        #[derive(Deserialize)]
        struct MemoryRow {
            total: String,
            free: String,
            used: String,
            swap_total: String,
            swap_free: String,
        }

        let mem_rows: Vec<MemoryRow> = self.client.query(&queries::memory_info()).await?;
        let mem_row = mem_rows
            .into_iter()
            .next()
            .ok_or_else(|| InventoryError::ParseError("no memory_info data".to_string()))?;

        let memory = MemoryInfo {
            total_bytes: mem_row.total.parse().unwrap_or(0),
            free_bytes: mem_row.free.parse().unwrap_or(0),
            used_bytes: mem_row.used.parse().unwrap_or(0),
            swap_total_bytes: mem_row.swap_total.parse().unwrap_or(0),
            swap_free_bytes: mem_row.swap_free.parse().unwrap_or(0),
        };

        // Get disk info from mounts
        #[derive(Deserialize)]
        struct MountRow {
            device: String,
            path: String,
            #[serde(rename = "type")]
            fs_type: String,
            blocks: String,
            blocks_free: String,
            #[serde(rename = "blocks_size")]
            block_size: String,
        }

        let mount_rows: Vec<MountRow> = self.client.query(&queries::mounts()).await?;
        let mut disks = Vec::new();

        for mount in mount_rows {
            let block_size: u64 = mount.block_size.parse().unwrap_or(4096);
            let total_blocks: u64 = mount.blocks.parse().unwrap_or(0);
            let free_blocks: u64 = mount.blocks_free.parse().unwrap_or(0);

            disks.push(DiskInfo {
                device: mount.device,
                mount_point: mount.path,
                filesystem: mount.fs_type,
                total_bytes: total_blocks * block_size,
                free_bytes: free_blocks * block_size,
                used_bytes: (total_blocks - free_blocks) * block_size,
            });
        }

        // Get network interfaces
        #[derive(Deserialize)]
        struct InterfaceRow {
            interface: String,
            mac: String,
        }

        #[derive(Deserialize)]
        struct AddressRow {
            interface: String,
            address: String,
        }

        let iface_rows: Vec<InterfaceRow> =
            self.client.query(&queries::interface_details()).await?;
        let addr_rows: Vec<AddressRow> = self.client.query(&queries::interface_addresses()).await?;

        let mut network_interfaces = Vec::new();

        for iface in iface_rows {
            let addrs: Vec<&AddressRow> = addr_rows
                .iter()
                .filter(|a| a.interface == iface.interface)
                .collect();

            let ipv4: Vec<String> = addrs
                .iter()
                .filter(|a| !a.address.contains(':'))
                .map(|a| a.address.clone())
                .collect();

            let ipv6: Vec<String> = addrs
                .iter()
                .filter(|a| a.address.contains(':'))
                .map(|a| a.address.clone())
                .collect();

            network_interfaces.push(NetworkInterface {
                name: iface.interface,
                mac: iface.mac,
                ipv4,
                ipv6,
            });
        }

        Ok(HardwareInfo {
            cpu,
            memory,
            disks,
            network_interfaces,
        })
    }

    /// Get installed packages
    ///
    /// # Errors
    /// Returns an error if osquery queries fail or if no package manager is available.
    #[instrument(skip(self))]
    pub async fn get_packages(&self) -> Result<Vec<Package>, InventoryError> {
        debug!("collecting packages");

        let mut packages = Vec::new();

        // Try deb_packages first
        #[derive(Deserialize)]
        struct DebRow {
            name: String,
            version: String,
            arch: String,
            install_time: Option<String>,
        }

        match self.client.query::<DebRow>(&queries::deb_packages()).await {
            Ok(rows) => {
                for row in rows {
                    packages.push(Package {
                        name: row.name,
                        version: row.version,
                        arch: row.arch,
                        source: PackageSource::Deb,
                        install_time: row.install_time.and_then(|t| {
                            t.parse::<i64>()
                                .ok()
                                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                        }),
                        size_bytes: None,
                    });
                }
            }
            Err(InventoryError::TableNotAvailable(_)) => {
                // Try rpm_packages
                #[derive(Deserialize)]
                struct RpmRow {
                    name: String,
                    version: String,
                    arch: String,
                    install_time: Option<String>,
                }

                match self.client.query::<RpmRow>(&queries::rpm_packages()).await {
                    Ok(rows) => {
                        for row in rows {
                            packages.push(Package {
                                name: row.name,
                                version: row.version,
                                arch: row.arch,
                                source: PackageSource::Rpm,
                                install_time: row.install_time.and_then(|t| {
                                    t.parse::<i64>()
                                        .ok()
                                        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                                }),
                                size_bytes: None,
                            });
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Err(e) => return Err(e),
        }

        info!(count = packages.len(), "collected packages");

        Ok(packages)
    }

    /// Get Docker containers
    ///
    /// # Errors
    /// Returns an error if osquery queries fail or Docker is not available.
    #[instrument(skip(self))]
    pub async fn get_docker_containers(&self) -> Result<Vec<Container>, InventoryError> {
        debug!("collecting docker containers");

        #[derive(Deserialize)]
        struct ContainerRow {
            id: String,
            name: String,
            image: String,
            state: String,
            status: String,
            created: String,
        }

        let rows: Vec<ContainerRow> = self.client.query(&queries::docker_containers()).await?;

        let containers = rows
            .into_iter()
            .map(|r| Container {
                id: r.id,
                name: r.name,
                image: r.image,
                state: r.state,
                status: r.status,
                created: r
                    .created
                    .parse::<i64>()
                    .ok()
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                    .unwrap_or_else(Utc::now),
                ports: Vec::new(),  // Would need docker_container_ports table
                mounts: Vec::new(), // Would need docker_container_mounts table
            })
            .collect();

        Ok(containers)
    }

    /// Get Docker images
    ///
    /// # Errors
    /// Returns an error if osquery queries fail or Docker is not available.
    #[instrument(skip(self))]
    pub async fn get_docker_images(&self) -> Result<Vec<Image>, InventoryError> {
        debug!("collecting docker images");

        #[derive(Deserialize)]
        struct ImageRow {
            id: String,
            tags: String,
            created: String,
            size: String,
        }

        let rows: Vec<ImageRow> = self.client.query(&queries::docker_images()).await?;

        let images = rows
            .into_iter()
            .map(|r| Image {
                id: r.id,
                tags: r.tags.split(',').map(|s| s.trim().to_string()).collect(),
                created: r
                    .created
                    .parse::<i64>()
                    .ok()
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                    .unwrap_or_else(Utc::now),
                size_bytes: r.size.parse().unwrap_or(0),
            })
            .collect();

        Ok(images)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tendhost_exec::LocalExecutor;

    // These tests require osquery to be installed
    // Marked as ignore for CI

    #[tokio::test]
    #[ignore = "requires osquery"]
    async fn test_collect_system_info() {
        let executor = Arc::new(LocalExecutor::new());
        let collector = InventoryCollector::new(executor, Duration::from_secs(60));

        let info = collector.get_system_info().await.unwrap();
        assert!(!info.hostname.is_empty());
    }
}

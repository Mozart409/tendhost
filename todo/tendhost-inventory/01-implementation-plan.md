# Implementation Plan: tendhost-inventory

## Overview

This plan implements the osquery-based inventory system for tendhost, providing:
- `OsqueryClient`: SQL query execution via osqueryi
- Type-safe query builder
- Structured inventory types (system, packages, docker, hardware)
- Caching layer for expensive queries
- High-level convenience API

## Architecture

```
┌─────────────────────────────────────────────────────┐
│              Inventory API (high-level)             │
│  • get_system_info() -> SystemInfo                  │
│  • get_packages() -> Vec<Package>                   │
│  • get_docker_containers() -> Vec<Container>        │
│  • get_hardware_info() -> HardwareInfo              │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│              OsqueryClient                          │
│  • query<T>(sql) -> Result<T>                       │
│  • query_cached<T>(sql, ttl) -> Result<T>           │
│  • execute(sql) -> Result<Vec<Row>>                 │
└──────────────────────┬──────────────────────────────┘
                       │ uses RemoteExecutor
                       ▼
┌─────────────────────────────────────────────────────┐
│              osqueryi (via SSH/local)               │
│  osqueryi --json "SELECT * FROM os_version"         │
└─────────────────────────────────────────────────────┘
```

---

## Phase 1: Foundation

### Task 1.1: Create Error Types (`error.rs`)
**Priority**: High  
**Estimated effort**: 30 min

Create `crates/tendhost-inventory/src/error.rs`:

```rust
//! Error types for tendhost-inventory

use thiserror::Error;

/// Errors that can occur during inventory operations
#[derive(Error, Debug, Clone)]
pub enum InventoryError {
    /// osquery is not installed on the target system
    #[error("osquery not found: {0}")]
    OsqueryNotFound(String),

    /// SQL query execution failed
    #[error("query execution failed: {0}")]
    QueryFailed(String),

    /// SQL syntax error
    #[error("SQL syntax error: {0}")]
    SqlSyntax(String),

    /// Failed to parse query results
    #[error("JSON parse error: {0}")]
    ParseError(String),

    /// Remote execution error
    #[error("execution error: {0}")]
    ExecutionError(String),

    /// Table not available on this system
    #[error("table not available: {0}")]
    TableNotAvailable(String),

    /// Query timeout
    #[error("query timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Cache error
    #[error("cache error: {0}")]
    CacheError(String),

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    ConfigError(String),
}

impl InventoryError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            InventoryError::ExecutionError(_) | InventoryError::Timeout(_)
        )
    }

    /// Check if osquery needs to be installed
    pub fn needs_installation(&self) -> bool {
        matches!(self, InventoryError::OsqueryNotFound(_))
    }
}
```

**Acceptance criteria**:
- [ ] Error enum covers all inventory failure modes
- [ ] `is_retryable()` helper for transient failures
- [ ] `needs_installation()` helper for missing osquery
- [ ] Public in lib.rs

---

## Phase 2: Type Definitions

### Task 2.1: Create Inventory Types (`types.rs`)
**Priority**: High  
**Estimated effort**: 1 hour

Rewrite `crates/tendhost-inventory/src/types.rs`:

```rust
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
    /// Architecture (x86_64, arm64, etc.)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    pub fn package_count_by_source(&self) -> HashMap<PackageSource, usize> {
        let mut counts = HashMap::new();
        for pkg in &self.packages {
            *counts.entry(pkg.source).or_insert(0) += 1;
        }
        counts
    }

    /// Check if Docker is installed
    pub fn has_docker(&self) -> bool {
        !self.docker_containers.is_empty() || !self.docker_images.is_empty()
    }
}

impl Default for HostInventory {
    fn default() -> Self {
        Self::new()
    }
}
```

**Acceptance criteria**:
- [ ] All types from GOALS.md defined
- [ ] Serde derives for JSON serialization
- [ ] Helper methods on `HostInventory`
- [ ] Comprehensive doc comments
- [ ] Public in lib.rs

---

## Phase 3: SQL Query Builder

### Task 3.1: Create Query Builder (`query.rs`)
**Priority**: High  
**Estimated effort**: 1 hour

Create `crates/tendhost-inventory/src/query.rs`:

```rust
//! SQL query builder for osquery

use std::fmt;

/// SQL query builder
///
/// Provides type-safe construction of osquery SQL queries.
#[derive(Debug, Clone)]
pub struct Query {
    /// SELECT clause
    select: Vec<String>,
    /// FROM clause
    from: String,
    /// WHERE clauses
    where_clauses: Vec<String>,
    /// ORDER BY clause
    order_by: Option<String>,
    /// LIMIT clause
    limit: Option<usize>,
}

impl Query {
    /// Create a new query for a table
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            select: vec!["*".to_string()],
            from: table.into(),
            where_clauses: Vec::new(),
            order_by: None,
            limit: None,
        }
    }

    /// Select specific columns
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.select = columns.iter().map(|c| c.to_string()).collect();
        self
    }

    /// Add WHERE clause
    pub fn where_eq(mut self, column: &str, value: &str) -> Self {
        // Escape single quotes in value
        let escaped = value.replace("'", "''");
        self.where_clauses.push(format!("{} = '{}'", column, escaped));
        self
    }

    /// Add WHERE clause with LIKE
    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        let escaped = pattern.replace("'", "''");
        self.where_clauses.push(format!("{} LIKE '{}'", column, escaped));
        self
    }

    /// Add WHERE clause with IN
    pub fn where_in(mut self, column: &str, values: &[&str]) -> Self {
        let escaped: Vec<String> = values
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        self.where_clauses.push(format!("{} IN ({})", column, escaped.join(", ")));
        self
    }

    /// Order by column
    pub fn order_by(mut self, column: &str, ascending: bool) -> Self {
        let dir = if ascending { "ASC" } else { "DESC" };
        self.order_by = Some(format!("{} {}", column, dir));
        self
    }

    /// Limit results
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Build the SQL string
    pub fn build(&self) -> String {
        let mut sql = format!("SELECT {} FROM {}", self.select.join(", "), self.from);

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        if let Some(ref order) = self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(order);
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        sql
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.build())
    }
}

/// Predefined queries for common inventory tasks
pub mod queries {
    use super::Query;

    /// Query for system information
    pub fn system_info() -> Query {
        Query::new("system_info")
            .select(&["hostname", "cpu_brand", "cpu_physical_cores", "cpu_logical_cores", "physical_memory"])
    }

    /// Query for OS version
    pub fn os_version() -> Query {
        Query::new("os_version")
            .select(&["name", "version", "codename", "platform", "arch"])
    }

    /// Query for uptime
    pub fn uptime() -> Query {
        Query::new("uptime")
            .select(&["days", "hours", "minutes", "seconds"])
    }

    /// Query for Debian packages
    pub fn deb_packages() -> Query {
        Query::new("deb_packages")
            .select(&["name", "version", "arch", "install_time"])
    }

    /// Query for RPM packages
    pub fn rpm_packages() -> Query {
        Query::new("rpm_packages")
            .select(&["name", "version", "arch", "install_time"])
    }

    /// Query for Docker containers
    pub fn docker_containers() -> Query {
        Query::new("docker_containers")
            .select(&["id", "name", "image", "state", "status", "created"])
    }

    /// Query for Docker images
    pub fn docker_images() -> Query {
        Query::new("docker_images")
            .select(&["id", "tags", "created", "size"])
    }

    /// Query for CPU info
    pub fn cpu_info() -> Query {
        Query::new("cpu_info")
            .select(&["model", "vendor", "physical_cores", "logical_cores", "mhz"])
    }

    /// Query for memory info
    pub fn memory_info() -> Query {
        Query::new("memory_info")
            .select(&["total", "free", "used", "swap_total", "swap_free"])
    }

    /// Query for disk info
    pub fn disk_info() -> Query {
        Query::new("disk_encryption")
            .select(&["name", "type", "uuid"])
    }

    /// Query for mounts
    pub fn mounts() -> Query {
        Query::new("mounts")
            .select(&["device", "path", "type", "total_blocks", "free_blocks"])
    }

    /// Query for network interfaces
    pub fn interface_addresses() -> Query {
        Query::new("interface_addresses")
            .select(&["interface", "address", "mask"])
    }

    /// Query for interface details
    pub fn interface_details() -> Query {
        Query::new("interface_details")
            .select(&["interface", "mac", "type"])
    }

    /// Query for listening ports
    pub fn listening_ports() -> Query {
        Query::new("listening_ports")
            .select(&["pid", "port", "protocol", "family"])
    }

    /// Query for kernel info
    pub fn kernel_info() -> Query {
        Query::new("kernel_info")
            .select(&["version", "arguments"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = Query::new("deb_packages")
            .select(&["name", "version"])
            .where_eq("arch", "amd64")
            .limit(10);

        let sql = query.build();
        assert!(sql.contains("SELECT name, version FROM deb_packages"));
        assert!(sql.contains("WHERE arch = 'amd64'"));
        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_query_where_like() {
        let query = Query::new("deb_packages")
            .where_like("name", "lib%");

        let sql = query.build();
        assert!(sql.contains("WHERE name LIKE 'lib%'"));
    }

    #[test]
    fn test_predefined_queries() {
        let query = queries::system_info();
        let sql = query.build();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM system_info"));
    }
}
```

**Acceptance criteria**:
- [ ] Type-safe query builder
- [ ] SQL injection prevention (quote escaping)
- [ ] Predefined queries for common tables
- [ ] Unit tests for query building

---

## Phase 4: OsqueryClient Implementation

### Task 4.1: Implement OsqueryClient (`osquery.rs`)
**Priority**: High  
**Estimated effort**: 2 hours

Rewrite `crates/tendhost-inventory/src/osquery.rs`:

```rust
//! osquery client for inventory collection

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tendhost_exec::traits::RemoteExecutor;
use tendhost_exec::RemoteExecutorExt;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use crate::error::InventoryError;
use crate::query::Query;
use crate::types::*;

/// Cached query result
#[derive(Debug, Clone)]
struct CachedResult {
    /// JSON result data
    data: String,
    /// When cached
    cached_at: Instant,
    /// Time-to-live
    ttl: Duration,
}

impl CachedResult {
    /// Check if cache entry is expired
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// osquery client for executing queries
///
/// Manages query execution and caching for inventory collection.
pub struct OsqueryClient {
    /// Remote executor for running osqueryi
    executor: Arc<dyn RemoteExecutor>,
    /// Query cache
    cache: RwLock<HashMap<String, CachedResult>>,
    /// Default cache TTL
    default_ttl: Duration,
    /// Query timeout
    timeout: Duration,
}

impl OsqueryClient {
    /// Create a new osquery client
    ///
    /// # Arguments
    /// * `executor` - Remote executor for running commands
    /// * `default_ttl` - Default cache time-to-live
    pub fn new(executor: Arc<dyn RemoteExecutor>, default_ttl: Duration) -> Self {
        Self {
            executor,
            cache: RwLock::new(HashMap::new()),
            default_ttl,
            timeout: Duration::from_secs(60),
        }
    }

    /// Set query timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if osquery is available
    #[instrument(skip(self))]
    pub async fn is_available(&self) -> bool {
        self.executor.command_exists("osqueryi").await.unwrap_or(false)
    }

    /// Execute a raw SQL query
    ///
    /// # Arguments
    /// * `sql` - SQL query string
    ///
    /// # Returns
    /// * `Ok(Vec<Value>)` - JSON array of results
    /// * `Err(InventoryError)` - Query failed
    #[instrument(skip(self, sql), fields(query = %sql))]
    pub async fn query_raw(&self, sql: &str) -> Result<Vec<Value>, InventoryError> {
        debug!("executing osquery");

        // Check if osquery is available
        if !self.is_available().await {
            return Err(InventoryError::OsqueryNotFound(
                "osqueryi not found on target system".to_string()
            ));
        }

        // Build command
        let cmd = format!("osqueryi --json '{}'", sql.replace("'", "'\"'\"'"));

        // Execute with timeout
        let result = self.executor.run_with_timeout(&cmd, self.timeout).await
            .map_err(|e| InventoryError::ExecutionError(e.to_string()))?;

        if !result.success() {
            // Check for specific errors
            if result.stderr.contains("no such table") {
                let table = extract_table_name(sql).unwrap_or_else(|| "unknown".to_string());
                return Err(InventoryError::TableNotAvailable(table));
            }
            if result.stderr.contains("syntax error") {
                return Err(InventoryError::SqlSyntax(result.stderr));
            }
            return Err(InventoryError::QueryFailed(result.stderr));
        }

        // Parse JSON output
        let json: Vec<Value> = serde_json::from_str(&result.stdout)
            .map_err(|e| InventoryError::ParseError(e.to_string()))?;

        debug!(rows = json.len(), "query completed");

        Ok(json)
    }

    /// Execute a typed query
    ///
    /// # Arguments
    /// * `query` - Query builder
    ///
    /// # Returns
    /// * `Ok(Vec<T>)` - Deserialized results
    pub async fn query<T: DeserializeOwned>(&self, query: &Query) -> Result<Vec<T>, InventoryError> {
        let sql = query.build();
        let json = self.query_raw(&sql).await?;

        // Deserialize each row
        let mut results = Vec::with_capacity(json.len());
        for value in json {
            let row: T = serde_json::from_value(value)
                .map_err(|e| InventoryError::ParseError(e.to_string()))?;
            results.push(row);
        }

        Ok(results)
    }

    /// Execute a query with caching
    ///
    /// # Arguments
    /// * `query` - Query builder
    /// * `ttl` - Cache time-to-live (None for default)
    ///
    /// # Returns
    /// * `Ok(Vec<T>)` - Deserialized results (cached or fresh)
    pub async fn query_cached<T: DeserializeOwned>(
        &self,
        query: &Query,
        ttl: Option<Duration>,
    ) -> Result<Vec<T>, InventoryError> {
        let sql = query.build();
        let cache_key = format!("{}", sql);

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if !cached.is_expired() {
                    debug!("cache hit");
                    let json: Vec<Value> = serde_json::from_str(&cached.data)
                        .map_err(|e| InventoryError::CacheError(e.to_string()))?;

                    let mut results = Vec::with_capacity(json.len());
                    for value in json {
                        let row: T = serde_json::from_value(value)
                            .map_err(|e| InventoryError::ParseError(e.to_string()))?;
                        results.push(row);
                    }
                    return Ok(results);
                }
            }
        }

        // Cache miss - execute query
        debug!("cache miss, executing query");
        let json = self.query_raw(&sql).await?;

        // Store in cache
        let ttl = ttl.unwrap_or(self.default_ttl);
        let cached = CachedResult {
            data: serde_json::to_string(&json)
                .map_err(|e| InventoryError::CacheError(e.to_string()))?,
            cached_at: Instant::now(),
            ttl,
        };

        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, cached);
        }

        // Deserialize results
        let mut results = Vec::with_capacity(json.len());
        for value in json {
            let row: T = serde_json::from_value(value)
                .map_err(|e| InventoryError::ParseError(e.to_string()))?;
            results.push(row);
        }

        Ok(results)
    }

    /// Clear query cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        debug!("cache cleared");
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|v| v.is_expired()).count();
        (total, expired)
    }
}

/// Extract table name from SQL query (simple heuristic)
fn extract_table_name(sql: &str) -> Option<String> {
    let sql_lower = sql.to_lowercase();
    if let Some(pos) = sql_lower.find("from ") {
        let after_from = &sql[pos + 5..];
        let end = after_from.find(|c: char| c.is_whitespace() || c == ';')
            .unwrap_or(after_from.len());
        Some(after_from[..end].trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tendhost_exec::LocalExecutor;

    #[test]
    fn test_extract_table_name() {
        assert_eq!(
            extract_table_name("SELECT * FROM deb_packages"),
            Some("deb_packages".to_string())
        );
        assert_eq!(
            extract_table_name("SELECT * FROM os_version WHERE name = 'Ubuntu'"),
            Some("os_version".to_string())
        );
    }
}
```

**Acceptance criteria**:
- [ ] Uses `RemoteExecutor` for command execution
- [ ] JSON output parsing
- [ ] Query caching with TTL
- [ ] Error handling for missing tables, syntax errors
- [ ] Timeout support
- [ ] Unit tests

---

## Phase 5: High-level Inventory API

### Task 5.1: Create Inventory Collector (`collector.rs`)
**Priority**: High  
**Estimated effort**: 1.5 hours

Create `crates/tendhost-inventory/src/collector.rs`:

```rust
//! High-level inventory collection API

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use tendhost_exec::traits::RemoteExecutor;
use tracing::{debug, error, info, instrument, warn};

use crate::error::InventoryError;
use crate::osquery::OsqueryClient;
use crate::query::queries;
use crate::types::*;

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
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.client = self.client.with_timeout(timeout);
        self
    }

    /// Collect full inventory
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
        let os = os_rows.into_iter().next().ok_or_else(|| {
            InventoryError::ParseError("no os_version data".to_string())
        })?;

        // Get system info
        #[derive(Deserialize)]
        struct SystemInfoRow {
            hostname: String,
            cpu_brand: String,
            cpu_physical_cores: String,
            cpu_logical_cores: String,
            physical_memory: String,
        }

        let sys_rows: Vec<SystemInfoRow> = self.client.query(&queries::system_info()).await?;
        let sys = sys_rows.into_iter().next().ok_or_else(|| {
            InventoryError::ParseError("no system_info data".to_string())
        })?;

        // Get uptime
        #[derive(Deserialize)]
        struct UptimeRow {
            seconds: String,
        }

        let uptime_rows: Vec<UptimeRow> = self.client.query(&queries::uptime()).await?;
        let uptime_seconds = uptime_rows
            .into_iter()
            .next()
            .and_then(|r| r.seconds.parse().ok())
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
        let cpu_row = cpu_rows.into_iter().next().ok_or_else(|| {
            InventoryError::ParseError("no cpu_info data".to_string())
        })?;

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
        let mem_row = mem_rows.into_iter().next().ok_or_else(|| {
            InventoryError::ParseError("no memory_info data".to_string())
        })?;

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
            mask: String,
        }

        let iface_rows: Vec<InterfaceRow> = self.client.query(&queries::interface_details()).await?;
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
                            t.parse::<i64>().ok().map(|ts| {
                                chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.to_utc())
                            })?
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
                                    t.parse::<i64>().ok().map(|ts| {
                                        chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.to_utc())
                                    })?
                                }),
                                size_bytes: None,
                            }));
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
                created: r.created.parse().unwrap_or_else(|_| Utc::now()),
                ports: Vec::new(), // Would need docker_container_ports table
                mounts: Vec::new(), // Would need docker_container_mounts table
            })
            .collect();

        Ok(containers)
    }

    /// Get Docker images
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
                created: r.created.parse().unwrap_or_else(|_| Utc::now()),
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
```

**Acceptance criteria**:
- [ ] High-level methods for all inventory types
- [ ] Error resilience (continue on partial failures)
- [ ] Structured logging
- [ ] Unit tests (marked ignore for CI)

---

## Phase 6: Integration

### Task 6.1: Update lib.rs Exports
**Priority**: Medium  
**Estimated effort**: 15 min

Update `crates/tendhost-inventory/src/lib.rs`:

```rust
//! tendhost-inventory: osquery-based inventory collection
//!
//! Provides unified inventory querying across different Linux distributions using osquery.
//!
//! # Example
//! ```rust
//! use std::sync::Arc;
//! use std::time::Duration;
//! use tendhost_exec::LocalExecutor;
//! use tendhost_inventory::{InventoryCollector, OsqueryClient, queries};
//!
//! let executor = Arc::new(LocalExecutor::new());
//! let collector = InventoryCollector::new(executor, Duration::from_secs(300));
//! // let inventory = collector.collect_full().await?;
//! ```

pub mod collector;
pub mod error;
pub mod osquery;
pub mod query;
pub mod types;

pub use collector::InventoryCollector;
pub use error::InventoryError;
pub use osquery::OsqueryClient;
pub use query::{Query, queries};
pub use types::*;
```

---

## Summary

### File Changes Required

| File | Action | Description |
|------|--------|-------------|
| `src/error.rs` | Create | Inventory error types |
| `src/types.rs` | Modify | Inventory type definitions |
| `src/query.rs` | Create | SQL query builder |
| `src/osquery.rs` | Modify | OsqueryClient implementation |
| `src/collector.rs` | Create | High-level inventory API |
| `src/lib.rs` | Modify | Re-exports |

### Estimated Total Effort

| Phase | Effort |
|-------|--------|
| Phase 1: Foundation | 30 min |
| Phase 2: Type Definitions | 1 hour |
| Phase 3: Query Builder | 1 hour |
| Phase 4: OsqueryClient | 2 hours |
| Phase 5: Inventory API | 1.5 hours |
| Phase 6: Integration | 15 min |
| **Total** | **~6.5 hours** |

### Dependencies

- **Blocks**: `tendhost-core` (uses inventory for host data)
- **Blocked by**: `tendhost-exec` (uses RemoteExecutor)

### Notes

- osquery must be installed on target hosts
- Some tables may not be available on all distros
- Caching is important for performance (package lists can be large)
- Integration tests require osquery to be installed

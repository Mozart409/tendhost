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
    #[must_use]
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.select = columns.iter().map(|c| (*c).to_string()).collect();
        self
    }

    /// Add WHERE clause
    #[must_use]
    pub fn where_eq(mut self, column: &str, value: &str) -> Self {
        // Escape single quotes in value
        let escaped = value.replace('\'', "''");
        self.where_clauses.push(format!("{column} = '{escaped}'"));
        self
    }

    /// Add WHERE clause with LIKE
    #[must_use]
    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        let escaped = pattern.replace('\'', "''");
        self.where_clauses
            .push(format!("{column} LIKE '{escaped}'"));
        self
    }

    /// Add WHERE clause with IN
    #[must_use]
    pub fn where_in(mut self, column: &str, values: &[&str]) -> Self {
        let escaped: Vec<String> = values
            .iter()
            .map(|v| format!("'{}'", v.replace('\'', "''")))
            .collect();
        self.where_clauses
            .push(format!("{column} IN ({})", escaped.join(", ")));
        self
    }

    /// Order by column
    #[must_use]
    pub fn order_by(mut self, column: &str, ascending: bool) -> Self {
        let dir = if ascending { "ASC" } else { "DESC" };
        self.order_by = Some(format!("{column} {dir}"));
        self
    }

    /// Limit results
    #[must_use]
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Build the SQL string
    #[must_use]
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
            use std::fmt::Write;
            let _ = write!(sql, " LIMIT {limit}");
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
    #[must_use]
    pub fn system_info() -> Query {
        Query::new("system_info").select(&[
            "hostname",
            "cpu_brand",
            "cpu_physical_cores",
            "cpu_logical_cores",
            "physical_memory",
        ])
    }

    /// Query for OS version
    #[must_use]
    pub fn os_version() -> Query {
        Query::new("os_version").select(&["name", "version", "codename", "platform", "arch"])
    }

    /// Query for uptime
    #[must_use]
    pub fn uptime() -> Query {
        Query::new("uptime").select(&["days", "hours", "minutes", "seconds", "total_seconds"])
    }

    /// Query for Debian packages
    #[must_use]
    pub fn deb_packages() -> Query {
        Query::new("deb_packages").select(&["name", "version", "arch", "install_time"])
    }

    /// Query for RPM packages
    #[must_use]
    pub fn rpm_packages() -> Query {
        Query::new("rpm_packages").select(&["name", "version", "arch", "install_time"])
    }

    /// Query for Docker containers
    #[must_use]
    pub fn docker_containers() -> Query {
        Query::new("docker_containers")
            .select(&["id", "name", "image", "state", "status", "created"])
    }

    /// Query for Docker images
    #[must_use]
    pub fn docker_images() -> Query {
        Query::new("docker_images").select(&["id", "tags", "created", "size"])
    }

    /// Query for CPU info
    #[must_use]
    pub fn cpu_info() -> Query {
        Query::new("cpu_info")
            .select(&[
                "brand as model",
                "vendor",
                "physical_cores",
                "logical_cores",
                "max_mhz as mhz",
            ])
            .limit(1)
    }

    /// Query for memory info
    #[must_use]
    pub fn memory_info() -> Query {
        Query::new("memory_info").select(&[
            "memory_total as total",
            "memory_free as free",
            "(memory_total - memory_free) as used",
            "swap_total",
            "swap_free",
        ])
    }

    /// Query for disk info
    #[must_use]
    pub fn disk_info() -> Query {
        Query::new("disk_encryption").select(&["name", "type", "uuid"])
    }

    /// Query for mounts
    #[must_use]
    pub fn mounts() -> Query {
        Query::new("mounts").select(&[
            "device",
            "path",
            "type",
            "blocks",
            "blocks_free",
            "blocks_size",
        ])
    }

    /// Query for network interfaces
    #[must_use]
    pub fn interface_addresses() -> Query {
        Query::new("interface_addresses").select(&["interface", "address", "mask"])
    }

    /// Query for interface details
    #[must_use]
    pub fn interface_details() -> Query {
        Query::new("interface_details").select(&["interface", "mac", "type"])
    }

    /// Query for listening ports
    #[must_use]
    pub fn listening_ports() -> Query {
        Query::new("listening_ports").select(&["pid", "port", "protocol", "family"])
    }

    /// Query for kernel info
    #[must_use]
    pub fn kernel_info() -> Query {
        Query::new("kernel_info").select(&["version", "arguments"])
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
        let query = Query::new("deb_packages").where_like("name", "lib%");

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

    #[test]
    fn test_sql_injection_prevention() {
        let query = Query::new("deb_packages").where_eq("name", "test' OR '1'='1");

        let sql = query.build();
        // Single quotes should be escaped
        assert!(sql.contains("test'' OR ''1''=''1"));
    }

    #[test]
    fn test_where_in() {
        let query = Query::new("deb_packages").where_in("arch", &["amd64", "arm64"]);

        let sql = query.build();
        assert!(sql.contains("WHERE arch IN ('amd64', 'arm64')"));
    }

    #[test]
    fn test_order_by() {
        let query = Query::new("deb_packages").order_by("name", true);

        let sql = query.build();
        assert!(sql.contains("ORDER BY name ASC"));
    }
}

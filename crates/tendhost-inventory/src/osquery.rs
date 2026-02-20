//! osquery client for inventory collection

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde_json::Value;
use tendhost_exec::traits::RemoteExecutor;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

use crate::error::InventoryError;
use crate::query::Query;

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
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if osquery is available
    #[instrument(skip(self))]
    pub async fn is_available(&self) -> bool {
        // Check if osqueryi command exists
        let result = self.executor.run("which osqueryi").await;
        result.map(|r| r.success()).unwrap_or(false)
    }

    /// Execute a raw SQL query
    ///
    /// # Arguments
    /// * `sql` - SQL query string
    ///
    /// # Returns
    /// * `Ok(Vec<Value>)` - JSON array of results
    /// * `Err(InventoryError)` - Query failed
    ///
    /// # Errors
    /// Returns an error if osquery is not available, the query fails, or JSON parsing fails.
    #[instrument(skip(self, sql), fields(query = %sql))]
    pub async fn query_raw(&self, sql: &str) -> Result<Vec<Value>, InventoryError> {
        debug!("executing osquery");

        // Check if osquery is available
        if !self.is_available().await {
            return Err(InventoryError::OsqueryNotFound(
                "osqueryi not found on target system".to_string(),
            ));
        }

        // Build command - escape single quotes in SQL by replacing with '"'"'
        let cmd = format!("osqueryi --json '{}'", sql.replace('\'', "'\"'\"'"));

        // Execute with timeout
        let result = self
            .executor
            .run_with_timeout(&cmd, self.timeout)
            .await
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
    ///
    /// # Errors
    /// Returns an error if the query fails or deserialization fails.
    pub async fn query<T: DeserializeOwned>(
        &self,
        query: &Query,
    ) -> Result<Vec<T>, InventoryError> {
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
    ///
    /// # Errors
    /// Returns an error if the query fails, cache operations fail, or deserialization fails.
    pub async fn query_cached<T: DeserializeOwned>(
        &self,
        query: &Query,
        ttl: Option<Duration>,
    ) -> Result<Vec<T>, InventoryError> {
        let sql = query.build();
        let cache_key = sql.clone();

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key)
                && !cached.is_expired()
            {
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
    #[must_use]
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
        let end = after_from
            .find(|c: char| c.is_whitespace() || c == ';')
            .unwrap_or(after_from.len());
        Some(after_from[..end].trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

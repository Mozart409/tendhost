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
//! # async fn example() {
//! let executor = Arc::new(LocalExecutor::new());
//! let collector = InventoryCollector::new(executor, Duration::from_secs(300));
//! // let inventory = collector.collect_full().await?;
//! # }
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

//! tendhost-pkg: Package manager abstraction
//!
//! Provides traits and implementations for different package managers
//! (apt, dnf, docker compose).
//!
//! # Example
//! ```rust,no_run
//! use std::sync::Arc;
//! use tendhost_exec::LocalExecutor;
//! use tendhost_pkg::{AptManager, PackageManager};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), tendhost_pkg::PackageError> {
//! let executor = Arc::new(LocalExecutor::new());
//! let manager = AptManager::new(executor, true);
//! let packages = manager.list_upgradable().await?;
//! # Ok(())
//! # }
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

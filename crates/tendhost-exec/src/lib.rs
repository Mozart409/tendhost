//! tendhost-exec: Remote execution abstraction
//!
//! Provides traits and implementations for executing commands locally and remotely via SSH.
//!
//! # Example
//! ```rust,no_run
//! use tendhost_exec::{LocalExecutor, RemoteExecutor};
//!
//! #[tokio::main]
//! async fn main() {
//!     let executor = LocalExecutor::new();
//!     let result = executor.run("echo hello").await.unwrap();
//!     assert!(result.success());
//! }
//! ```

pub mod error;
pub mod keys;
pub mod local;
pub mod result;
pub mod ssh;
pub mod traits;

pub use error::ExecError;
pub use keys::{KeySource, ResolvedKey};
pub use local::LocalExecutor;
pub use result::{CommandResult, ConnectionInfo};
pub use ssh::{SshExecutor, SshExecutorBuilder};
pub use traits::{RemoteExecutor, RemoteExecutorExt};

//! tendhost-pkg: Package manager abstraction
//!
//! Provides traits and implementations for different package managers
//! (apt, dnf, docker compose)

pub mod traits;
pub mod apt;
pub mod dnf;
pub mod docker;

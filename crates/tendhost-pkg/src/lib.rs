//! tendhost-pkg: Package manager abstraction
//!
//! Provides traits and implementations for different package managers
//! (apt, dnf, docker compose)

pub mod apt;
pub mod dnf;
pub mod docker;
pub mod traits;

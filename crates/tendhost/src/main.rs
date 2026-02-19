//! tendhost daemon
//!
//! Actor-based homelab orchestration system using axum HTTP server and kameo actors

use color_eyre::Result;

mod api;
mod config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // TODO: Load configuration
    // TODO: Initialize actors
    // TODO: Start HTTP server

    println!("tendhost daemon starting...");
    Ok(())
}

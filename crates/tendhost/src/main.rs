// Copyright (C) 2026 Mozart409
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! tendhost daemon
//!
//! Actor-based homelab orchestration system.
//!
//! # Status
//! This is a minimal skeleton implementation. Full API implementation pending.
//!
//! # Usage
//! ```bash
//! # Run with default config
//! tendhost
//!
//! # Run with specific config file
//! TENDHOST_CONFIG=/path/to/tendhost.toml tendhost
//! ```

use std::sync::Arc;

use color_eyre::Result;
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

use kameo::actor::Spawn;
use tendhost_core::{OrchestratorActor, OrchestratorActorArgs};

mod api;
mod config;
mod factory;
mod router;
mod state;

use config::Config;
use factory::DefaultHostFactory;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Load configuration
    let config = Config::load_default()?;

    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.daemon.log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    info!("tendhost daemon starting...");
    info!(bind = %config.daemon.bind, "configuration loaded");

    // Create host factory
    let host_factory = Arc::new(DefaultHostFactory::new());

    // Spawn orchestrator actor with factory
    let orchestrator_args = OrchestratorActorArgs {
        event_channel_capacity: 1024,
        host_factory,
    };
    let orchestrator = OrchestratorActor::spawn(orchestrator_args);

    info!("orchestrator actor started");

    // TODO: Register hosts from config
    // for host_config in &config.host {
    //     orchestrator.ask(RegisterHost { config: host_config.clone() }).await?;
    // }

    // Create application state
    let state = Arc::new(AppState::new(orchestrator.clone(), config.clone()));

    // Create router
    let app = router::create_router(state);

    // Create listener
    let listener = tokio::net::TcpListener::bind(&config.daemon.bind).await?;
    info!(addr = %config.daemon.bind, "HTTP server listening");
    info!(
        "Health endpoint available at http://{}/health",
        config.daemon.bind
    );

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("shutting down...");

    // Stop orchestrator (which stops all host actors)
    let _ = orchestrator.stop_gracefully().await;

    info!("shutdown complete");
    Ok(())
}

/// Wait for shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("shutdown signal received");
}

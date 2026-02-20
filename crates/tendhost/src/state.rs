//! Application state shared across HTTP handlers

#![allow(dead_code)]

use std::sync::Arc;

use kameo::actor::ActorRef;
use tendhost_core::OrchestratorActor;

use crate::config::Config;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Reference to the orchestrator actor
    pub orchestrator: ActorRef<OrchestratorActor>,
    /// Application configuration
    pub config: Arc<Config>,
}

impl AppState {
    /// Create new application state
    pub fn new(orchestrator: ActorRef<OrchestratorActor>, config: Config) -> Self {
        Self {
            orchestrator,
            config: Arc::new(config),
        }
    }
}

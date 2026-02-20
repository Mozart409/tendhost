//! HTTP router configuration
//!
//! Minimal skeleton - full implementation pending

use std::sync::Arc;

use axum::{Router, routing::get};

use crate::api::system;
use crate::state::AppState;

/// Create the application router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // System endpoints
        .route("/health", get(system::health))
        // State
        .with_state(state)
}

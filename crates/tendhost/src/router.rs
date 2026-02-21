//! HTTP router configuration

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::api::{hosts, system};
use crate::state::AppState;

/// Create the application router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // System endpoints
        .route("/health", get(system::health))
        // Host endpoints
        .route("/hosts", get(hosts::list_hosts).post(hosts::register_host))
        .route(
            "/hosts/:hostname",
            get(hosts::get_host).delete(hosts::unregister_host),
        )
        .route("/hosts/:hostname/update", post(hosts::update_host))
        .route("/hosts/:hostname/reboot", post(hosts::reboot_host))
        .route("/hosts/:hostname/retry", post(hosts::retry_host))
        .route(
            "/hosts/:hostname/acknowledge",
            post(hosts::acknowledge_host),
        )
        .route("/hosts/:hostname/inventory", get(hosts::get_host_inventory))
        // State
        .with_state(state)
}

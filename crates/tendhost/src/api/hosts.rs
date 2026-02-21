//! Host management API endpoints

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tendhost_api::requests::UpdateRequest;
use tendhost_core::{
    AcknowledgeHost, GetHostStatus, ListHosts, QueryHostInventory, RegisterHost, RetryHost,
    TriggerHostUpdate, UnregisterHost,
};
use utoipa::ToSchema;

use crate::api::error::AppError;
use crate::state::AppState;

/// Query parameters for listing hosts
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListHostsQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u64,
    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: u64,
    /// Filter by tags
    #[serde(default)]
    pub tags: Option<String>,
}

fn default_page() -> u64 {
    1
}

fn default_per_page() -> u64 {
    50
}

/// Host list response
#[derive(Debug, Serialize, ToSchema)]
pub struct HostListResponse {
    /// List of hosts
    pub hosts: Vec<HostSummary>,
    /// Pagination info
    pub pagination: PaginationInfo,
}

/// Host summary for list view
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HostSummary {
    /// Host name
    pub name: String,
    /// Current state
    pub state: String,
    /// Number of pending updates
    pub pending_updates: Option<u32>,
    /// Tags
    pub tags: Vec<String>,
    /// Last update timestamp
    pub last_updated: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Pagination metadata
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    /// Current page (1-indexed)
    pub page: u64,
    /// Items per page
    pub per_page: u64,
    /// Total number of items
    pub total_items: u64,
    /// Total number of pages
    pub total_pages: u64,
}

/// Host details response
#[derive(Debug, Serialize, ToSchema)]
pub struct HostDetailResponse {
    /// Host name
    pub name: String,
    /// Current state
    pub state: String,
    /// Number of pending updates
    pub pending_updates: Option<u32>,
    /// Tags
    pub tags: Vec<String>,
    /// Last update timestamp
    pub last_updated: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Host registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterHostRequest {
    /// Host name
    pub name: String,
    /// Host address
    pub addr: String,
    /// SSH user
    #[serde(default = "default_user")]
    pub user: String,
    /// SSH key path
    pub ssh_key: Option<String>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_user() -> String {
    "root".to_string()
}

/// List all managed hosts
///
/// # Errors
/// Returns `AppError` if orchestrator communication fails
pub async fn list_hosts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListHostsQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Get all hosts from orchestrator
    let hosts = state
        .orchestrator
        .ask(ListHosts)
        .await
        .map_err(|e| AppError::internal(format!("failed to list hosts: {e}")))?;

    // Apply tag filtering if specified
    let mut filtered_hosts = hosts;
    if let Some(tags_str) = &query.tags {
        let filter_tags: Vec<&str> = tags_str.split(',').collect();
        filtered_hosts.retain(|h| {
            filter_tags
                .iter()
                .all(|tag| h.tags.iter().any(|t| t == tag))
        });
    }

    // Calculate pagination
    let total_items = filtered_hosts.len() as u64;
    let total_pages = total_items.div_ceil(query.per_page);
    #[allow(clippy::cast_possible_truncation)]
    let start = ((query.page - 1) * query.per_page) as usize;
    #[allow(clippy::cast_possible_truncation)]
    let end = (start + query.per_page as usize).min(filtered_hosts.len());

    // Get page of results
    let page_hosts: Vec<HostSummary> = filtered_hosts[start..end]
        .iter()
        .map(|h| HostSummary {
            name: h.name.clone(),
            state: format!("{:?}", h.state),
            pending_updates: h.pending_updates,
            tags: h.tags.clone(),
            last_updated: h.last_updated.map(|dt| dt.to_rfc3339()),
            error: h.error.clone(),
        })
        .collect();

    Ok(Json(HostListResponse {
        hosts: page_hosts,
        pagination: PaginationInfo {
            page: query.page,
            per_page: query.per_page,
            total_items,
            total_pages,
        },
    }))
}

/// Get details for a specific host
///
/// # Errors
/// Returns `AppError` if host not found or orchestrator communication fails
pub async fn get_host(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let status = state
        .orchestrator
        .ask(GetHostStatus { hostname })
        .await
        .map_err(|e| AppError::internal(format!("failed to get host status: {e}")))?;

    Ok(Json(HostDetailResponse {
        name: status.name,
        state: format!("{:?}", status.state),
        pending_updates: status.pending_updates,
        tags: status.tags,
        last_updated: status.last_updated.map(|dt| dt.to_rfc3339()),
        error: status.error,
    }))
}

/// Register a new host
///
/// # Errors
/// Returns `AppError` if registration fails
pub async fn register_host(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterHostRequest>,
) -> Result<impl IntoResponse, AppError> {
    use tendhost_core::{HostConfig, HostPolicy};

    let config = HostConfig {
        name: req.name,
        addr: req.addr,
        user: req.user,
        ssh_key: req.ssh_key,
        compose_paths: vec![],
        tags: req.tags,
        policy: HostPolicy::default(),
    };

    state
        .orchestrator
        .ask(RegisterHost { config })
        .await
        .map_err(|e| AppError::internal(format!("failed to register host: {e}")))?;

    Ok(StatusCode::CREATED)
}

/// Unregister a host
///
/// # Errors
/// Returns `AppError` if host not found or unregistration fails
pub async fn unregister_host(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state
        .orchestrator
        .ask(UnregisterHost { hostname })
        .await
        .map_err(|e| AppError::internal(format!("failed to unregister host: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Trigger update for a specific host
///
/// # Errors
/// Returns `AppError` if update trigger fails
pub async fn update_host(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
    Json(req): Json<UpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .orchestrator
        .ask(TriggerHostUpdate {
            hostname,
            dry_run: req.dry_run,
        })
        .await
        .map_err(|e| AppError::internal(format!("failed to trigger update: {e}")))?;

    Ok(StatusCode::ACCEPTED)
}

/// Trigger reboot for a specific host
///
/// # Errors
/// Returns `AppError` if reboot trigger fails
pub async fn reboot_host(
    State(_state): State<Arc<AppState>>,
    Path(_hostname): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // For now, we just accept the request
    // TODO: Implement actual reboot logic through orchestrator
    Ok(StatusCode::ACCEPTED)
}

/// Retry a failed host
///
/// # Errors
/// Returns `AppError` if retry fails
pub async fn retry_host(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state
        .orchestrator
        .ask(RetryHost { hostname })
        .await
        .map_err(|e| AppError::internal(format!("failed to retry host: {e}")))?;

    Ok(StatusCode::ACCEPTED)
}

/// Acknowledge a failed host
///
/// # Errors
/// Returns `AppError` if acknowledgement fails
pub async fn acknowledge_host(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state
        .orchestrator
        .ask(AcknowledgeHost { hostname })
        .await
        .map_err(|e| AppError::internal(format!("failed to acknowledge host: {e}")))?;

    Ok(StatusCode::ACCEPTED)
}

/// Get host inventory
///
/// # Errors
/// Returns `AppError` if inventory query fails
pub async fn get_host_inventory(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let _inventory = state
        .orchestrator
        .ask(QueryHostInventory { hostname })
        .await
        .map_err(|e| AppError::internal(format!("failed to query inventory: {e}")))?;

    // TODO: Return actual inventory when tendhost-core::InventoryResult implements Serialize
    // For now, return a placeholder
    Ok(Json(serde_json::json!({
        "message": "inventory query accepted - response structure pending"
    })))
}

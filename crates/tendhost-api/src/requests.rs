//! Request types for the API

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateRequest {
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FleetUpdateRequest {
    pub batch_size: usize,
    pub delay_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<FleetUpdateFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FleetUpdateFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_hosts: Option<Vec<String>>,
}

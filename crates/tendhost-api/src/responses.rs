//! Response types for the API

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Pagination {
    pub page: u64,
    pub per_page: u64,
    pub total_items: u64,
    pub total_pages: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

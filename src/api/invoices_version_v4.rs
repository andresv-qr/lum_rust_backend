use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::api::common::{ApiResponse, get_dataset_version};
use crate::state::AppState;

/// Version response for lightweight sync checks
#[derive(Debug, serde::Serialize)]
pub struct VersionResponse {
    pub dataset_version: i64,
    pub last_modified: Option<chrono::NaiveDateTime>,
    pub server_timestamp: chrono::NaiveDateTime,
    pub total_records: i64,
}

/// Create router for version endpoints
pub fn create_version_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/invoices/:resource/version", get(get_resource_version))
}

/// GET /api/v4/invoices/{resource}/version - Get dataset version info
/// Lightweight endpoint to check if local data is up-to-date
/// Supported resources: products, issuers, headers, details
pub async fn get_resource_version(
    State(state): State<Arc<AppState>>,
    Path(resource): Path<String>,
) -> Result<Json<ApiResponse<VersionResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now().naive_utc();
    
    info!("📊 Fetching version for resource: {} [{}]", resource, request_id);

    // Map resource to table name
    let table_name = match resource.as_str() {
        "products" => "dim_product",
        "issuers" => "dim_issuer",
        "headers" => "invoice_header",
        "details" => "invoice_detail",
        _ => {
            error!("❌ Invalid resource: {} [{}]", resource, request_id);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Get dataset version
    let dataset_version = get_dataset_version(&state.db_pool, table_name)
        .await
        .map_err(|e| {
            error!("❌ Failed to get dataset version for {}: {} [{}]", table_name, e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get last_modified from dataset_versions table
    let last_modified: Option<chrono::NaiveDateTime> = sqlx::query_scalar(
        "SELECT last_modified FROM dataset_versions WHERE table_name = $1"
    )
    .bind(table_name)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to fetch last_modified: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .flatten();

    // Get total_records count
    let count_query = match resource.as_str() {
        "products" => "SELECT COUNT(*) FROM dim_product WHERE is_deleted = FALSE",
        "issuers" => "SELECT COUNT(*) FROM dim_issuer WHERE is_deleted = FALSE",
        "headers" => "SELECT COUNT(*) FROM invoice_header WHERE is_deleted = FALSE",
        "details" => "SELECT COUNT(*) FROM invoice_detail WHERE is_deleted = FALSE",
        _ => unreachable!(),
    };
    
    let total_records: i64 = sqlx::query_scalar(count_query)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0);

    let response = VersionResponse {
        dataset_version,
        last_modified,
        server_timestamp,
        total_records,
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        "✅ Version check for {} - version: {}, total: {}, modified: {} in {}ms [{}]", 
        resource,
        dataset_version,
        total_records,
        last_modified.map(|d| d.to_string()).unwrap_or_else(|| "never".to_string()),
        execution_time,
        request_id
    );

    Ok(Json(ApiResponse::success(
        response, 
        request_id, 
        Some(execution_time), 
        false
    )))
}

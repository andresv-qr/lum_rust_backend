use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::api::common::ApiResponse;
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;

/// Response model for sync status endpoint
/// Provides quick metadata for client to determine sync strategy
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncStatusResponse {
    /// Number of invoice headers for the user
    pub headers_count: i64,
    /// Most recent update_date among headers (for incremental sync reference)
    pub headers_max_update_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Server timestamp when this status was generated
    pub server_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Internal struct for query result
#[derive(Debug, sqlx::FromRow)]
struct SyncStatusQuery {
    count: Option<i64>,
    max_update: Option<chrono::DateTime<chrono::Utc>>,
}

/// Create router for sync status endpoint
pub fn create_sync_status_v4_router() -> Router<Arc<AppState>> {
    use axum::middleware::from_fn;
    use crate::middleware::auth::extract_current_user;
    
    Router::new()
        .route("/sync-status", get(get_sync_status))
        .layer(from_fn(extract_current_user))
}

/// GET /api/v4/invoices/sync-status - Get sync status for a user
/// 
/// Returns count and max_update_date of invoice headers to help client
/// determine the appropriate sync strategy:
/// - If local count == server count AND local max_date >= server max_date → fully synced
/// - If local count < server count → use incremental sync with update_date_from
/// - If significant difference → consider full_sync=true
/// 
/// Performance: Single indexed query, ~1-5ms response time
pub async fn get_sync_status(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<SyncStatusResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now();
    
    info!("📊 Fetching sync status for user_id: {} [{}]", current_user.user_id, request_id);

    // Single efficient query using index on (user_id, is_deleted)
    let result = sqlx::query_as::<_, SyncStatusQuery>(
        r#"
        SELECT 
            COUNT(*) as count, 
            MAX(update_date) as max_update
        FROM public.invoice_header 
        WHERE user_id = $1 AND is_deleted = FALSE
        "#
    )
    .bind(current_user.user_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to fetch sync status: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let elapsed = start_time.elapsed();
    info!(
        "✅ Sync status fetched: count={}, max_update={:?} in {:?} [{}]",
        result.count.unwrap_or(0),
        result.max_update,
        elapsed,
        request_id
    );

    let response = ApiResponse {
        success: true,
        data: Some(SyncStatusResponse {
            headers_count: result.count.unwrap_or(0),
            headers_max_update_date: result.max_update,
            server_timestamp,
        }),
        error: None,
        request_id,
        timestamp: server_timestamp,
        execution_time_ms: Some(elapsed.as_millis() as u64),
        cached: false,
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_response_serialization() {
        let response = SyncStatusResponse {
            headers_count: 1234,
            headers_max_update_date: Some(chrono::Utc::now()),
            server_timestamp: chrono::Utc::now(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("headers_count"));
        assert!(json.contains("1234"));
    }
}

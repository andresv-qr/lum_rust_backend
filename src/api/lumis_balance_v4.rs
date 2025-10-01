use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::api::common::{ApiResponse, ApiError, DatabaseService};
use crate::api::templates::lumis_balance_templates::{
    LumisBalanceQueryTemplates, LumisBalanceResponse
};
use crate::state::AppState;

/// Create router for lumis balance V4 endpoints
pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:user_id", get(get_user_lumis_balance))
}

/// Get user's current Lumis balance - V4 endpoint
/// GET /api/v4/lumis_balance/:user_id
pub async fn get_user_lumis_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<Json<ApiResponse<LumisBalanceResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let _db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    let _cache_key_prefix = LumisBalanceQueryTemplates::get_cache_key_prefix();
    let sql = LumisBalanceQueryTemplates::get_user_lumis_balance_query();
    
    info!("Executing lumis balance query for user {}: {}", user_id, sql);
    
    // Execute query to get user's Lumis balance
    let balance_result = sqlx::query_scalar::<_, i32>(sql)
        .bind(user_id.to_string())
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query execution failed: {}", e)))?;

    let lumis_balance = balance_result.unwrap_or(0);
    
    // Format the response with additional metadata
    let response_data = LumisBalanceResponse {
        lumis_balance,
        formatted_balance: format!("{} Lümis", lumis_balance),
        last_updated: Some(chrono::Utc::now()),
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!("Lumis balance query completed for user {} in {}ms: {} Lumis", 
          user_id, execution_time, lumis_balance);

    Ok(Json(ApiResponse::success(response_data, request_id, Some(execution_time), false)))
}

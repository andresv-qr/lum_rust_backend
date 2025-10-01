use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::api::common::{ApiResponse, ApiError, DatabaseService};
use crate::api::templates::movements_summary_templates::{
    MovementsSummaryQueryTemplates, MovementsSummaryResponse, RecentMovementResponse
};
use crate::state::AppState;

/// Create router for movements summary V4 endpoints
pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:user_id", get(get_user_movements_summary))
        .route("/:user_id/recent", get(get_recent_movements))
}

/// Get user's movements summary - V4 endpoint
/// GET /api/v4/movements_summary/:user_id
pub async fn get_user_movements_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<Json<ApiResponse<MovementsSummaryResponse>>, ApiError> {
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

    let _cache_key_prefix = MovementsSummaryQueryTemplates::get_cache_key_prefix();
    let sql = MovementsSummaryQueryTemplates::get_user_movements_summary_query();
    
    info!("Executing movements summary query for user {}: {}", user_id, sql);
    
    // Execute query to get user's movements summary
    let summary_result = sqlx::query_as::<_, MovementsSummaryResponse>(sql)
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query execution failed: {}", e)))?;

    let mut response_data = summary_result.unwrap_or(MovementsSummaryResponse {
        total_transactions: 0,
        total_earned: 0,
        total_spent: 0,
        recent_activity: 0,
        last_activity: None,
        net_balance: 0,
        activity_level: "inactive".to_string(),
    });

    // Calculate derived fields
    response_data.net_balance = response_data.total_earned - response_data.total_spent;
    response_data.activity_level = match response_data.recent_activity {
        0 => "inactive".to_string(),
        1..=5 => "low".to_string(),
        6..=15 => "moderate".to_string(),
        _ => "high".to_string(),
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!("Movements summary query completed for user {} in {}ms: {} transactions", 
          user_id, execution_time, response_data.total_transactions);

    Ok(Json(ApiResponse::success(response_data, request_id, Some(execution_time), false)))
}

/// Get user's recent movements - V4 endpoint
/// GET /api/v4/movements_summary/:user_id/recent
pub async fn get_recent_movements(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<RecentMovementResponse>>>, ApiError> {
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

    let sql = MovementsSummaryQueryTemplates::get_recent_movements_query();
    
    info!("Executing recent movements query for user {}: {}", user_id, sql);
    
    // Execute query to get user's recent movements
    let movements = sqlx::query_as::<_, RecentMovementResponse>(sql)
        .bind(user_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query execution failed: {}", e)))?;

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!("Recent movements query completed for user {} in {}ms: {} movements", 
          user_id, execution_time, movements.len());

    Ok(Json(ApiResponse::success(movements, request_id, Some(execution_time), false)))
}

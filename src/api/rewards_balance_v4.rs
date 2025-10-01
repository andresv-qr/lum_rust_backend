use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::api::common::{ApiResponse, ApiError, DatabaseService};
use crate::api::templates::rewards_balance_templates::{
    RewardsBalanceQueryTemplates, RewardsBalanceResponse, RewardsBalanceHelpers
};
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;

/// Create rewards balance v4 router
pub fn create_rewards_balance_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/rewards/balance", get(get_user_rewards_balance))
}

/// Get user rewards balance from JWT token (secure)
pub async fn get_user_rewards_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<RewardsBalanceResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let user_id = current_user.user_id;  // Extract user_id from JWT for security
    let _db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    let sql = RewardsBalanceQueryTemplates::get_user_balance_query();
    let cache_key = format!("{}_{}",RewardsBalanceQueryTemplates::get_user_balance_cache_key_prefix(), user_id);
    
    // TODO: Check cache first
    info!("Executing query for {}: {}", cache_key, sql);
    
    let result = sqlx::query_as::<_, RewardsBalanceResponse>(sql)
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query execution failed: {}", e)))?;

    let data = result.ok_or_else(|| ApiError::not_found("User rewards balance"))?;
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    // Add helpful message to response
    let balance_message = RewardsBalanceHelpers::format_balance_message(data.balance);
    let is_recent = RewardsBalanceHelpers::is_balance_recent(data.latest_update);
    
    info!("User {} balance: {} points ({})", user_id, data.balance, 
          if is_recent { "recently updated" } else { "needs update" });
    info!("Balance message: {}", balance_message);
    
    Ok(Json(ApiResponse::success(data, request_id, Some(execution_time), false)))
}

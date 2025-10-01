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
use crate::api::templates::user_profile_templates::{
    UserProfileQueryTemplates, UserProfileResponse, UserProfileSafeResponse, UserProfileHelpers
};
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;

/// Create user profile v4 router - SECURE VERSION
pub fn create_user_profile_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/users/profile", get(get_current_user_profile))
}

/// Get current user profile from JWT token (secure)
pub async fn get_current_user_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<UserProfileSafeResponse>>, ApiError> {
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

    let sql = UserProfileQueryTemplates::get_user_by_id_query();
    let _cache_key = format!("{}_{}",UserProfileQueryTemplates::get_user_by_id_cache_key_prefix(), user_id);
    
    // TODO: Check cache first
    info!("Executing profile query for user {}: {}", user_id, sql);
    
    // Query with UserProfileResponse (has FromRow) then convert to safe version
    let result = sqlx::query_as::<_, UserProfileResponse>(sql)
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query execution failed: {}", e)))?;

    let profile = result.ok_or_else(|| ApiError::not_found("User profile"))?;
    
    // Convert to safe response using From trait (removes sensitive data like password)
    let safe_data = UserProfileSafeResponse::from(profile);
    
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    // Add helpful profile info to logs
    let completion_percentage = UserProfileHelpers::get_profile_completion_percentage(&safe_data);
    let is_complete = UserProfileHelpers::is_profile_complete(&safe_data);
    
    info!("User {} profile: {} - completion: {}% ({})", 
          user_id, 
          safe_data.email,
          completion_percentage,
          if is_complete { "complete" } else { "incomplete" });
    
    Ok(Json(ApiResponse::success(safe_data, request_id, Some(execution_time), false)))
}

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::info;


use crate::api::common::{ApiResponse, ApiError, DatabaseService};
use crate::api::templates::profile_templates::{
    ProfileQueryTemplates, ProfileResponse
};
use crate::state::AppState;

/// Create profile v4 router
pub fn create_profile_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/profile/:id", get(get_user_profile))
}

/// get_user_profile handler - Get single record by ID
pub async fn get_user_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ProfileResponse>>, ApiError> {
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

    let sql = ProfileQueryTemplates::get_user_profile_query();
    let cache_key = format!("{}_{}", ProfileQueryTemplates::get_user_profile_cache_key_prefix(), id);
    
    // TODO: Check cache first
    info!("Executing query for {}: {}", cache_key, sql);
    
    let result = sqlx::query_as::<_, ProfileResponse>(sql)
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query execution failed: {}", e)))?;

    let data = result.ok_or_else(|| ApiError::not_found("Profile"))?;
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    Ok(Json(ApiResponse::success(data, request_id, Some(execution_time), false)))
}

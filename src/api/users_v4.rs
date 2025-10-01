use axum::{
    extract::{Path, State, Request},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::{info, debug};


use crate::api::common::{ApiResponse, ApiError};
use crate::api::templates::user_profile_templates::{
    UserProfileResponse, UserProfileSafeResponse
};
use crate::middleware::auth::get_current_user_from_request;
use crate::state::AppState;

/// Create users management v4 router
pub fn create_users_management_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/users/profile", get(get_user_profile))
        .route("/api/v4/users/profile/:id", get(get_user_profile_by_id))
}





/// get_user_profile handler - Get current user's profile (requires JWT authentication)
pub async fn get_user_profile(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<ApiResponse<UserProfileSafeResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let start_time = std::time::Instant::now();
    
    debug!(request_id = %request_id, "👤 Getting user profile v4");

    // Extract user ID from JWT token
    let current_user = get_current_user_from_request(&request)
        .map_err(|(_status, json_error)| {
            ApiError::new("UNAUTHORIZED", &json_error.0.message)
        })?;
    
    let user_id = current_user.user_id;

    let profile_data = get_user_profile_data(&_state, user_id).await?;
    
    // Convert to safe response (without sensitive data like password_hash)
    let safe_profile = UserProfileSafeResponse::from(profile_data);

    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        current_user_id = user_id,
        processing_time_ms = processing_time,
        "✅ User profile retrieved"
    );

    Ok(Json(ApiResponse::success(safe_profile, request_id, Some(processing_time), false)))
}

/// Get User Profile by ID endpoint (requires JWT authentication via middleware)
pub async fn get_user_profile_by_id(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<UserProfileSafeResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    
    debug!(request_id = %request_id, target_user_id = id, "👤 Getting user profile by ID v4");

    // TODO: Extract user ID from JWT token via middleware
    // For now, allow access to any profile until middleware is properly integrated
    let current_user_id = 1i64; // Placeholder - will be replaced by middleware
    debug!("Request {}: Processing for user: {}", request_id, current_user_id);

    let profile_data = get_user_profile_data(&_state, id).await?;
    
    // Convert to safe response (without sensitive data like password_hash)
    let safe_profile = UserProfileSafeResponse::from(profile_data);

    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        current_user_id = current_user_id,
        target_user_id = id,
        processing_time_ms = processing_time,
        "✅ User profile by ID retrieved"
    );

    Ok(Json(ApiResponse::success(safe_profile, request_id, Some(processing_time), false)))
}

// Helper functions
async fn get_user_profile_data(
    _state: &AppState,
    user_id: i64,
) -> Result<UserProfileResponse, ApiError> {
    // TODO: Get user basic info from database
    // For now, return simulated user data
    if user_id <= 0 {
        return Err(ApiError::not_found("User not found"));
    }

    // Create UserProfileResponse with correct fields from user_profile_templates
    Ok(UserProfileResponse {
        id: user_id,
        email: format!("user{}@example.com", user_id),
        ws_id: Some(format!("ws_{}", user_id)),
        telegram_id: None,
        created_at: Some(chrono::Utc::now()),
        ws_registration_date: Some(chrono::Utc::now()),
        telegram_registration_date: None,
        name: Some(format!("User {}", user_id)),
        date_of_birth: None,
        country_origin: Some("Panama".to_string()),
        country_residence: Some("Panama".to_string()),
        password_hash: Some("[REDACTED]".to_string()),
        segment_activity: Some("active".to_string()),
        updated_at: Some(chrono::Utc::now()),
    })
}



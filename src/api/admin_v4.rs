// ============================================================================
// ADMIN ENDPOINTS - Configuration management without restart
// ============================================================================
//
// ENDPOINTS:
//   POST /api/v4/admin/update-dgi-captcha
//     Updates DGI MEF captcha token and session ID at runtime.
//     Body: { "captcha_token": "...", "session_id": "..." (optional) }
//
//   GET /api/v4/admin/dgi-config-status
//     Returns current DGI configuration status (lengths, not values).
//
// SECURITY:
//   - Requires valid JWT token
//   - Admin user_id validation (configurable via ADMIN_USER_IDS env var)
//
// USAGE EXAMPLE:
//   curl -X POST https://api.example.com/api/v4/admin/update-dgi-captcha \
//     -H "Authorization: Bearer <jwt>" \
//     -H "Content-Type: application/json" \
//     -d '{"captcha_token": "0cAFcWeA6e...", "session_id": "abc123"}'
//
// ============================================================================

use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::api::common::{ApiError, ApiResponse};
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;
use axum::Extension;

// ============================================================================
// ADMIN USER VALIDATION
// ============================================================================

/// List of admin user IDs (loaded from env or hardcoded for now)
/// In production, this should come from database or environment variable
fn get_admin_user_ids() -> Vec<i64> {
    // Try to load from environment variable
    if let Ok(admin_ids) = std::env::var("ADMIN_USER_IDS") {
        admin_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect()
    } else {
        // Default admin IDs - UPDATE THESE for production
        vec![1, 2, 3] // user_id 1, 2, 3 are admins by default
    }
}

/// Validates if the current user is an admin
fn is_admin_user(user_id: i64) -> bool {
    let admin_ids = get_admin_user_ids();
    admin_ids.contains(&user_id)
}

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

#[derive(serde::Deserialize)]
pub struct UpdateDgiCaptchaRequest {
    /// The new reCAPTCHA token from DGI MEF
    pub captcha_token: String,
    /// Optional: The new ASP.NET_SessionId cookie
    pub session_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct UpdateDgiCaptchaResponse {
    pub message: String,
    pub captcha_token_length: usize,
    pub session_id_length: usize,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct DgiConfigStatusResponse {
    pub captcha_token_configured: bool,
    pub captcha_token_length: usize,
    pub session_id_configured: bool,
    pub session_id_length: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// POST /api/v4/admin/update-dgi-captcha
/// 
/// Updates the DGI captcha token and optionally session ID at runtime.
/// This allows updating the captcha without restarting the application.
/// 
/// SECURITY: Only admin users can access this endpoint.
/// 
/// Request body:
/// ```json
/// {
///   "captcha_token": "0cAFcWeA6e...",  // Required, reCAPTCHA token
///   "session_id": "abc123"              // Optional, ASP.NET_SessionId
/// }
/// ```
/// 
/// Response:
/// ```json
/// {
///   "success": true,
///   "data": {
///     "message": "DGI configuration updated successfully",
///     "captcha_token_length": 850,
///     "session_id_length": 24,
///     "updated_at": "2025-12-03T..."
///   }
/// }
/// ```
#[axum::debug_handler]
pub async fn update_dgi_captcha_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<UpdateDgiCaptchaRequest>,
) -> Result<Json<ApiResponse<UpdateDgiCaptchaResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let user_id = current_user.user_id;
    
    // SECURITY: Validate admin role
    if !is_admin_user(user_id) {
        error!("🚫 Unauthorized admin access attempt by user {}", user_id);
        return Err(ApiError::new("FORBIDDEN", "No tienes permisos de administrador"));
    }
    
    info!("🔐 Admin user {} is updating DGI captcha token", user_id);
    
    // Validate captcha token
    if request.captcha_token.is_empty() {
        return Err(ApiError::validation_error("captcha_token cannot be empty"));
    }
    
    if request.captcha_token.len() < 100 {
        warn!("⚠️ Captcha token seems too short ({} chars) - may be invalid", request.captcha_token.len());
    }
    
    // Update captcha token
    {
        let mut captcha = state.dgi_captcha_token.write().await;
        *captcha = request.captcha_token.clone();
    }
    
    info!("✅ DGI captcha token updated ({} chars) by admin user {}", 
          request.captcha_token.len(), user_id);
    
    // Update session ID if provided
    let session_id_length = if let Some(ref session_id) = request.session_id {
        if !session_id.is_empty() {
            let mut session = state.dgi_session_id.write().await;
            *session = session_id.clone();
            info!("✅ DGI session ID updated ({} chars)", session_id.len());
            session_id.len()
        } else {
            state.dgi_session_id.read().await.len()
        }
    } else {
        state.dgi_session_id.read().await.len()
    };
    
    let response_data = UpdateDgiCaptchaResponse {
        message: "DGI configuration updated successfully".to_string(),
        captcha_token_length: request.captcha_token.len(),
        session_id_length,
        updated_at: chrono::Utc::now(),
    };
    
    let response = ApiResponse {
        success: true,
        data: Some(response_data),
        error: None,
        request_id,
        timestamp: chrono::Utc::now(),
        execution_time_ms: Some(0),
        cached: false,
    };
    
    Ok(Json(response))
}

/// GET /api/v4/admin/dgi-config-status
/// 
/// Returns the current status of DGI configuration (token lengths, not the actual values).
#[axum::debug_handler]
pub async fn dgi_config_status_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<DgiConfigStatusResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let user_id = current_user.user_id;
    
    info!("🔍 User {} checking DGI config status", user_id);
    
    let captcha_len = state.dgi_captcha_token.read().await.len();
    let session_len = state.dgi_session_id.read().await.len();
    
    let response_data = DgiConfigStatusResponse {
        captcha_token_configured: captcha_len > 0,
        captcha_token_length: captcha_len,
        session_id_configured: session_len > 0,
        session_id_length: session_len,
        timestamp: chrono::Utc::now(),
    };
    
    let response = ApiResponse {
        success: true,
        data: Some(response_data),
        error: None,
        request_id,
        timestamp: chrono::Utc::now(),
        execution_time_ms: Some(0),
        cached: false,
    };
    
    Ok(Json(response))
}

// ============================================================================
// ROUTER
// ============================================================================

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/update-dgi-captcha", post(update_dgi_captcha_handler))
        .route("/dgi-config-status", get(dgi_config_status_handler))
}

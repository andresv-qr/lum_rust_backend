// ============================================================================
// UNIFIED AUTHENTICATION API ENDPOINT
// ============================================================================
// Date: September 19, 2025
// Purpose: Single endpoint for all authentication methods
// ============================================================================

use crate::{
    models::unified_auth::{
        UnifiedAuthRequest,
    },
    services::{
        google_service::GoogleService,
        token_service::TokenService,
        redis_service::RedisService,
    },
    state::AppState,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
    Json,
};

use std::sync::Arc;
use tracing::{info, error};

// ============================================================================
// ROUTER CREATION
// ============================================================================

/// Create unified authentication router
pub fn create_unified_auth_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/unified", post(unified_auth))
        .route("/unified/health", get(unified_auth_health))
        .route("/unified/config", get(unified_auth_config))
}

/// Unified authentication endpoint
/// 
/// Handles all authentication methods:
/// - Email/password login and registration
/// - Google OAuth2 authentication
/// - Account linking flows
/// 
/// # Examples
/// 
/// Email login:
/// ```json
/// {
///   "provider": "email",
///   "email": "user@example.com",
///   "password": "secure_password",
///   "create_if_not_exists": false
/// }
/// ```
/// 
/// Google OAuth:
/// ```json
/// {
///   "provider": "google", 
///   "id_token": "google_id_token_here",
///   "create_if_not_exists": true
/// }
/// ```
pub async fn unified_auth(
    State(app_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<UnifiedAuthRequest>,
) -> Result<Response, AuthError> {
    info!(
        provider = ?request.provider_data.provider_type(),
        create_if_not_exists = request.create_if_not_exists,
        has_linking_token = request.linking_token.is_some(),
        "🔐 Unified authentication request received"
    );

    // Extract client information for audit logging
    let user_agent = headers.get("user-agent")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            // Extract from client_info if provided in request
            request.client_info.as_ref()
                .and_then(|ci| ci.user_agent.as_deref())
        });
    
    let ip_address = headers.get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim())
        .or_else(|| {
            // Extract from client_info if provided in request
            request.client_info.as_ref()
                .and_then(|ci| ci.ip_address.as_deref())
        });

    // Basic email validation if provider is email
    if let crate::models::unified_auth::ProviderData::Email { email, .. } = &request.provider_data {
        if !email.contains('@') || email.len() < 5 {
            error!("❌ Invalid email format");
            return Err(AuthError::ValidationError("Invalid email format".to_string()));
        }
    }

    // Create services
    let google_client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let google_service = GoogleService::new(
        google_client_id,
        app_state.http_client.clone(),
        RedisService::from_pool(app_state.redis_pool.clone()),
    );
    
    let token_service = TokenService::new(
        RedisService::from_pool(app_state.redis_pool.clone()),
        chrono::Duration::hours(24),  // linking_token_ttl
        chrono::Duration::minutes(15), // verification_code_ttl
    );
    
    // Create unified auth service
    let auth_service = crate::services::unified_auth_simple::SimpleUnifiedAuthService::new(
        app_state.db_pool.clone(),
        google_service,
        token_service,
    );

    // Use the real authentication service with client info
    match auth_service.authenticate_with_client_info(&request, ip_address, user_agent).await {
        Ok(response) => Ok((StatusCode::OK, Json(response)).into_response()),
        Err(e) => {
            error!("Authentication failed: {}", e);
            let error_response = crate::models::unified_auth::UnifiedAuthResponse {
                result: crate::models::unified_auth::AuthResult::Error {
                    message: e.to_string(),
                    error_code: "AUTH_FAILED".to_string(),
                    retry_after: None,
                },
                metadata: crate::models::unified_auth::AuthMetadata {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    provider_used: "unknown".to_string(),
                    is_new_user: false,
                    linking_performed: false,
                    execution_time_ms: 0,
                    timestamp: chrono::Utc::now(),
                },
            };
            Ok((StatusCode::UNAUTHORIZED, Json(error_response)).into_response())
        }
    }
}

/// Health check for unified auth system
pub async fn unified_auth_health(
    State(app_state): State<Arc<AppState>>,
) -> Result<Response, AuthError> {
    let google_service = GoogleService::new(
        std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
        reqwest::Client::new(),
        RedisService::from_pool(app_state.redis_pool.clone()),
    );

    match google_service.health_check().await {
        Ok(health_status) => {
            if health_status.overall_healthy {
                Ok((StatusCode::OK, Json(serde_json::json!({
                    "status": "healthy",
                    "unified_auth": {
                        "google_oauth": health_status.overall_healthy,
                        "redis_connectivity": health_status.redis_connectivity,
                        "response_time_ms": health_status.response_time_ms,
                    },
                    "timestamp": chrono::Utc::now()
                }))).into_response())
            } else {
                Ok((StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                    "status": "unhealthy",
                    "unified_auth": {
                        "google_oauth": health_status.overall_healthy,
                        "redis_connectivity": health_status.redis_connectivity,
                        "cert_error": health_status.cert_error,
                        "redis_error": health_status.redis_error,
                    },
                    "timestamp": chrono::Utc::now()
                }))).into_response())
            }
        }
        Err(e) => {
            error!(error = %e, "❌ Health check failed");
            Ok((StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                "status": "unhealthy",
                "error": e.to_string(),
                "timestamp": chrono::Utc::now()
            }))).into_response())
        }
    }
}

/// Configuration info endpoint (for debugging)
pub async fn unified_auth_config() -> Result<Response, AuthError> {
    let google_client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "unified_auth_config": {
            "google": {
                "client_id_masked": mask_sensitive(&google_client_id),
                "enabled": !google_client_id.is_empty(),
            },
            "providers_supported": ["email", "google"],
            "features": {
                "account_linking": true,
                "email_verification": true,
                "audit_logging": true,
                "rate_limiting": true,
            }
        },
        "timestamp": chrono::Utc::now()
    }))).into_response())
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AuthError::ConfigurationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AuthError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
            "timestamp": chrono::Utc::now()
        }));

        (status, body).into_response()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn mask_sensitive(value: &str) -> String {
    if value.len() > 8 {
        format!("{}...{}", &value[..4], &value[value.len()-4..])
    } else if value.is_empty() {
        "not_set".to_string()
    } else {
        "***".to_string()
    }
}
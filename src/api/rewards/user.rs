// ============================================================================
// USER REDEMPTIONS ENDPOINT - Consultar redenciones del usuario
// ============================================================================

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    domains::rewards::models::{
        UserRedemptionItem, UserRedemptionStats, RedemptionError,
        // CancellationResponse, // Unused - CancelResponse defined locally
    },
    middleware::auth::CurrentUser,
    state::AppState,
};

/// Query parameters for listing redemptions
#[derive(Debug, Deserialize)]
pub struct RedemptionsQuery {
    /// Filter by status: 'pending', 'confirmed', 'cancelled', 'expired'
    pub status: Option<String>,
    /// Max results (default: 50, max: 100)
    pub limit: Option<i32>,
    /// Pagination offset (default: 0)
    pub offset: Option<i32>,
}

/// Response for user redemptions list
#[derive(Debug, Serialize)]
pub struct RedemptionsResponse {
    pub success: bool,
    pub redemptions: Vec<UserRedemptionItem>,
    pub stats: UserRedemptionStats,
    pub total_count: usize,
}

/// Response for single redemption detail
#[derive(Debug, Serialize)]
pub struct RedemptionDetailResponse {
    pub success: bool,
    pub redemption: UserRedemptionItem,
}

/// Response for cancellation
#[derive(Debug, Serialize)]
pub struct CancelResponse {
    pub success: bool,
    pub message: String,
    pub lumis_refunded: i32,
}

/// List user's redemptions with optional filters
/// 
/// # Endpoint
/// GET /api/v1/rewards/redemptions?status=pending&limit=20
/// 
/// # Authentication
/// Requires valid JWT token
/// 
/// # Query Parameters
/// - status: Filter by redemption status (optional)
/// - limit: Max results (default: 50)
/// - offset: Pagination offset (default: 0)
/// 
/// # Returns
/// - 200 OK: List of redemptions with stats
/// - 401 Unauthorized: Invalid token
/// - 500 Internal Server Error: Database error
pub async fn list_user_redemptions(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<RedemptionsQuery>,
) -> Result<Json<RedemptionsResponse>, ApiError> {
    let user_id = current_user.user_id as i32;
    
    info!(
        "Listing redemptions for user_id={} status={:?} limit={}",
        user_id,
        query.status,
        query.limit.unwrap_or(50)
    );
    
    // Get redemptions
    let redemptions = state.redemption_service
        .get_user_redemptions(
            user_id,
            query.status.clone(),
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            error!("Failed to list redemptions: {:?}", e);
            ApiError::from(e)
        })?;
    
    // Get stats
    let stats = state.redemption_service
        .get_user_redemption_stats(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get redemption stats: {:?}", e);
            ApiError::from(e)
        })?;
    
    let total_count = redemptions.len();
    
    info!("Successfully retrieved {} redemptions", total_count);
    
    Ok(Json(RedemptionsResponse {
        success: true,
        redemptions,
        stats,
        total_count,
    }))
}

/// Get detailed information about a specific redemption
/// 
/// # Endpoint
/// GET /api/v1/rewards/redemptions/:id
/// 
/// # Authentication
/// Requires valid JWT token
/// 
/// # Path Parameters
/// - id: UUID of the redemption
/// 
/// # Returns
/// - 200 OK: Detailed redemption information
/// - 401 Unauthorized: Invalid token
/// - 404 Not Found: Redemption doesn't exist or doesn't belong to user
/// - 500 Internal Server Error: Database error
pub async fn get_redemption_detail(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path(redemption_id): Path<Uuid>,
) -> Result<Json<RedemptionDetailResponse>, ApiError> {
    let user_id = current_user.user_id as i32;
    
    info!(
        "Getting redemption detail: redemption_id={} user_id={}",
        redemption_id, user_id
    );
    
    // Get redemption and verify ownership
    let redemption = state.redemption_service
        .get_redemption_by_id(redemption_id, user_id)
        .await
        .map_err(|e| {
            error!("Failed to get redemption detail: {:?}", e);
            ApiError::from(e)
        })?;
    
    info!("Successfully retrieved redemption: {}", redemption.offer_name);
    
    Ok(Json(RedemptionDetailResponse {
        success: true,
        redemption,
    }))
}

/// Cancel a redemption and refund Lümis
/// 
/// # Endpoint
/// DELETE /api/v1/rewards/redemptions/:id
/// 
/// # Authentication
/// Requires valid JWT token
/// 
/// # Path Parameters
/// - id: UUID of the redemption to cancel
/// 
/// # Returns
/// - 200 OK: Redemption cancelled and Lümis refunded
/// - 400 Bad Request: Cannot cancel (already confirmed/cancelled/expired)
/// - 401 Unauthorized: Invalid token
/// - 404 Not Found: Redemption doesn't exist or doesn't belong to user
/// - 500 Internal Server Error: Database error
pub async fn cancel_redemption(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path(redemption_id): Path<Uuid>,
) -> Result<Json<CancelResponse>, ApiError> {
    let user_id = current_user.user_id as i32;
    
    info!(
        "Cancelling redemption: redemption_id={} user_id={}",
        redemption_id, user_id
    );
    
    // Cancel redemption
    let result = state.redemption_service
        .cancel_redemption(redemption_id, user_id)
        .await
        .map_err(|e| {
            error!("Failed to cancel redemption: {:?}", e);
            ApiError::from(e)
        })?;
    
    info!(
        "Successfully cancelled redemption: refunded {} Lümis",
        result.lumis_refunded
    );
    
    Ok(Json(CancelResponse {
        success: true,
        message: result.message,
        lumis_refunded: result.lumis_refunded,
    }))
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// API Error wrapper for HTTP responses
#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl From<RedemptionError> for ApiError {
    fn from(err: RedemptionError) -> Self {
        match err {
            RedemptionError::RedemptionNotFound => {
                ApiError::NotFound("Redención no encontrada".to_string())
            }
            RedemptionError::Database(msg) => ApiError::InternalError(msg),
            RedemptionError::CannotCancel { status } => {
                ApiError::BadRequest(format!(
                    "No puedes cancelar una redención con estado: {}",
                    status
                ))
            }
            RedemptionError::InvalidRedemptionCode => {
                ApiError::BadRequest("Esta redención ya fue utilizada".to_string())
            }
            RedemptionError::CodeExpired => {
                ApiError::BadRequest("Esta redención ya expiró".to_string())
            }
            _ => ApiError::InternalError(format!("{:?}", err)),
        }
    }
}

/// Get user statistics and balance
/// 
/// # Endpoint
/// GET /api/v1/rewards/stats
/// 
/// # Authentication
/// Requires valid JWT token
/// 
/// # Returns
/// - 200 OK: User statistics including balance and redemption counts
/// - 401 Unauthorized: Invalid token
/// - 500 Internal Server Error: Database error
pub async fn get_user_stats(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<UserStatsResponse>, ApiError> {
    let user_id = current_user.user_id as i32;
    
    info!("Fetching stats for user_id={}", user_id);
    
    // Get user balance
    let balance = state.offer_service
        .get_user_balance(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user balance: {:?}", e);
            ApiError::from(e)
        })?;
    
    // Get redemption statistics
    let redemption_stats = state.redemption_service
        .get_user_redemption_stats(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get redemption stats: {:?}", e);
            ApiError::from(e)
        })?;
    
    Ok(Json(UserStatsResponse {
        success: true,
        balance: balance as i32,
        total_redemptions: redemption_stats.total_redemptions as i32,
        pending_redemptions: redemption_stats.pending as i32,
        confirmed_redemptions: redemption_stats.confirmed as i32,
        cancelled_redemptions: redemption_stats.cancelled as i32,
        total_lumis_spent: redemption_stats.total_lumis_spent as i32,
    }))
}

/// Response for user statistics
#[derive(Debug, Serialize)]
pub struct UserStatsResponse {
    pub success: bool,
    pub balance: i32,
    pub total_redemptions: i32,
    pub pending_redemptions: i32,
    pub confirmed_redemptions: i32,
    pub cancelled_redemptions: i32,
    pub total_lumis_spent: i32,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        
        let body = Json(serde_json::json!({
            "success": false,
            "error": message,
        }));
        
        (status, body).into_response()
    }
}


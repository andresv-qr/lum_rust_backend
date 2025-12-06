// ============================================================================
// REDEEM ENDPOINT - Crear redenciones de Lümis
// ============================================================================

use axum::{
    extract::State,
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
    domains::rewards::models::{CreateRedemptionRequest, RedemptionCreatedResponse, RedemptionError},
    middleware::auth::CurrentUser,
    state::AppState,
};

/// Request body for creating a redemption
#[derive(Debug, Deserialize)]
pub struct RedeemRequest {
    pub offer_id: Uuid,
}

/// Response structure for redemption creation
#[derive(Debug, Serialize)]
pub struct RedeemResponse {
    pub success: bool,
    pub redemption: RedemptionCreatedResponse,
}

/// Create a new redemption (redeem Lümis for an offer)
/// 
/// # Endpoint
/// POST /api/v1/rewards/redeem
/// 
/// # Authentication
/// Requires valid JWT token
/// 
/// # Request Body
/// ```json
/// {
///   "offer_id": "123e4567-e89b-12d3-a456-426614174000"
/// }
/// ```
/// 
/// # Returns
/// - 201 Created: Redemption created successfully with QR code
/// - 400 Bad Request: Insufficient balance, max redemptions reached, etc.
/// - 401 Unauthorized: Invalid token
/// - 404 Not Found: Offer doesn't exist
/// - 500 Internal Server Error: Database error
pub async fn create_redemption(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<RedeemRequest>,
) -> Result<(StatusCode, Json<RedeemResponse>), ApiError> {
    let user_id = current_user.user_id as i32;
    
    info!("Creating redemption for user_id={} offer_id={}", user_id, payload.offer_id);
    
    // Create redemption request
    let request = CreateRedemptionRequest {
        user_id,
        offer_id: payload.offer_id,
    };
    
    // Call service to create redemption
    let redemption = state.redemption_service
        .create_redemption(request, None) // No IP address for now
        .await
        .map_err(|e| {
            error!("Failed to create redemption: {:?}", e);
            ApiError::from(e)
        })?;
    
    info!(
        "Successfully created redemption: id={} code={}",
        redemption.redemption_id, redemption.redemption_code
    );
    
    Ok((
        StatusCode::CREATED,
        Json(RedeemResponse {
            success: true,
            redemption,
        }),
    ))
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
            RedemptionError::OfferNotFound => ApiError::NotFound("Offer not found".to_string()),
            RedemptionError::Database(msg) => ApiError::InternalError(msg),
            RedemptionError::InsufficientBalance { required, current } => {
                ApiError::BadRequest(format!(
                    "Saldo insuficiente. Necesitas {} Lümis, tienes {}",
                    required, current
                ))
            }
            RedemptionError::MaxRedemptionsReached { max, current } => {
                ApiError::BadRequest(format!(
                    "Límite de redenciones alcanzado: {}/{}",
                    current, max
                ))
            }
            RedemptionError::OfferInactive => {
                ApiError::BadRequest("La oferta no está activa actualmente".to_string())
            }
            RedemptionError::OutOfStock => {
                ApiError::BadRequest("La oferta no tiene stock disponible".to_string())
            }
            RedemptionError::QRGenerationFailed(msg) => {
                ApiError::InternalError(format!("Error generando QR: {}", msg))
            }
            _ => ApiError::InternalError(format!("{:?}", err)),
        }
    }
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

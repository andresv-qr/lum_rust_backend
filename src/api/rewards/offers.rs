// ============================================================================
// OFFERS ENDPOINT - Catálogo de ofertas para redimir Lümis
// ============================================================================

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    domains::rewards::models::{OfferFilters, OfferListItem, RedemptionOffer, RedemptionError},
    middleware::auth::CurrentUser,
    state::AppState,
};

/// Response structure for offers list
#[derive(Debug, Serialize)]
pub struct OffersResponse {
    pub success: bool,
    pub offers: Vec<OfferListItem>,
    pub total_count: usize,
}

/// List available redemption offers with filters
/// 
/// # Endpoint
/// GET /api/v1/rewards/offers?category=food&sort=cost_asc&limit=20
/// 
/// # Authentication
/// Requires valid JWT token in Authorization header
/// 
/// # Query Parameters
/// - category: Filter by offer category (optional)
/// - sort: Sort order - "cost_asc", "cost_desc", "newest" (optional)
/// - limit: Max results (default: 50, max: 100)
/// - offset: Pagination offset (default: 0)
/// 
/// # Returns
/// - 200 OK: List of offers available to user
/// - 401 Unauthorized: Invalid or missing token
/// - 500 Internal Server Error: Database or service error
pub async fn list_offers(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(filters): Query<OfferFilters>,
) -> Result<Json<OffersResponse>, ApiError> {
    // Get user_id from CurrentUser (already parsed by middleware)
    let user_id = current_user.user_id as i32; // Convert i64 to i32
    
    info!(
        "Listing offers for user_id={} with filters: category={:?}, sort={:?}, limit={}",
        user_id, filters.category, filters.sort, filters.limit.unwrap_or(50)
    );
    
    // Call service to get offers
    let offers = state.offer_service
        .list_offers(user_id, filters) // Fixed parameter order
        .await
        .map_err(|e| {
            error!("Failed to list offers: {:?}", e);
            ApiError::from(e)
        })?;
    
    let total_count = offers.len();
    
    info!("Successfully retrieved {} offers for user_id={}", total_count, user_id);
    
    Ok(Json(OffersResponse {
        success: true,
        offers,
        total_count,
    }))
}

/// Get detailed information about a specific offer
/// 
/// # Endpoint
/// GET /api/v1/rewards/offers/:id
/// 
/// # Authentication
/// Requires valid JWT token
/// 
/// # Path Parameters
/// - id: UUID of the offer
/// 
/// # Returns
/// - 200 OK: Detailed offer information
/// - 401 Unauthorized: Invalid token
/// - 404 Not Found: Offer doesn't exist
/// - 500 Internal Server Error: Database error
pub async fn get_offer_detail(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path(offer_id): Path<Uuid>,
) -> Result<Json<OfferDetailResponse>, ApiError> {
    let user_id = current_user.user_id as i32;
    
    info!("Getting offer detail for offer_id={} user_id={}", offer_id, user_id);
    
    let offer = state.offer_service
        .get_offer_details(offer_id, user_id)
        .await
        .map_err(|e| {
            error!("Failed to get offer detail: {:?}", e);
            ApiError::from(e)
        })?;
    
    info!("Successfully retrieved offer: {}", offer.name);
    
    Ok(Json(OfferDetailResponse {
        success: true,
        offer,
    }))
}

/// Response for offer detail
#[derive(Debug, Serialize)]
pub struct OfferDetailResponse {
    pub success: bool,
    pub offer: RedemptionOffer,
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
                    "Insufficient balance: required {} Lümis, available {}",
                    required, current
                ))
            }
            RedemptionError::MaxRedemptionsReached { max, current } => {
                ApiError::BadRequest(format!(
                    "Maximum redemptions reached: {}/{}",
                    current, max
                ))
            }
            RedemptionError::OfferInactive => {
                ApiError::BadRequest("Offer is not currently active".to_string())
            }
            RedemptionError::OutOfStock => {
                ApiError::BadRequest("Offer is out of stock".to_string())
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

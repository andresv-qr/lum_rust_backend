// ============================================================================
// REWARDS API MODULE - Sistema de Redención de Lümis
// ============================================================================

pub mod offers;
pub mod redeem;
pub mod user;

use axum::{
    routing::{get, post, delete},
    Router,
    middleware::from_fn,
};
use std::sync::Arc;

use crate::state::AppState;
use crate::middleware::extract_current_user;

/// Create rewards router with all endpoints
/// Returns Router<Arc<AppState>> to be compatible with create_api_router
/// Applies JWT authentication middleware to protect all endpoints
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Offer endpoints
        .route("/offers", get(offers::list_offers))
        .route("/offers/:id", get(offers::get_offer_detail))
        // Redemption creation
        .route("/redeem", post(redeem::create_redemption))
        // User redemptions management
        .route("/history", get(user::list_user_redemptions))  // Changed from /redemptions to /history
        .route("/history/:id", get(user::get_redemption_detail))
        .route("/history/:id", delete(user::cancel_redemption))
        // User statistics
        .route("/stats", get(user::get_user_stats))
        .layer(from_fn(extract_current_user)) // Apply JWT authentication
}

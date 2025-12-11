// ============================================================================
// REWARDS API MODULE - Sistema de Redención de Lümis
// ============================================================================

pub mod offers;
pub mod redeem;
pub mod user;
pub mod qr_static;
pub mod admin_offers;
pub mod admin_merchants;
pub mod reports;

use axum::{
    routing::{get, post, put, delete},
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
    // Protected routes requiring JWT
    let protected = Router::new()
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
        .layer(from_fn(extract_current_user)); // Apply JWT authentication
    
    // Admin routes (requires JWT + admin check inside handlers)
    let admin_offers_routes = Router::new()
        .nest("/admin/offers", admin_offers::admin_offers_router())
        .layer(from_fn(extract_current_user));
    
    // Admin merchants routes
    let admin_merchants_routes = Router::new()
        .route("/admin/merchants", get(admin_merchants::list_merchants))
        .route("/admin/merchants", post(admin_merchants::create_merchant))
        .route("/admin/merchants/:id", get(admin_merchants::get_merchant))
        .route("/admin/merchants/:id", put(admin_merchants::update_merchant))
        .route("/admin/merchants/:id", delete(admin_merchants::delete_merchant))
        .route("/admin/merchants/:id/activate", post(admin_merchants::activate_merchant))
        .route("/admin/merchants/:id/regenerate-key", post(admin_merchants::regenerate_api_key))
        .layer(from_fn(extract_current_user));
    
    // Admin reports routes
    let admin_reports_routes = Router::new()
        .route("/admin/reports", get(reports::admin_generate_report))
        .route("/admin/export/redemptions", get(reports::admin_export_redemptions))
        .layer(from_fn(extract_current_user));
    
    // Public routes (QR images don't need auth)
    let public = Router::new()
        .route("/qr/:filename", get(qr_static::serve_qr_image));
    
    // Merge all
    protected
        .merge(admin_offers_routes)
        .merge(admin_merchants_routes)
        .merge(admin_reports_routes)
        .merge(public)
}

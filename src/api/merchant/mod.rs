// ============================================================================
// MERCHANT API MODULE - Endpoints para comercios aliados
// ============================================================================

pub mod auth;
pub mod validate;
pub mod stats;
pub mod analytics;

use axum::{
    routing::{get, post},
    Router,
    middleware::from_fn,
};
use std::sync::Arc;

use crate::state::AppState;
use crate::middleware::extract_merchant;

/// Create merchant router with all endpoints
/// Returns Router<Arc<AppState>> to be compatible with create_api_router
pub fn router() -> Router<Arc<AppState>> {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/auth/login", post(auth::merchant_login));
    
    // Protected routes (require merchant JWT)
    let protected_routes = Router::new()
        .route("/validate", post(validate::validate_redemption))
        .route("/confirm/:id", post(validate::confirm_redemption))
        .route("/stats", get(stats::get_merchant_stats))
        .route("/analytics", get(analytics::get_merchant_analytics))
        .layer(from_fn(extract_merchant));
    
    // Merge both
    public_routes.merge(protected_routes)
}

use crate::state::AppState;
use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;

use super::handlers::{get_webhook, post_webhook};
use super::stats::get_webhook_stats;

/// Creates the webhook router for WhatsApp endpoints
pub fn create_webhook_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/webhookws", get(get_webhook).post(post_webhook))
        .route("/webhook-stats", get(get_webhook_stats))
        .with_state(app_state)
}

use crate::state::AppState;
use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::handlers::webhook_handler::{get_webhook, post_webhook};

/// Crea y devuelve el enrutador para los webhooks.
pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/webhookws", get(get_webhook).post(post_webhook))
        .with_state(app_state)
}

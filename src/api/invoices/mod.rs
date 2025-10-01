// Invoice Processing API Module
// Implements the robust API for DGI Panama invoice processing

pub mod handlers;
pub mod models;
pub mod validation;
pub mod scraper_service;
pub mod repository;
pub mod logging_service;
pub mod error_handling;

pub use handlers::*;
pub use models::*;
pub use error_handling::*;

use axum::{
    routing::post,
    Router,
};
use std::sync::Arc;
use crate::state::AppState;

/// Create the invoice processing router
pub fn create_invoice_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/process", post(process_invoice_handler))
}

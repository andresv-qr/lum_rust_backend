use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub mod api;
pub mod webhook;
pub mod models;
pub mod processing;
pub mod services;
pub mod state;
pub mod cache;
pub mod cache_key;
pub mod cache_ttl;
pub mod utils;
pub mod db;
pub mod tasks;

// New domain-driven architecture
pub mod domains;
pub mod shared;

// Production-ready modules
pub mod monitoring;
pub mod security;
pub mod optimization;
pub mod middleware;

// Tests module removed

use api::create_api_router;
use webhook::{get_webhook, post_webhook};
use state::AppState;
use security::{security_headers_middleware, rate_limiting_middleware, get_cors_layer};
use monitoring::endpoints::monitoring_router;

use axum::middleware as axum_middleware;

pub fn create_app_router(app_state: Arc<AppState>) -> Router {
    // Crear el router API con todas las rutas
    let api_router = create_api_router();
    
    // Crear el router principal y aplicar el estado
    Router::new()
        // Webhooks de WhatsApp
        .route("/webhookws", get(get_webhook))
        .route("/webhookws", post(post_webhook))
        // Endpoints de monitoreo (sin autenticación)
        .merge(monitoring_router())
        // API endpoints con estado
        .merge(api_router)
        // Aplicar el estado a todas las rutas
        .with_state(app_state.clone())
        // Aplicar middlewares que requieren estado DESPUÉS de .with_state()
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            rate_limiting_middleware
        ))
        // Middlewares sin estado
        .layer(TraceLayer::new_for_http())
        .layer(get_cors_layer())
        .layer(axum_middleware::from_fn(security_headers_middleware))
}


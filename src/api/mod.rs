pub mod models;  // API models for compatibility

pub mod auth;
pub mod users;
pub mod invoices;
pub mod qr;
pub mod performance;
pub mod common;
pub mod templates;
pub mod v4;
pub mod email_check_v4;
pub mod invoice_headers_v4;
pub mod qr_v4;
pub mod lumis_balance_v4;
pub mod movements_summary_v4;
pub mod qr_processing_v4;
pub mod invoices_v4;
pub mod profile_v4;
pub mod users_v4;
pub mod register_v4;
pub mod auth_v4;
pub mod unified_auth_v4;  // New unified authentication endpoint
pub mod daily_game;       // Daily constellation game
pub mod rewards;          // Redemption system - Lümis rewards
pub mod merchant;         // Merchant portal - Redemption validation

// Re-export models from main models module
pub use crate::models::{
    UnifiedAuthRequest, UnifiedAuthResponse, AuthResponseType,
    AuthTokens, VerificationRequired, LinkAccountRequest,
    VerifyEmailRequest, ResendVerificationRequest,
    AuthProviderLink, ProviderType, LinkMethod,
    AuthAuditLog, AuthEvent, AuthEventType
};
pub mod rewards_balance_v4;
pub mod user_profile_v4;
pub mod verification_v4;
pub mod url_processing_v4;
pub mod webscraping;
// pub mod webscraping_test_v4; // Removed - was a test module
pub mod database_persistence;
pub mod user_registration_v4;
pub mod invoice_query_v4;
pub mod root_v4;
pub mod system_v4;
pub mod user_metrics2_v4; // Nuevo módulo para métricas de usuario
pub mod rewards_v4; // Nuevo módulo para rewards y métricas de facturas
pub mod userdata_v4; // Nuevo módulo para datos de usuario desde dim_users
pub mod rewards_history_v4; // Nuevo módulo para historial de acumulaciones y redenciones
pub mod surveys_v4; // Nuevo módulo para encuestas y surveys
pub mod tinder_v4; // Módulo Lumimatch - preguntas tipo Tinder
pub mod gamification_v4; // Nuevo módulo para gamificación completa
pub mod ocr_iterative_v4; // Nuevo módulo para OCR iterativo
pub mod upload_ocr_v4; // Nuevo módulo para upload OCR endpoint
pub mod upload_ocr_retry_v4; // Nuevo módulo para retry de OCR con campos específicos
pub mod gamification_service; // Servicio de gamificación (cálculo y acreditación de Lumis)
pub mod user_issuers_v4; // Nuevo módulo para obtener issuers de un usuario
pub mod user_products_v4; // Nuevo módulo para obtener productos de un usuario
pub mod user_invoice_headers_v4; // Nuevo módulo para obtener headers de facturas de un usuario
pub mod user_invoice_details_v4; // Nuevo módulo para obtener detalles de facturas de un usuario
pub mod sync_status_v4; // Nuevo módulo para estado de sincronización
pub mod integrity_summary_v4; // Nuevo módulo para validación de integridad diaria
// pub mod invoices_version_v4; // DEPRECATED - dataset_version system removed
pub mod unified_password; // Nuevo módulo para gestión unificada de contraseñas
pub mod ofertasws_v4; // Nuevo módulo para ofertas WS con cache Redis
pub mod notifications_v4; // Sistema de notificaciones in-app y push
pub mod admin_v4; // Admin endpoints - DGI captcha configuration
pub mod ask_ai_v4; // AI endpoint for natural language queries
pub mod interpret_results_v4; // AI interpretation of query results

// NEW: Invoice processing module
pub mod invoice_processor; // New robust invoice processing API

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use axum::middleware::from_fn;

use crate::state::AppState;
use crate::middleware::{extract_current_user, request_limits_middleware, validate_upload_middleware};

// Helper functions to combine multiple routers
fn create_users_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        // Verification endpoints moved to public router for security design
        // Note: user_registration_v4 endpoints are in public router, not here
}

fn create_invoices_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        // Comentado temporalmente para evitar duplicación con el router público
        // .merge(url_processing_v4::router())
        // NOTE: invoice_query_v4 conflicts with user_invoice_headers/details - using sync-specific routes
        // .merge(invoice_query_v4::create_invoice_query_v4_router())
        .merge(user_issuers_v4::create_user_issuers_v4_router())
        .merge(user_products_v4::create_user_products_v4_router())
        .merge(user_invoice_headers_v4::create_user_invoice_headers_v4_router())
        .merge(user_invoice_details_v4::create_user_invoice_details_v4_router())
        .merge(integrity_summary_v4::create_integrity_summary_v4_router())
        // Sync status MUST be merged before invoices_v4 which has /:id route
        .merge(sync_status_v4::create_sync_status_v4_router())
        // .merge(invoices_version_v4::create_version_v4_router())  // DEPRECATED
        // IMPORTANTE: invoices_v4 tiene /:id que captura rutas genéricas, debe ir AL FINAL
        .merge(invoices_v4::create_invoices_v4_router())
        // Solo middlewares que NO requieren estado
        .layer(from_fn(validate_upload_middleware))
        .layer(from_fn(request_limits_middleware))
}

// NEW: Create invoice processing router (robust API)
fn create_invoice_processing_router() -> Router<Arc<AppState>> {
    invoice_processor::create_invoice_router()
}

// Rutas públicas (sin JWT)
fn create_public_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/api/v4/auth", auth_v4::create_auth_v4_router())
        .nest("/api/v4/auth", unified_auth_v4::create_unified_auth_router())  // New unified auth
        .merge(register_v4::create_register_v4_router())
        .merge(user_registration_v4::create_user_registration_v4_public_router())
        .merge(email_check_v4::create_email_check_v4_router())
        .nest("/api/v4/users", unified_password::create_unified_verification_v4_router())  // Unified verification system
        .merge(unified_password::create_unified_password_v4_router())
        // NEW: Add robust invoice processing API (public for WhatsApp integration)
        .nest("/api/invoices", create_invoice_processing_router())
}

// Rutas protegidas (aplican JWT)
fn create_protected_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/api/v4/users", create_users_v4_router())
        .nest("/api/v4/invoices", create_invoices_v4_router())
        .nest("/api/v4/lumis_balance", lumis_balance_v4::create_router())
        .nest("/api/v4/movements_summary", movements_summary_v4::create_router())
        .nest("/api/v4/qr_processing", qr_processing_v4::create_router())
        .merge(v4::create_v4_router())
        .merge(qr_v4::create_qr_v4_router())
        .merge(system_v4::create_system_v4_router())
        .merge(invoice_headers_v4::create_invoice_headers_v4_router())
        .merge(user_profile_v4::create_user_profile_v4_router())
        .merge(user_metrics2_v4::create_user_metrics2_v4_router())
        .merge(userdata_v4::create_userdata_v4_router())
        .merge(rewards_history_v4::create_rewards_history_v4_router())
        .merge(surveys_v4::create_surveys_v4_router())
        .merge(gamification_v4::create_gamification_v4_router())
        .merge(tinder_v4::create_tinder_router())
        .nest("/api/v4/rewards", rewards_v4::create_rewards_v4_router())
        // Notifications system endpoints
        .nest("/api/v4/notifications", notifications_v4::create_notifications_v4_router())
        // ADD: Protected URL processing endpoint with JWT authentication
        .route("/api/v4/invoices/process-from-url", post(url_processing_v4::process_url_handler))
        // ADD: Protected CUFE processing endpoint (for OCR-detected CUFE codes)
        .route("/api/v4/invoices/process-from-cufe", post(url_processing_v4::process_cufe_handler))
        // Daily Game endpoints (protected)
        .route("/api/v4/daily-game/claim", post(daily_game::handle_claim))
        .route("/api/v4/daily-game/status", get(daily_game::handle_status))
        // Ofertas WS endpoints
        .route("/api/v4/ofertasws", get(ofertasws_v4::get_ofertasws))
        .route("/api/v4/ofertasws/refresh", post(ofertasws_v4::refresh_ofertasws_cache))
        // Admin endpoints - DGI configuration
        .nest("/api/v4/admin", admin_v4::router())
        // AI endpoints
        .route("/api/v4/ask-ai", post(ask_ai_v4::ask_ai_data))
        .route("/api/v4/interpret-results", post(interpret_results_v4::interpret_results))
        .layer(from_fn(extract_current_user))
}

/// Creates the API router with all REST endpoints
pub fn create_api_router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(root_v4::create_root_v4_router())
        .merge(create_public_v4_router())
        .merge(create_protected_v4_router())
        // Redemption system endpoints (v1)
        .nest("/api/v1/rewards", rewards::router())
        .nest("/api/v1/merchant", merchant::router())
        // Legacy V3 endpoints - TEMPORARILY COMMENTED OUT DURING MIGRATION
        // .route("/api/v3/invoices/upload-ocr", post(invoices::upload_ocr_invoice))
        // .route("/api/v3/invoices/process-from-url", post(invoices::process_invoice_from_url))
        // .route("/api/v3/invoices/details", get(invoices::get_invoice_details))
        // .route("/api/v3/invoices/header", get(invoices::get_invoice_headers))
        .route("/api/v3/performance/metrics", get(performance::get_performance_metrics))
        .route("/api/v3/performance/cache", get(performance::get_cache_statistics))
        .route("/api/v3/performance/reset", post(performance::reset_performance_metrics))
}

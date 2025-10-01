use axum::{
    extract::{State, Request},
    http::StatusCode,
    response::Json,
    body::Body,
    routing::get,
    Router,
};
use std::sync::Arc;
use tracing::info;
use serde::Serialize;

use crate::state::AppState;
use crate::utils::get_request_id;

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct RootResponse {
    pub service: String,
    pub version: String,
    pub status: String,
    pub message: String,
    pub timestamp: String,
    pub endpoints: ApiEndpoints,
    pub features: Vec<String>,
    pub documentation: DocumentationLinks,
}

#[derive(Debug, Serialize)]
pub struct ApiEndpoints {
    pub authentication: Vec<EndpointInfo>,
    pub users: Vec<EndpointInfo>,
    pub invoices: Vec<EndpointInfo>,
    pub rewards: Vec<EndpointInfo>,
    pub webscraping: Vec<EndpointInfo>,
    pub persistence: Vec<EndpointInfo>,
}

#[derive(Debug, Serialize)]
pub struct EndpointInfo {
    pub method: String,
    pub path: String,
    pub description: String,
    pub auth_required: bool,
}

#[derive(Debug, Serialize)]
pub struct DocumentationLinks {
    pub api_docs: String,
    pub endpoints: String,
    pub migration_report: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub timestamp: String,
    pub version: String,
    pub uptime_seconds: u64,
}

// ============================================================================
// API HANDLERS
// ============================================================================

/// Root endpoint providing API overview and available endpoints
pub async fn root_endpoint(
    State(_state): State<Arc<AppState>>,
    request: Request<Body>,
) -> Result<Json<RootResponse>, StatusCode> {
    let request_id = get_request_id(&request);
    
    info!("Request {}: Root endpoint accessed", request_id);
    
    let response = RootResponse {
        service: "LÜM Invoice Processing API".to_string(),
        version: "4.0.0".to_string(),
        status: "operational".to_string(),
        message: "Welcome to LÜM Invoice Processing API v4 - Rust Native Implementation".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        endpoints: create_endpoints_info(),
        features: vec![
            "Native Rust webscraping".to_string(),
            "Real-time invoice processing".to_string(),
            "Complete database persistence".to_string(),
            "JWT authentication".to_string(),
            "User management".to_string(),
            "Rewards processing".to_string(),
            "Rate limiting".to_string(),
            "Request tracing".to_string(),
            "Performance optimized".to_string(),
        ],
        documentation: DocumentationLinks {
            api_docs: "/api/v4/docs".to_string(),
            endpoints: "/api/v4/endpoints".to_string(),
            migration_report: "/migration-report".to_string(),
        },
    };
    
    Ok(Json(response))
}

// Health check functionality moved to monitoring system (src/monitoring/mod.rs)

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_endpoints_info() -> ApiEndpoints {
    ApiEndpoints {
        authentication: vec![
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v3/auth/login".to_string(),
                description: "User authentication with email and password".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v3/auth/check-status".to_string(),
                description: "Check user authentication status".to_string(),
                auth_required: false,
            },
        ],
        users: vec![
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/users/register".to_string(),
                description: "Register new user account".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/users/check-email".to_string(),
                description: "Check if email is already registered".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/users/send-verification".to_string(),
                description: "Send email verification code".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/users/verify-account".to_string(),
                description: "Verify user account with code".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/passwords/request-code".to_string(),
                description: "Request verification code for password operations".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/passwords/set-with-code".to_string(),
                description: "Set password using verification code (ALL password changes)".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/profile".to_string(),
                description: "Get user profile information".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/users/profile".to_string(),
                description: "Get user profile by email".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/invoices/issuers".to_string(),
                description: "Get issuers that user has invoices with".to_string(),
                auth_required: true,
            },
        ],
        invoices: vec![
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/invoices/upload-ocr".to_string(),
                description: "Upload invoice image for OCR processing".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/invoices/process-from-url".to_string(),
                description: "Process invoice from URL (CUFE/QR)".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/invoices/validate-url".to_string(),
                description: "Validate invoice URL format".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/invoices/processing-stats".to_string(),
                description: "Get invoice processing statistics".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/invoices/details".to_string(),
                description: "Get invoice details with filters".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/invoices/headers".to_string(),
                description: "Get invoice headers with filters".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/invoices/products".to_string(),
                description: "Get products that user has purchased".to_string(),
                auth_required: true,
            },
        ],
        rewards: vec![
            EndpointInfo {
                method: "GET".to_string(),
                path: "/api/v4/rewards/balance".to_string(),
                description: "Get user rewards balance and points".to_string(),
                auth_required: true,
            },
        ],
        webscraping: vec![
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/webscraping/test".to_string(),
                description: "Test webscraping functionality (no auth)".to_string(),
                auth_required: false,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/webscraping/info".to_string(),
                description: "Get webscraping service information".to_string(),
                auth_required: false,
            },
        ],
        persistence: vec![
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/persistence/process-with-persistence".to_string(),
                description: "Process invoice with complete database persistence".to_string(),
                auth_required: true,
            },
            EndpointInfo {
                method: "POST".to_string(),
                path: "/api/v4/persistence/user-stats".to_string(),
                description: "Get user invoice statistics".to_string(),
                auth_required: true,
            },
        ],
    }
}

// ============================================================================
// ROUTER CREATION
// ============================================================================

pub fn create_root_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root_endpoint))
        // /health route moved to monitoring system in api/mod.rs
}

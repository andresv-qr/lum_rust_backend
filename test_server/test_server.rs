// ============================================================================
// MINIMAL SERVER FOR UNIFIED AUTH TESTING
// ============================================================================
// Date: December 2024
// Purpose: Test server with only unified auth endpoint

use axum::{
    extract::{State, Json},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use sqlx::PgPool;
use deadpool_redis::{Pool as RedisPool, Manager};

// State for the application
#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub redis_pool: RedisPool,
}

// Simple request/response models
#[derive(Debug, Deserialize)]
pub struct SimpleAuthRequest {
    pub provider: String,
    pub email: Option<String>,
    pub password: Option<String>,
    pub google_id_token: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SimpleAuthResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<i64>,
    pub token: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub database: String,
    pub redis: String,
}

// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    // Simple DB check
    let db_status = match sqlx::query("SELECT 1").execute(&state.db_pool).await {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("error: {}", e),
    };

    // Simple Redis check
    let redis_status = match state.redis_pool.get().await {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("error: {}", e),
    };

    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        database: db_status,
        redis: redis_status,
    }))
}

// Simplified auth endpoint
async fn unified_auth(
    State(state): State<AppState>,
    Json(request): Json<SimpleAuthRequest>,
) -> Result<Json<SimpleAuthResponse>, StatusCode> {
    println!("🔑 Auth request received: provider={}, email={:?}", 
             request.provider, request.email);

    match request.provider.as_str() {
        "email" => {
            let email = request.email.ok_or(StatusCode::BAD_REQUEST)?;
            let password = request.password.ok_or(StatusCode::BAD_REQUEST)?;
            
            // Simple email auth logic
            let user_query = sqlx::query!(
                "SELECT id, email, password_hash, name FROM dim_users WHERE email = $1 LIMIT 1",
                email
            )
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| {
                eprintln!("Database error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            if let Some(user) = user_query {
                // Verify password
                let is_valid = bcrypt::verify(&password, &user.password_hash.unwrap_or_default())
                    .unwrap_or(false);
                
                if is_valid {
                    Ok(Json(SimpleAuthResponse {
                        success: true,
                        message: "Login successful".to_string(),
                        user_id: Some(user.id),
                        token: Some(format!("token_{}", user.id)),
                        provider: Some("email".to_string()),
                    }))
                } else {
                    Ok(Json(SimpleAuthResponse {
                        success: false,
                        message: "Invalid credentials".to_string(),
                        user_id: None,
                        token: None,
                        provider: None,
                    }))
                }
            } else {
                // User not found, could implement registration here
                Ok(Json(SimpleAuthResponse {
                    success: false,
                    message: "User not found".to_string(),
                    user_id: None,
                    token: None,
                    provider: None,
                }))
            }
        }
        "google" => {
            Ok(Json(SimpleAuthResponse {
                success: false,
                message: "Google auth not implemented in minimal server".to_string(),
                user_id: None,
                token: None,
                provider: None,
            }))
        }
        _ => {
            Ok(Json(SimpleAuthResponse {
                success: false,
                message: "Unsupported provider".to_string(),
                user_id: None,
                token: None,
                provider: None,
            }))
        }
    }
}

// Root endpoint
async fn root() -> &'static str {
    "Unified Auth Test Server - v1.0"
}

// Create the router
fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/v4/auth/login", post(unified_auth))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 Starting Unified Auth Test Server...");

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://avalencia:Valencia123@localhost/tfactu".to_string());
    
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✅ Database connected");

    // Redis connection
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    let redis_manager = Manager::new(redis_url)?;
    let redis_pool = deadpool_redis::Pool::builder(redis_manager)
        .max_size(5)
        .build()
        .map_err(|e| format!("Failed to create Redis pool: {}", e))?;

    println!("✅ Redis connected");

    // Create app state
    let state = AppState {
        db_pool,
        redis_pool,
    };

    // Create router
    let app = create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    println!("🌟 Server running on http://127.0.0.1:8000");
    println!("📋 Available endpoints:");
    println!("   GET  / - Root");
    println!("   GET  /health - Health check");
    println!("   POST /api/v4/auth/login - Unified authentication");

    axum::serve(listener, app).await?;

    Ok(())
}
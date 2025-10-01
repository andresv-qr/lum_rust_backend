//! API Gateway for Lüm microservices
//! 
//! This service acts as the main entry point for all client requests,
//! routing them to the appropriate microservices and handling:
//! - Authentication and authorization
//! - Rate limiting
//! - Request/response transformation
//! - Load balancing
//! - Circuit breaking

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use base64::{Engine as _, engine::general_purpose};
use shared::{
    auth::{AuthService, middleware::auth_middleware},
    cache::RedisService,
    config::Config,
    database::DatabaseService,
    error::AppError,
    service_client::*,
    types::*,
    Result,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    timeout::TimeoutLayer,
};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub auth_service: Arc<AuthService>,
    pub redis_service: Arc<RedisService>,
    pub database_service: Arc<DatabaseService>,
    pub qr_detection_client: Arc<QrDetectionClient>,
    pub ocr_processing_client: Arc<OcrProcessingClient>,
    pub rewards_engine_client: Arc<RewardsEngineClient>,
    pub user_management_client: Arc<UserManagementClient>,
    pub notification_client: Arc<NotificationClient>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let config = Config::from_env()?;
        
        // Initialize services
        let auth_service = Arc::new(AuthService::new(&config.auth)?);
        let redis_service = Arc::new(RedisService::new(&config.redis).await?);
        let database_service = Arc::new(DatabaseService::new(&config.database).await?);
        
        // Initialize service clients
        let qr_detection_client = Arc::new(QrDetectionClient::new(config.services.qr_detection_url.clone())?);
        let ocr_processing_client = Arc::new(OcrProcessingClient::new(config.services.ocr_processing_url.clone())?);
        let rewards_engine_client = Arc::new(RewardsEngineClient::new(config.services.rewards_engine_url.clone())?);
        let user_management_client = Arc::new(UserManagementClient::new(config.services.user_management_url.clone())?);
        let notification_client = Arc::new(NotificationClient::new(config.services.notification_url.clone())?);

        Ok(Self {
            config,
            auth_service,
            redis_service,
            database_service,
            qr_detection_client,
            ocr_processing_client,
            rewards_engine_client,
            user_management_client,
            notification_client,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    info!("Starting API Gateway...");

    // Initialize application state
    let app_state = AppState::new().await?;

    // Create router
    let app = create_router(app_state);

    // Start server
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .unwrap_or(8000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("API Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // WhatsApp webhook endpoints (preserve compatibility)
        .route("/webhookws", get(webhook_verify))
        .route("/webhookws", post(webhook_handler))
        
        // Telegram webhook endpoints
        .route("/webhook/:token", post(telegram_webhook_handler))
        
        // QR Detection endpoints
        .route("/api/v1/qr/detect", post(qr_detect_handler))
        
        // OCR Processing endpoints
        .route("/api/v1/ocr/process", post(ocr_process_handler))
        
        // Rewards endpoints
        .route("/api/v1/rewards/accumulate", post(rewards_accumulate_handler))
        .route("/api/v1/rewards/redeem", post(rewards_redeem_handler))
        .route("/api/v1/rewards/balance/:user_id", get(rewards_balance_handler))
        
        // User management endpoints
        .route("/api/v1/users/register", post(user_register_handler))
        .route("/api/v1/users/lookup", post(user_lookup_handler))
        
        // Notification endpoints
        .route("/api/v1/notifications/send", post(notification_send_handler))
        
        // Service health endpoints
        .route("/api/v1/services/health", get(services_health_handler))
        
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(middleware::from_fn_with_state(
                    Arc::new(AuthService::new(&Config::from_env().unwrap().auth).unwrap()),
                    auth_middleware,
                ))
        )
}

// Health check handler
async fn health_check(State(state): State<AppState>) -> Result<Json<HealthStatus>> {
    let mut dependencies = Vec::new();

    // Check database
    let db_status = match state.database_service.health_check().await {
        Ok(_) => ServiceStatus::Healthy,
        Err(_) => ServiceStatus::Unhealthy,
    };
    dependencies.push(DependencyStatus {
        name: "database".to_string(),
        status: db_status,
        response_time_ms: None,
        error: None,
    });

    // Check Redis
    let redis_status = match state.redis_service.health_check().await {
        Ok(_) => ServiceStatus::Healthy,
        Err(_) => ServiceStatus::Unhealthy,
    };
    dependencies.push(DependencyStatus {
        name: "redis".to_string(),
        status: redis_status,
        response_time_ms: None,
        error: None,
    });

    let overall_status = if dependencies.iter().all(|d| matches!(d.status, ServiceStatus::Healthy)) {
        ServiceStatus::Healthy
    } else {
        ServiceStatus::Degraded
    };

    Ok(Json(HealthStatus {
        service: "api-gateway".to_string(),
        status: overall_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
        dependencies,
    }))
}

// WhatsApp webhook verification
async fn webhook_verify(Query(params): Query<std::collections::HashMap<String, String>>) -> Result<String> {
    if let (Some(mode), Some(token), Some(challenge)) = (
        params.get("hub.mode"),
        params.get("hub.verify_token"),
        params.get("hub.challenge"),
    ) {
        let verify_token = std::env::var("VERIFY_TOKEN").unwrap_or_default();
        if mode == "subscribe" && token == &verify_token {
            info!("WhatsApp webhook verified successfully");
            return Ok(challenge.clone());
        }
    }
    Err(AppError::authentication("Invalid webhook verification"))
}

// WhatsApp webhook handler
async fn webhook_handler(
    State(_state): State<AppState>,
    _headers: HeaderMap,
    _body: String,
) -> Result<Json<serde_json::Value>> {
    info!("Received WhatsApp webhook");
    
    // TODO: Process WhatsApp webhook
    // This would involve parsing the webhook payload and routing to appropriate services
    
    Ok(Json(serde_json::json!({"status": "received"})))
}

// Telegram webhook handler
async fn telegram_webhook_handler(
    Path(token): Path<String>,
    State(_state): State<AppState>,
    _body: String,
) -> Result<Json<serde_json::Value>> {
    info!("Received Telegram webhook for token: {}", token);
    
    // TODO: Process Telegram webhook
    // This would involve parsing the webhook payload and routing to appropriate services
    
    Ok(Json(serde_json::json!({"ok": true})))
}

// QR Detection handler
async fn qr_detect_handler(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<QrDetectionResult>> {
    let image_data = payload.get("image_data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::validation("Missing image_data field"))?;

    let decoded_data = general_purpose::STANDARD.decode(image_data)
        .map_err(|_| AppError::validation("Invalid base64 image data"))?;

    let result = state.qr_detection_client.detect_qr(&decoded_data).await?;
    Ok(Json(result))
}

// OCR Processing handler
async fn ocr_process_handler(
    State(state): State<AppState>,
    Json(request): Json<OcrRequest>,
) -> Result<Json<OcrResult>> {
    let result = state.ocr_processing_client.process_ocr(&request).await?;
    Ok(Json(result))
}

// Rewards accumulation handler
async fn rewards_accumulate_handler(
    State(state): State<AppState>,
    Json(request): Json<RewardsAccumulationRequest>,
) -> Result<Json<RewardsAccumulationResult>> {
    let result = state.rewards_engine_client.process_accumulation(&request).await?;
    Ok(Json(result))
}

// Rewards redemption handler
async fn rewards_redeem_handler(
    State(state): State<AppState>,
    Json(request): Json<RewardsRedemptionRequest>,
) -> Result<Json<RewardsRedemptionResult>> {
    let result = state.rewards_engine_client.process_redemption(&request).await?;
    Ok(Json(result))
}

// Rewards balance handler
async fn rewards_balance_handler(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let balance = state.rewards_engine_client.get_user_balance(user_id).await?;
    Ok(Json(serde_json::json!({"balance": balance})))
}

// User registration handler
async fn user_register_handler(
    State(state): State<AppState>,
    Json(request): Json<UserRegistrationRequest>,
) -> Result<Json<UserInfo>> {
    let user = state.user_management_client.register_user(&request).await?;
    Ok(Json(user))
}

// User lookup handler
async fn user_lookup_handler(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Option<UserInfo>>> {
    let user_id = payload.get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::validation("Missing user_id field"))?;
    
    let source: AppSource = payload.get("source")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| AppError::validation("Missing or invalid source field"))?;

    let user = state.user_management_client.get_user(user_id, &source).await?;
    Ok(Json(user))
}

// Notification send handler
async fn notification_send_handler(
    State(state): State<AppState>,
    Json(request): Json<NotificationRequest>,
) -> Result<Json<serde_json::Value>> {
    state.notification_client.send_notification(&request).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

// Services health handler
async fn services_health_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>> {
    let mut services = serde_json::Map::new();

    // Check QR Detection service
    match state.qr_detection_client.health().await {
        Ok(health) => {
            services.insert("qr-detection".to_string(), serde_json::to_value(health)?);
        }
        Err(e) => {
            services.insert("qr-detection".to_string(), serde_json::json!({
                "status": "unhealthy",
                "error": e.to_string()
            }));
        }
    }

    // Check other services similarly...
    // (OCR, Rewards, User Management, Notification)

    Ok(Json(serde_json::Value::Object(services)))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("signal received, starting graceful shutdown");
}
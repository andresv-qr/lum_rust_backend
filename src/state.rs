// use crate::services::redis_service::create_redis_client; // Not needed with optimized config
use crate::domains::qr::service::QrService;
use crate::cache::UserCache;
use crate::shared::performance::{PerformanceManager, PerformanceConfig};
use crate::optimization::{DatabaseConfig, RedisConfig, create_optimized_db_pool, create_optimized_redis_client};
use crate::webhook::MessageDeduplicator;
use dashmap::DashMap;
use redis::Client as RedisClient;
use reqwest::Client as ReqwestClient;
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use std::time::Instant;

/// Estado compartido de la aplicación.
/// Contiene las conexiones a Redis, base de datos y cliente HTTP.

// Optimized structure for tracking processed messages with TTL
#[derive(Clone)]
pub struct ProcessedMessage {
    pub timestamp: Instant,
}

#[derive(Clone)]
pub struct AppState {
    pub redis_client: RedisClient,
    pub redis_pool: deadpool_redis::Pool,  // Add Redis pool for unified auth
    pub http_client: ReqwestClient,
    pub user_cache: UserCache,
    pub processed_messages: Arc<DashMap<String, ProcessedMessage>>,
    pub db_pool: PgPool,
    pub ws_pool: Option<PgPool>, // WS database pool for ofertas (optional)
    pub whatsapp_token: String,
    pub phone_number_id: String,
    pub whatsapp_api_base_url: String,
    pub qr_service: QrService,
    pub performance_manager: Arc<PerformanceManager>,
    pub message_deduplicator: MessageDeduplicator,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        // Create optimized database pool with production-ready configuration
        let database_url = env::var("DATABASE_URL").map_err(|e| anyhow::anyhow!("DATABASE_URL must be set: {}", e))?;
        let db_config = DatabaseConfig::production();
        let db_pool = create_optimized_db_pool(&database_url, db_config).await?;
        
        // Create optimized Redis client with connection pooling
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let redis_config = RedisConfig::default();
        let redis_client = create_optimized_redis_client(&redis_url, redis_config)?;

        // Create Redis pool for unified auth services
        let redis_pool = deadpool_redis::Config::from_url(&redis_url)
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| anyhow::anyhow!("Failed to create Redis pool: {}", e))?;

        let whatsapp_token = env::var("WHATSAPP_TOKEN").map_err(|e| anyhow::anyhow!("WHATSAPP_TOKEN must be set: {}", e))?;
        let phone_number_id = env::var("PHONE_NUMBER_ID").map_err(|e| anyhow::anyhow!("PHONE_NUMBER_ID must be set: {}", e))?;
        let whatsapp_api_base_url = env::var("WHATSAPP_API_BASE_URL").unwrap_or_else(|_| "https://graph.facebook.com/v18.0".to_string());

        // Optimized HTTP client configuration
        // Create a shared cookie store for the HTTP client
        let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
        
        let http_client = ReqwestClient::builder()
            .cookie_provider(cookie_jar)
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build Reqwest client");

        let user_cache = UserCache::new();
        
        // Initialize QR service with Python fallback URL
        let python_fallback_url = env::var("PYTHON_QR_FALLBACK_URL")
            .unwrap_or_else(|_| "http://localhost:8008".to_string());
        let qr_service = QrService::new(python_fallback_url);

        // Initialize PerformanceManager with configuration from environment
        let performance_config = PerformanceConfig::from_env();
        let performance_manager = Arc::new(PerformanceManager::new(performance_config));
        
        // Initialize MessageDeduplicator
        let message_deduplicator = MessageDeduplicator::default();
        
        // Warm up connections and caches
        if let Err(e) = performance_manager.warm_up(&db_pool, &redis_client).await {
            tracing::warn!("⚠️ Performance warm-up failed: {}", e);
        }

        // Create WS database pool if WS_DATABASE_URL is set
        let ws_pool = if let Ok(ws_url) = env::var("WS_DATABASE_URL") {
            match crate::db::create_ws_pool().await {
                Ok(pool) => {
                    tracing::info!("✅ WS database pool initialized for ofertas");
                    Some(pool)
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to create WS pool: {}. Ofertas API will not be available.", e);
                    None
                }
            }
        } else {
            tracing::info!("ℹ️ WS_DATABASE_URL not set. Ofertas API will not be available.");
            None
        };

        Ok(AppState {
            db_pool,
            redis_client,
            redis_pool,  // Add Redis pool to AppState
            http_client,
            user_cache,
            whatsapp_api_base_url,
            qr_service,
            performance_manager,
            message_deduplicator,
            processed_messages: Arc::new(DashMap::new()),
            whatsapp_token,
            phone_number_id,
            ws_pool,
        })
    }
}

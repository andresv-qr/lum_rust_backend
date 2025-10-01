//! Configuration management for all microservices

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub services: ServicesConfig,
    pub app: AppConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_seconds: u64,
    pub refresh_token_ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub api_gateway_url: String,
    pub qr_detection_url: String,
    pub ocr_processing_url: String,
    pub rewards_engine_url: String,
    pub user_management_url: String,
    pub notification_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub environment: String,
    pub log_level: String,
    pub max_request_size_mb: u64,
    pub request_timeout_seconds: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://localhost:5432/lumis_db".to_string()),
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "20".to_string())
                    .parse()?,
                min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()?,
                acquire_timeout_seconds: env::var("DATABASE_ACQUIRE_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()?,
                idle_timeout_seconds: env::var("DATABASE_IDLE_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "600".to_string())
                    .parse()?,
                max_lifetime_seconds: env::var("DATABASE_MAX_LIFETIME_SECONDS")
                    .unwrap_or_else(|_| "1800".to_string())
                    .parse()?,
            },
            redis: RedisConfig {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                pool_size: env::var("REDIS_POOL_SIZE")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()?,
                connection_timeout_seconds: env::var("REDIS_CONNECTION_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "5".to_string())
                    .parse()?,
            },
            auth: AuthConfig {
                jwt_secret: env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "your-super-secret-jwt-key-here".to_string()),
                access_token_ttl_seconds: env::var("JWT_ACCESS_TOKEN_TTL_SECONDS")
                    .unwrap_or_else(|_| "900".to_string())
                    .parse()?,
                refresh_token_ttl_seconds: env::var("JWT_REFRESH_TOKEN_TTL_SECONDS")
                    .unwrap_or_else(|_| "604800".to_string())
                    .parse()?,
            },
            services: ServicesConfig {
                api_gateway_url: env::var("API_GATEWAY_URL")
                    .unwrap_or_else(|_| "http://localhost:8000".to_string()),
                qr_detection_url: env::var("QR_DETECTION_SERVICE_URL")
                    .unwrap_or_else(|_| "http://localhost:8001".to_string()),
                ocr_processing_url: env::var("OCR_PROCESSING_SERVICE_URL")
                    .unwrap_or_else(|_| "http://localhost:8002".to_string()),
                rewards_engine_url: env::var("REWARDS_ENGINE_SERVICE_URL")
                    .unwrap_or_else(|_| "http://localhost:8003".to_string()),
                user_management_url: env::var("USER_MANAGEMENT_SERVICE_URL")
                    .unwrap_or_else(|_| "http://localhost:8005".to_string()),
                notification_url: env::var("NOTIFICATION_SERVICE_URL")
                    .unwrap_or_else(|_| "http://localhost:8006".to_string()),
            },
            app: AppConfig {
                environment: env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".to_string()),
                log_level: env::var("RUST_LOG")
                    .unwrap_or_else(|_| "info".to_string()),
                max_request_size_mb: env::var("MAX_REQUEST_SIZE_MB")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()?,
                request_timeout_seconds: env::var("REQUEST_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()?,
            },
        })
    }

    pub fn is_production(&self) -> bool {
        self.app.environment == "production"
    }

    pub fn is_development(&self) -> bool {
        self.app.environment == "development"
    }
}
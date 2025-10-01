//! Redis cache service for all microservices

use crate::{config::RedisConfig, error::AppError, Result};
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone)]
pub struct RedisService {
    connection: MultiplexedConnection,
}

impl RedisService {
    pub async fn new(config: &RedisConfig) -> Result<Self> {
        info!("Initializing Redis connection");

        let client = Client::open(config.url.as_str())
            .map_err(|e| AppError::configuration(format!("Failed to create Redis client: {}", e)))?;

        let connection = client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| AppError::configuration(format!("Failed to connect to Redis: {}", e)))?;

        // Test the connection
        let mut conn = connection.clone();
        let _: String = redis::cmd("PING").query_async(&mut conn).await
            .map_err(|e| AppError::configuration(format!("Redis health check failed: {}", e)))?;

        info!("Redis connection initialized successfully");

        Ok(Self { connection })
    }

    /// Set a value with expiration
    pub async fn set_ex<T>(&self, key: &str, value: &T, expiration_seconds: u64) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)
            .map_err(|e| AppError::internal(format!("Failed to serialize value: {}", e)))?;

        let mut conn = self.connection.clone();
        conn.set_ex::<_, _, ()>(key, serialized, expiration_seconds).await?;

        Ok(())
    }

    /// Set a value without expiration
    pub async fn set<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)
            .map_err(|e| AppError::internal(format!("Failed to serialize value: {}", e)))?;

        let mut conn = self.connection.clone();
        conn.set::<_, _, ()>(key, serialized).await?;

        Ok(())
    }

    /// Get a value
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.connection.clone();
        let result: Option<String> = conn.get(key).await?;

        match result {
            Some(serialized) => {
                let value = serde_json::from_str(&serialized)
                    .map_err(|e| AppError::internal(format!("Failed to deserialize value: {}", e)))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Delete a key
    pub async fn del(&self, key: &str) -> Result<bool> {
        let mut conn = self.connection.clone();
        let result: i32 = conn.del(key).await?;
        Ok(result > 0)
    }

    /// Check if key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.connection.clone();
        let result: bool = conn.exists(key).await?;
        Ok(result)
    }

    /// Set expiration for a key
    pub async fn expire(&self, key: &str, seconds: u64) -> Result<bool> {
        let mut conn = self.connection.clone();
        let result: bool = conn.expire(key, seconds as i64).await?;
        Ok(result)
    }

    /// Get TTL for a key
    pub async fn ttl(&self, key: &str) -> Result<i64> {
        let mut conn = self.connection.clone();
        let result: i64 = conn.ttl(key).await?;
        Ok(result)
    }

    /// Increment a counter
    pub async fn incr(&self, key: &str) -> Result<i64> {
        let mut conn = self.connection.clone();
        let result: i64 = conn.incr(key, 1).await?;
        Ok(result)
    }

    /// Increment a counter with expiration
    pub async fn incr_ex(&self, key: &str, expiration_seconds: u64) -> Result<i64> {
        let mut conn = self.connection.clone();
        let result: i64 = conn.incr(key, 1).await?;
        conn.expire::<_, ()>(key, expiration_seconds as i64).await?;
        Ok(result)
    }

    /// Set user registration cache
    pub async fn set_user_registration(
        &self,
        user_id: &str,
        source: &str,
        email: &str,
        db_user_id: i32,
        expiration_seconds: u64,
    ) -> Result<()> {
        let cache_key = format!("user:{}:{}", source, user_id);
        let user_data = UserCacheData {
            email: email.to_string(),
            id: db_user_id,
        };

        self.set_ex(&cache_key, &user_data, expiration_seconds).await
    }

    /// Get user registration from cache
    pub async fn get_user_registration(
        &self,
        user_id: &str,
        source: &str,
    ) -> Result<Option<UserCacheData>> {
        let cache_key = format!("user:{}:{}", source, user_id);
        self.get(&cache_key).await
    }

    /// Invalidate user cache
    pub async fn invalidate_user_cache(&self, user_id: &str, source: &str) -> Result<bool> {
        let cache_key = format!("user:{}:{}", source, user_id);
        self.del(&cache_key).await
    }

    /// Set user state for registration flow
    pub async fn set_user_state<T>(&self, user_id: &str, state: &T) -> Result<()>
    where
        T: Serialize,
    {
        let cache_key = format!("user_state:{}", user_id);
        self.set_ex(&cache_key, state, 1800).await // 30 minutes
    }

    /// Get user state
    pub async fn get_user_state<T>(&self, user_id: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let cache_key = format!("user_state:{}", user_id);
        self.get(&cache_key).await
    }

    /// Remove user state
    pub async fn remove_user_state(&self, user_id: &str) -> Result<bool> {
        let cache_key = format!("user_state:{}", user_id);
        self.del(&cache_key).await
    }

    /// Rate limiting
    pub async fn check_rate_limit(
        &self,
        identifier: &str,
        max_requests: u32,
        window_seconds: u64,
    ) -> Result<RateLimitResult> {
        let cache_key = format!("rate_limit:{}", identifier);
        let current_count = self.incr_ex(&cache_key, window_seconds).await?;

        Ok(RateLimitResult {
            allowed: current_count <= max_requests as i64,
            current_count: current_count as u32,
            max_requests,
            reset_time_seconds: window_seconds,
        })
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.connection.clone();
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }

    /// Get connection for custom operations
    pub fn connection(&self) -> MultiplexedConnection {
        self.connection.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCacheData {
    pub email: String,
    pub id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub current_count: u32,
    pub max_requests: u32,
    pub reset_time_seconds: u64,
}

/// Cache key builders
pub struct CacheKeys;

impl CacheKeys {
    pub fn user_registration(source: &str, user_id: &str) -> String {
        format!("user:{}:{}", source, user_id)
    }

    pub fn user_state(user_id: &str) -> String {
        format!("user_state:{}", user_id)
    }

    pub fn rate_limit(identifier: &str) -> String {
        format!("rate_limit:{}", identifier)
    }

    pub fn qr_detection_result(hash: &str) -> String {
        format!("qr_result:{}", hash)
    }

    pub fn qr_detection(image_data: &str) -> String {
        // Usar los primeros 16 caracteres del hash SHA256 de la imagen
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(image_data.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        format!("qr_detection:{}", &hash[..16])
    }

    pub fn ocr_result(hash: &str) -> String {
        format!("ocr_result:{}", hash)
    }

    pub fn user_balance(user_id: i32) -> String {
        format!("user_balance:{}", user_id)
    }

    pub fn invoice_cache(cufe: &str) -> String {
        format!("invoice:{}", cufe)
    }
}

/// Cache TTL constants (in seconds)
pub struct CacheTtl;

impl CacheTtl {
    pub const USER_REGISTRATION: u64 = 1800; // 30 minutes
    pub const USER_STATE: u64 = 1800; // 30 minutes
    pub const RATE_LIMIT: u64 = 60; // 1 minute
    pub const QR_DETECTION: u64 = 3600; // 1 hour
    pub const OCR_RESULT: u64 = 3600; // 1 hour
    pub const USER_BALANCE: u64 = 300; // 5 minutes
    pub const INVOICE_DATA: u64 = 86400; // 24 hours
}
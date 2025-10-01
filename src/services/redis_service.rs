// ============================================================================
// REDIS SERVICE
// ============================================================================
// Date: September 18, 2025
// Purpose: Redis connection and operations management
// ============================================================================

use redis::{AsyncCommands, RedisError};
use deadpool_redis::{Pool, Connection};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use std::sync::Arc;
use tracing::{info, error, warn};
use uuid::Uuid;
use anyhow::Result;
use crate::state::AppState;

#[derive(Clone)]
pub struct RedisService {
    pool: Pool,
}

impl RedisService {
    pub fn new(pool: deadpool_redis::Pool) -> Self {
        Self { pool }
    }

    pub fn from_pool(pool: deadpool_redis::Pool) -> Self {
        Self::new(pool)
    }

    /// Get a Redis connection from the pool
    async fn get_connection(&self) -> Result<Connection, RedisError> {
        self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })
    }

    /// Set a key-value pair with TTL
    pub async fn set_with_ttl<T>(&self, key: &str, value: &T, ttl_seconds: u64) -> Result<(), RedisError>
    where
        T: Serialize,
    {
        let mut conn = self.get_connection().await?;
        let serialized = serde_json::to_string(value).map_err(|e| {
            error!("Failed to serialize value for key {}: {}", key, e);
            RedisError::from((redis::ErrorKind::TypeError, "Serialization error"))
        })?;

        conn.set_ex::<_, _, ()>(key, serialized, ttl_seconds).await?;
        
        info!(
            key = %key,
            ttl = ttl_seconds,
            "🔑 Redis key set with TTL"
        );
        
        Ok(())
    }

    /// Get a value by key and deserialize it
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, RedisError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.get_connection().await?;
        let value: Option<String> = conn.get(key).await?;

        match value {
            Some(serialized) => {
                let deserialized = serde_json::from_str(&serialized).map_err(|e| {
                    error!("Failed to deserialize value for key {}: {}", key, e);
                    RedisError::from((redis::ErrorKind::TypeError, "Deserialization error"))
                })?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    /// Delete a key
    pub async fn delete(&self, key: &str) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let deleted: i32 = conn.del(key).await?;
        
        info!(
            key = %key,
            deleted = deleted > 0,
            "🗑️ Redis key deletion"
        );
        
        Ok(deleted > 0)
    }

    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    /// Get TTL of a key
    pub async fn ttl(&self, key: &str) -> Result<i64, RedisError> {
        let mut conn = self.get_connection().await?;
        let ttl: i64 = conn.ttl(key).await?;
        Ok(ttl)
    }

    /// Increment a counter with TTL (for rate limiting)
    pub async fn increment_with_ttl(&self, key: &str, ttl_seconds: u64) -> Result<i64, RedisError> {
        let mut conn = self.get_connection().await?;
        
        // Use a Redis transaction to ensure atomicity
        let count: i64 = redis::pipe()
            .atomic()
            .incr(key, 1)
                        .expire(key, ttl_seconds as i64)
            .query_async(&mut conn)
            .await?;

        info!(
            key = %key,
            count = count,
            ttl = ttl_seconds,
            "📊 Redis counter incremented"
        );

        Ok(count)
    }

    /// Set multiple keys in a single operation
    pub async fn mset<T>(&self, pairs: Vec<(String, T, u64)>) -> Result<(), RedisError>
    where
        T: Serialize,
    {
        if pairs.is_empty() {
            return Ok(());
        }

        let mut conn = self.get_connection().await?;
        let mut pipe = redis::pipe();
        pipe.atomic();

        for (key, value, ttl) in pairs.iter() {
            let serialized = serde_json::to_string(value).map_err(|e| {
                error!("Failed to serialize value for key {}: {}", key, e);
                RedisError::from((redis::ErrorKind::TypeError, "Serialization error"))
            })?;
            
            pipe.set_ex(key, serialized, *ttl);
        }

        pipe.query_async::<()>(&mut conn).await?;
        
        info!(
            count = pairs.len(),
            "🔑 Redis bulk set operation completed"
        );

        Ok(())
    }

    /// Get multiple keys in a single operation
    pub async fn mget<T>(&self, keys: Vec<&str>) -> Result<Vec<Option<T>>, RedisError>
    where
        T: for<'de> Deserialize<'de>,
    {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.get_connection().await?;
        let values: Vec<Option<String>> = conn.get(&keys).await?;

        let mut results = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            match value {
                Some(serialized) => {
                    let deserialized = serde_json::from_str(&serialized).map_err(|e| {
                        error!("Failed to deserialize value for key {}: {}", keys[i], e);
                        RedisError::from((redis::ErrorKind::TypeError, "Deserialization error"))
                    })?;
                    results.push(Some(deserialized));
                }
                None => results.push(None),
            }
        }

        Ok(results)
    }

    /// Clean up expired keys (maintenance operation)
    pub async fn cleanup_expired_patterns(&self, pattern: &str) -> Result<u64, RedisError> {
        let mut conn = self.get_connection().await?;
        
        // Get all keys matching the pattern
        let keys: Vec<String> = conn.keys(pattern).await?;
        
        if keys.is_empty() {
            return Ok(0);
        }

        // Check which keys are expired (TTL <= 0)
        let mut expired_keys = Vec::new();
        for key in keys {
            let ttl: i64 = conn.ttl(&key).await?;
            if ttl == -2 { // Key doesn't exist (already expired)
                expired_keys.push(key);
            }
        }

        // Delete expired keys
        if !expired_keys.is_empty() {
            let deleted: i64 = conn.del(&expired_keys).await?;
            
            warn!(
                pattern = %pattern,
                expired_count = expired_keys.len(),
                deleted = deleted,
                "🧹 Redis cleanup operation"
            );
            
            Ok(deleted as u64)
        } else {
            Ok(0)
        }
    }

    /// Health check for Redis connection
    pub async fn health_check(&self) -> Result<String, RedisError> {
        let mut conn = self.get_connection().await?;
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(pong)
    }

    /// Get Redis info
    pub async fn info(&self) -> Result<String, RedisError> {
        let mut conn = self.get_connection().await?;
        let info: String = redis::cmd("INFO").query_async(&mut conn).await?;
        Ok(info)
    }
}

// ============================================================================
// REDIS KEY PATTERNS
// ============================================================================

pub struct RedisKeys;

impl RedisKeys {
    /// Generate key for linking tokens
    pub fn linking_token(token_id: &str) -> String {
        format!("auth:linking_token:{}", token_id)
    }

    /// Generate key for verification codes
    pub fn verification_code(email: &str, purpose: &str) -> String {
        format!("auth:verification:{}:{}", purpose, email)
    }

    /// Generate key for rate limiting
    pub fn rate_limit(identifier: &str, window: &str) -> String {
        format!("auth:rate_limit:{}:{}", window, identifier)
    }

    /// Generate key for session data
    pub fn session(session_id: &str) -> String {
        format!("auth:session:{}", session_id)
    }

    /// Generate key for Google certificate cache
    pub fn google_certs() -> String {
        "auth:google:certificates".to_string()
    }

    /// Generate key for user cache
    pub fn user_cache(user_id: i32) -> String {
        format!("auth:user:{}", user_id)
    }

    /// Generate key for provider cache
    pub fn provider_cache(user_id: i32) -> String {
        format!("auth:providers:{}", user_id)
    }
}

// ============================================================================
// REDIS UTILITIES
// ============================================================================

pub struct RedisUtils;

impl RedisUtils {
    /// Generate a unique token ID
    pub fn generate_token_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Calculate TTL in seconds from duration
    pub fn duration_to_seconds(duration: Duration) -> u64 {
        duration.as_secs()
    }

    /// Validate Redis key format
    pub fn validate_key(key: &str) -> bool {
        !key.is_empty() && 
        key.len() <= 512 && 
        !key.contains(' ') && 
        key.is_ascii()
    }

    /// Sanitize user input for Redis keys
    pub fn sanitize_key_component(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.' || *c == '@')
            .take(100) // Limit length
            .collect()
    }
}

// ============================================================================
// COMPATIBILITY FUNCTIONS (LEGACY SUPPORT)
// ============================================================================

use crate::models::user::UserState;

/// Legacy compatibility functions - these delegate to redis_compat module
pub async fn save_user_state(
    app_state: &Arc<AppState>,
    user_id: &str,
    state: &UserState,
    ttl_seconds: usize,
) -> Result<()> {
    let state_json = serde_json::to_value(state)
        .map_err(|e| anyhow::anyhow!("Failed to serialize user state: {}", e))?;
    crate::shared::redis_compat::save_user_state(app_state, user_id, state_json, ttl_seconds as u64).await
}

pub async fn get_user_state(
    app_state: &Arc<AppState>,
    user_id: &str,
) -> Result<Option<UserState>> {
    match crate::shared::redis_compat::get_user_state(app_state, user_id).await? {
        Some(json_value) => {
            let user_state = serde_json::from_value(json_value)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize user state: {}", e))?;
            Ok(Some(user_state))
        }
        None => Ok(None)
    }
}

pub async fn delete_user_state(
    app_state: &Arc<AppState>,
    user_id: &str,
) -> Result<()> {
    crate::shared::redis_compat::delete_user_state(app_state, user_id).await
}

pub async fn check_advanced_ocr_rate_limit(
    app_state: &Arc<AppState>,
    user_id: &str,
) -> Result<(bool, String)> {
    let allowed = crate::shared::redis_compat::check_advanced_ocr_rate_limit(app_state, user_id).await?;
    let message = if allowed {
        "Rate limit check passed".to_string()
    } else {
        "Rate limit exceeded".to_string()
    };
    Ok((allowed, message))
}

pub async fn get_user_ocr_limits(
    app_state: &Arc<AppState>,
    user_id: &str,
) -> Result<crate::shared::redis_compat::OcrLimits> {
    crate::shared::redis_compat::get_user_ocr_limits(app_state, user_id).await
}

pub async fn get_user_trust_score(
    app_state: &Arc<AppState>,
    user_id: &str,
) -> Result<f32> {
    crate::shared::redis_compat::get_user_trust_score(app_state, user_id).await
}

// Compatibility functions for OCR session service
pub async fn set_with_ttl<T: Serialize>(
    client: &deadpool_redis::Pool,
    key: &str,
    value: &T,
    ttl_seconds: u64,
) -> Result<()> {
    crate::shared::redis_compat::set_with_ttl(client, key, value, ttl_seconds).await
}

pub async fn get<T: serde::de::DeserializeOwned>(
    client: &deadpool_redis::Pool,
    key: &str,
) -> Result<Option<T>> {
    crate::shared::redis_compat::get(client, key).await
}

pub async fn delete(
    client: &deadpool_redis::Pool,
    key: &str,
) -> Result<()> {
    crate::shared::redis_compat::delete(client, key).await
}
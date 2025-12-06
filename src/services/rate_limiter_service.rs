// ============================================================================
// RATE LIMITING SERVICE - Protección contra abuso
// ============================================================================

use anyhow::Result;
use redis::AsyncCommands;
use crate::observability::metrics::RATE_LIMIT_EXCEEDED;
use tracing::warn;

pub struct RateLimiter {
    redis: deadpool_redis::Pool,
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_secs: u64,
}

impl RateLimiter {
    pub fn new(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }

    /// Verificar si una clave excede el límite
    pub async fn check_rate_limit(
        &self,
        key: &str,
        config: RateLimitConfig,
    ) -> Result<bool> {
        let mut conn = self.redis.get().await?;

        // Usar Redis INCR con expiración
        let count: u32 = conn.incr(key, 1).await?;

        // Si es la primera request, establecer TTL
        if count == 1 {
            conn.expire::<_, ()>(key, config.window_secs as i64).await?;
        }

        let allowed = count <= config.max_requests;

        if !allowed {
            warn!("Rate limit exceeded for key: {}", key);
            RATE_LIMIT_EXCEEDED
                .with_label_values(&[key, "requests_per_window"])
                .inc();
        }

        Ok(allowed)
    }

    /// Obtener requests restantes
    pub async fn get_remaining(&self, key: &str, config: RateLimitConfig) -> Result<u32> {
        let mut conn = self.redis.get().await?;
        let count: Option<u32> = conn.get(key).await?;
        let current = count.unwrap_or(0);
        Ok(config.max_requests.saturating_sub(current))
    }

    /// Resetear límite para una clave
    pub async fn reset(&self, key: &str) -> Result<()> {
        let mut conn = self.redis.get().await?;
        conn.del::<_, ()>(key).await?;
        Ok(())
    }
}

// ============================================================================
// RATE LIMIT CONFIGS - Configuraciones predefinidas
// ============================================================================

impl RateLimitConfig {
    /// Rate limit para requests HTTP generales (por IP)
    pub const REQUESTS_PER_MINUTE_IP: Self = Self {
        max_requests: 100,
        window_secs: 60,
    };

    /// Rate limit para redenciones (por usuario)
    pub const REDEMPTIONS_PER_DAY_USER: Self = Self {
        max_requests: 10,
        window_secs: 86400, // 24 horas
    };

    /// Rate limit para validaciones de merchant
    pub const VALIDATIONS_PER_MINUTE_MERCHANT: Self = Self {
        max_requests: 500,
        window_secs: 60,
    };

    /// Rate limit para login attempts (por IP)
    pub const LOGIN_ATTEMPTS_PER_HOUR_IP: Self = Self {
        max_requests: 10,
        window_secs: 3600,
    };

    /// Rate limit para creación de ofertas (por merchant)
    pub const OFFER_CREATION_PER_HOUR_MERCHANT: Self = Self {
        max_requests: 20,
        window_secs: 3600,
    };

    // ========================================================================
    // NOTIFICATION RATE LIMITS
    // ========================================================================

    /// Rate limit para notificaciones push por usuario (10 por hora)
    pub const NOTIFICATIONS_PER_HOUR_USER: Self = Self {
        max_requests: 10,
        window_secs: 3600,
    };

    /// Rate limit para promos por usuario (3 por día)
    pub const PROMO_NOTIFICATIONS_PER_DAY_USER: Self = Self {
        max_requests: 3,
        window_secs: 86400,
    };

    /// Cooldown entre notificaciones del mismo tipo (5 minutos)
    pub const NOTIFICATION_TYPE_COOLDOWN: Self = Self {
        max_requests: 1,
        window_secs: 300,
    };

    /// Rate limit para requests a endpoints de notificaciones (por usuario)
    pub const NOTIFICATION_API_PER_MINUTE_USER: Self = Self {
        max_requests: 60,
        window_secs: 60,
    };
}

// ============================================================================
// RATE LIMIT KEYS - Generadores de claves
// ============================================================================

pub fn rate_limit_key_ip(ip: &str) -> String {
    format!("ratelimit:ip:{}", ip)
}

pub fn rate_limit_key_user_redemptions(user_id: i32) -> String {
    format!("ratelimit:user:{}:redemptions", user_id)
}

pub fn rate_limit_key_merchant_validations(merchant_id: &str) -> String {
    format!("ratelimit:merchant:{}:validations", merchant_id)
}

pub fn rate_limit_key_login_attempts(ip: &str) -> String {
    format!("ratelimit:login:{}", ip)
}

// ============================================================================
// NOTIFICATION RATE LIMIT KEYS
// ============================================================================

/// Key para rate limit de notificaciones por usuario por hora
pub fn rate_limit_key_notifications_hourly(user_id: i64) -> String {
    format!("ratelimit:notif:user:{}:hourly", user_id)
}

/// Key para rate limit de promos por usuario por día
pub fn rate_limit_key_promo_daily(user_id: i64) -> String {
    format!("ratelimit:notif:user:{}:promo_daily", user_id)
}

/// Key para cooldown de notificaciones por tipo
pub fn rate_limit_key_notification_cooldown(user_id: i64, notification_type: &str) -> String {
    format!("ratelimit:notif:cooldown:{}:{}", user_id, notification_type)
}

/// Key para rate limit de API de notificaciones
pub fn rate_limit_key_notification_api(user_id: i64) -> String {
    format!("ratelimit:notif:api:{}", user_id)
}

// ============================================================================
// MIDDLEWARE PARA AXUM
// ============================================================================

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::sync::Arc;

pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(rate_limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = addr.ip().to_string();
    let key = rate_limit_key_ip(&ip);

    match rate_limiter
        .check_rate_limit(&key, RateLimitConfig::REQUESTS_PER_MINUTE_IP)
        .await
    {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => Err(StatusCode::TOO_MANY_REQUESTS),
        Err(e) => {
            warn!("Rate limit check error: {}", e);
            // En caso de error con Redis, permitir la request
            Ok(next.run(request).await)
        }
    }
}

// ============================================================================
// SHARED INSTANCE
// ============================================================================

use std::sync::OnceLock;

static RATE_LIMITER: OnceLock<Arc<RateLimiter>> = OnceLock::new();

pub fn init_rate_limiter(redis_pool: deadpool_redis::Pool) {
    let limiter = Arc::new(RateLimiter::new(redis_pool));
    if RATE_LIMITER.set(limiter).is_err() {
        warn!("Rate limiter already initialized");
    }
}

pub fn get_rate_limiter() -> Option<Arc<RateLimiter>> {
    RATE_LIMITER.get().cloned()
}

// ============================================================================
// NOTIFICATION RATE LIMIT HELPERS
// ============================================================================

impl RateLimiter {
    /// Check if a notification can be sent to user (hourly limit + type cooldown)
    pub async fn check_notification_rate_limit(
        &self,
        user_id: i64,
        notification_type: &str,
    ) -> Result<bool> {
        // 1. Check hourly limit
        let hourly_key = rate_limit_key_notifications_hourly(user_id);
        if !self.check_rate_limit(&hourly_key, RateLimitConfig::NOTIFICATIONS_PER_HOUR_USER).await? {
            return Ok(false);
        }

        // 2. Special limit for promos
        if notification_type == "promo" {
            let promo_key = rate_limit_key_promo_daily(user_id);
            if !self.check_rate_limit(&promo_key, RateLimitConfig::PROMO_NOTIFICATIONS_PER_DAY_USER).await? {
                return Ok(false);
            }
        }

        // 3. Check cooldown for same notification type
        let cooldown_key = rate_limit_key_notification_cooldown(user_id, notification_type);
        if !self.check_rate_limit(&cooldown_key, RateLimitConfig::NOTIFICATION_TYPE_COOLDOWN).await? {
            return Ok(false);
        }

        Ok(true)
    }

    /// Check if user can make notification API requests
    pub async fn check_notification_api_rate_limit(&self, user_id: i64) -> Result<bool> {
        let key = rate_limit_key_notification_api(user_id);
        self.check_rate_limit(&key, RateLimitConfig::NOTIFICATION_API_PER_MINUTE_USER).await
    }
}

use crate::{models::user::UserState, state::AppState};
use anyhow::Result;
use redis::{AsyncCommands, Client, RedisResult};
use anyhow::{Context, anyhow};
use std::{env, sync::Arc};
use tracing::{info, warn};

const OCR_RATE_LIMIT_COUNT: i32 = 5;

/// Crea y devuelve un cliente de Redis.
pub fn create_redis_client() -> RedisResult<Client> {
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    info!("Conectando a Redis en {}", redis_url);
    Client::open(redis_url)
}

fn get_user_state_key(user_id: &str) -> String {
    format!("user_state:{}", user_id)
}

/// Guarda el estado de un usuario en Redis con un TTL (tiempo de vida).
pub async fn save_user_state(
    app_state: &Arc<AppState>,
    user_id: &str,
    state: &UserState,
    ttl_seconds: usize,
) -> Result<()> {
    let key = get_user_state_key(user_id);
    let state_json = serde_json::to_string(state)?;
    let mut con = app_state.redis_client.get_multiplexed_async_connection().await?;
    let _: () = con.set_ex(key, state_json, ttl_seconds as u64).await?;
    Ok(())
}

pub async fn check_ocr_rate_limit(app_state: &Arc<AppState>, user_ws_id: &str) -> Result<()> {
    let mut conn = app_state.redis_client.get_multiplexed_async_connection().await.context("Failed to get Redis connection")?;
    let key = format!("ocr_rate_limit:{}", user_ws_id);

    let count: RedisResult<i32> = conn.get(&key).await;

    match count {
        Ok(c) => {
            if c >= OCR_RATE_LIMIT_COUNT {
                return Err(anyhow!("Rate limit exceeded"));
            }
        }
        Err(e) => {
            if e.kind() != redis::ErrorKind::TypeError { // TypeError means key not found
                return Err(e.into());
            }
        }
    }

    let _ : () = conn.incr(&key, 1).await?;
    Ok(())
}

/// Advanced OCR rate limiting with trust score and custom limits
pub async fn check_advanced_ocr_rate_limit(state: &Arc<AppState>, user_ws_id: &str) -> Result<(bool, String)> {
    use chrono::{Duration, Utc};
    
    // Get user limits based on trust score
    let limits = get_user_ocr_limits(state, user_ws_id).await?;
    
    let current_time = Utc::now();
    
    // Check hourly limit
    let hour_ago = current_time - Duration::hours(1);
    let hour_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM public.invoice_header WHERE user_ws = $1 AND type IN ('ocr_pending', 'ocr_validated') AND process_date >= $2",
        user_ws_id,
        hour_ago
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|count| count.unwrap_or(0))
    .unwrap_or(0);
    
    if hour_count >= limits.per_hour as i64 {
        return Ok((false, format!("⚠️ Límite excedido: {} facturas por hora", limits.per_hour)));
    }
    
    // Check daily limit
    let day_ago = current_time - Duration::days(1);
    let day_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM public.invoice_header WHERE user_ws = $1 AND type IN ('ocr_pending', 'ocr_validated') AND process_date >= $2",
        user_ws_id,
        day_ago
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|count| count.unwrap_or(0))
    .unwrap_or(0);
    
    if day_count >= limits.per_day as i64 {
        return Ok((false, format!("⚠️ Límite excedido: {} facturas por día", limits.per_day)));
    }
    
    // Check monthly limit (30 days)
    let month_ago = current_time - Duration::days(30);
    let month_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM public.invoice_header WHERE user_ws = $1 AND type IN ('ocr_pending', 'ocr_validated') AND process_date >= $2",
        user_ws_id,
        month_ago
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|count| count.unwrap_or(0))
    .unwrap_or(0);
    
    let monthly_limit = limits.per_day * 30 / 10; // Reasonable monthly limit
    if month_count >= monthly_limit as i64 {
        return Ok((false, format!("⚠️ Límite excedido: {} facturas por mes", monthly_limit)));
    }
    
    Ok((true, "OK".to_string()))
}

#[derive(Debug, Clone)]
pub struct UserOcrLimits {
    pub per_hour: i32,
    pub per_day: i32,
    pub cost_lumis: i32,
    pub priority: String,
}

/// Get user OCR limits based on trust score
pub async fn get_user_ocr_limits(state: &Arc<AppState>, user_ws_id: &str) -> Result<UserOcrLimits> {
    let (trust_score, _factors) = get_user_trust_score(state, user_ws_id).await?;
    
    let limits = if trust_score >= 40.0 {
        // Very trusted user
        UserOcrLimits {
            per_hour: 5,
            per_day: 20,
            cost_lumis: 0, // FREE during testing phase
            priority: "high".to_string(),
        }
    } else if trust_score >= 25.0 {
        // Trusted user
        UserOcrLimits {
            per_hour: 3,
            per_day: 12,
            cost_lumis: 0, // FREE during testing phase
            priority: "normal".to_string(),
        }
    } else if trust_score >= 10.0 {
        // New user with some activity
        UserOcrLimits {
            per_hour: 2,
            per_day: 8,
            cost_lumis: 0, // FREE during testing phase
            priority: "normal".to_string(),
        }
    } else {
        // New/suspicious user
        UserOcrLimits {
            per_hour: 1,
            per_day: 3,
            cost_lumis: 0, // FREE during testing phase
            priority: "low".to_string(),
        }
    };
    
    info!("🎯 User {} trust score: {:.1}, limits: {}h/{}d, cost: {} Lümis (FREE TESTING)", 
          user_ws_id, trust_score, limits.per_hour, limits.per_day, limits.cost_lumis);
    
    Ok(limits)
}

/// Calculate user trust score based on various factors
pub async fn get_user_trust_score(state: &Arc<AppState>, user_ws_id: &str) -> Result<(f64, std::collections::HashMap<String, f64>)> {
    use std::collections::HashMap;
    
    let mut trust_factors = HashMap::new();
    trust_factors.insert("account_age".to_string(), 0.0);
    trust_factors.insert("successful_qr".to_string(), 0.0);
    trust_factors.insert("ocr_success_rate".to_string(), 0.0);
    trust_factors.insert("rejection_rate".to_string(), 0.0);
    trust_factors.insert("premium_user".to_string(), 0.0);
    
    // Check if user exists (account age factor)
    let user_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM public.dim_users WHERE ws_id = $1",
        user_ws_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|count| count.unwrap_or(0))
    .unwrap_or(0);
    
    if user_exists > 0 {
        trust_factors.insert("account_age".to_string(), 5.0); // Base score for existing account
    }
    
    // Count successful QR scans (more reliable than OCR)
    let qr_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM public.invoice_header WHERE user_ws = $1 AND type IN ('QR', 'CUFE')",
        user_ws_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|count| count.unwrap_or(0))
    .unwrap_or(0);
    
    trust_factors.insert("successful_qr".to_string(), (qr_count as f64 / 10.0).min(15.0));
    
    // Calculate OCR success rate
    let ocr_stats = sqlx::query!(
        "SELECT COUNT(*) as total, SUM(CASE WHEN type = 'ocr_validated' THEN 1 ELSE 0 END) as approved FROM public.invoice_header WHERE user_ws = $1 AND type IN ('ocr_pending', 'ocr_validated')",
        user_ws_id
    )
    .fetch_one(&state.db_pool)
    .await;
    
    if let Ok(stats) = ocr_stats {
        let total_ocr = stats.total.unwrap_or(0) as f64;
        let approved_ocr = stats.approved.unwrap_or(0) as f64;
        
        if total_ocr > 0.0 {
            let success_rate = approved_ocr / total_ocr;
            trust_factors.insert("ocr_success_rate".to_string(), success_rate * 20.0);
            
            let rejection_rate = (total_ocr - approved_ocr) / total_ocr;
            trust_factors.insert("rejection_rate".to_string(), -(rejection_rate * 10.0));
        }
    }
    
    // Check premium status
    if let Ok(user_id) = sqlx::query_scalar!(
        "SELECT id FROM public.dim_users WHERE ws_id = $1",
        user_ws_id
    )
    .fetch_one(&state.db_pool)
    .await
    {
        // MIGRATED: fact_redemptions → user_redemptions
        let premium_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM rewards.user_redemptions WHERE user_id = $1 AND code_expires_at >= NOW() AND redemption_status = 'confirmed'",
            user_id as i32
        )
        .fetch_one(&state.db_pool)
        .await
        .map(|count| count.unwrap_or(0))
        .unwrap_or(0);
        
        if premium_count > 0 {
            trust_factors.insert("premium_user".to_string(), 10.0);
        }
    }
    
    let total_score: f64 = trust_factors.values().sum();
    let final_score = total_score.max(0.0).min(50.0); // Score between 0 and 50
    
    Ok((final_score, trust_factors))
}

/// Elimina el estado de un usuario de Redis.
pub async fn delete_user_state(app_state: &Arc<AppState>, user_id: &str) -> Result<()> {
    let key = get_user_state_key(user_id);
    let mut con = app_state.redis_client.get_multiplexed_async_connection().await?;
    let _: () = con.del(key).await?;
    info!("Estado eliminado para el usuario {}", user_id);
    Ok(())
}

/// Obtiene el estado de un usuario desde Redis.
pub async fn get_user_state(app_state: &Arc<AppState>, user_id: &str) -> Result<Option<UserState>> {
    let key = get_user_state_key(user_id);
    let mut con = app_state.redis_client.get_multiplexed_async_connection().await?;
    
    match con.get::<_, String>(key).await {
        Ok(state_json) => {
            match serde_json::from_str::<UserState>(&state_json) {
                Ok(state) => Ok(Some(state)),
                Err(e) => {
                    warn!("Error al deserializar el estado del usuario {}: {}. Estado JSON: {}", user_id, e, state_json);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            // Distinguir entre 'no encontrado' (Nil) y otros errores de Redis.
            if e.kind() == redis::ErrorKind::TypeError {
                Ok(None) // Key no existe o no es un string
            } else {
                Err(e.into()) // Otro error de Redis
            }
        }
    }
}

/// Guarda un valor en Redis con TTL
pub async fn set_with_ttl(
    redis_client: &redis::Client,
    key: &str,
    value: &str,
    ttl_seconds: usize,
) -> Result<()> {
    let mut con = redis_client.get_multiplexed_async_connection().await?;
    let _: () = con.set_ex(key, value, ttl_seconds as u64).await?;
    Ok(())
}

/// Obtiene un valor de Redis
pub async fn get(
    redis_client: &redis::Client,
    key: &str,
) -> Result<Option<String>> {
    let mut con = redis_client.get_multiplexed_async_connection().await?;
    let result: RedisResult<String> = con.get(key).await;
    
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            if e.kind() == redis::ErrorKind::TypeError {
                Ok(None) // Key no existe
            } else {
                Err(e.into())
            }
        }
    }
}

/// Elimina una clave de Redis
pub async fn delete(
    redis_client: &redis::Client,
    key: &str,
) -> Result<()> {
    let mut con = redis_client.get_multiplexed_async_connection().await?;
    let _: () = con.del(key).await?;
    Ok(())
}

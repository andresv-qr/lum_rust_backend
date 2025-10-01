pub mod metrics;
pub mod endpoints;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, error};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
    pub services: ServiceHealthStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
    pub database: ServiceStatus,
    pub redis: ServiceStatus,
    pub memory: MemoryStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub status: String,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryStatus {
    pub status: String,
    pub used_mb: Option<u64>,
    pub available_mb: Option<u64>,
}

static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

pub fn init_monitoring() {
    START_TIME.set(SystemTime::now()).ok();
    info!("🔍 Monitoring system initialized");
}

/// Comprehensive health check endpoint
pub async fn health_check_detailed(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    info!("🔍 Starting detailed health check");
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let uptime = START_TIME
        .get()
        .and_then(|start| SystemTime::now().duration_since(*start).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Check database health
    let db_status = check_database_health(&state.db_pool).await;
    
    // Check Redis health
    let redis_status = check_redis_health(&state.redis_client).await;
    
    // Check memory status
    let memory_status = check_memory_status();
    
    let overall_status = if db_status.status == "healthy" && redis_status.status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };
    
    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp,
        version: "4.0.0".to_string(),
        uptime_seconds: uptime,
        services: ServiceHealthStatus {
            database: db_status,
            redis: redis_status,
            memory: memory_status,
        },
    };
    
    let duration = start_time.elapsed().as_millis();
    info!("🔍 Health check completed in {}ms - Status: {}", duration, overall_status);
    
    if overall_status == "healthy" {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

async fn check_database_health(pool: &sqlx::PgPool) -> ServiceStatus {
    let start = std::time::Instant::now();
    
    match sqlx::query("SELECT 1 as health_check")
        .fetch_one(pool)
        .await
    {
        Ok(_) => {
            let response_time = start.elapsed().as_millis() as u64;
            info!("✅ Database health check passed ({}ms)", response_time);
            ServiceStatus {
                status: "healthy".to_string(),
                response_time_ms: Some(response_time),
                error: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Database connection failed: {}", e);
            error!("❌ Database health check failed: {}", error_msg);
            ServiceStatus {
                status: "unhealthy".to_string(),
                response_time_ms: None,
                error: Some(error_msg),
            }
        }
    }
}

async fn check_redis_health(redis_client: &redis::Client) -> ServiceStatus {
    let start = std::time::Instant::now();
    
    match redis_client.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            match redis::cmd("PING").query_async::<String>(&mut conn).await {
                Ok(_) => {
                    let response_time = start.elapsed().as_millis() as u64;
                    info!("✅ Redis health check passed ({}ms)", response_time);
                    ServiceStatus {
                        status: "healthy".to_string(),
                        response_time_ms: Some(response_time),
                        error: None,
                    }
                }
                Err(e) => {
                    let error_msg = format!("Redis ping failed: {}", e);
                    error!("❌ Redis health check failed: {}", error_msg);
                    ServiceStatus {
                        status: "unhealthy".to_string(),
                        response_time_ms: None,
                        error: Some(error_msg),
                    }
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Redis connection failed: {}", e);
            error!("❌ Redis health check failed: {}", error_msg);
            ServiceStatus {
                status: "unhealthy".to_string(),
                response_time_ms: None,
                error: Some(error_msg),
            }
        }
    }
}

fn check_memory_status() -> MemoryStatus {
    // Basic memory check - in production, use more sophisticated monitoring
    MemoryStatus {
        status: "healthy".to_string(),
        used_mb: None, // Would implement with system metrics
        available_mb: None,
    }
}

/// Simple health check for load balancers
pub async fn health_check_simple() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "ok",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    })))
}

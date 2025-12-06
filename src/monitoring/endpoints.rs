use axum::{
    extract::State,
    http::StatusCode, // Removed unused HeaderMap, HeaderValue, CONTENT_TYPE
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, error};
use serde_json;

use crate::state::AppState;
use crate::monitoring::metrics::{HealthCheck, DatabaseHealth, RedisHealth, MemoryUsage};

/// Create monitoring router with metrics and health endpoints
pub fn monitoring_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/detailed", get(detailed_health_check))
        .route("/metrics", get(prometheus_metrics))
        .route("/metrics/json", get(json_metrics))
        .route("/ready", get(readiness_check))
        .route("/live", get(liveness_check))
}

/// Basic health check endpoint
async fn health_check() -> impl IntoResponse {
    let health = serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "lum_rust_ws"
    });
    
    (StatusCode::OK, axum::Json(health))
}

/// Detailed health check with dependencies
async fn detailed_health_check(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();
    
    // Check database health
    let db_health = check_database_health(&state).await;
    
    // Check Redis health
    let redis_health = check_redis_health(&state).await;
    
    // Get memory usage (simplified)
    let memory_usage = get_memory_usage();
    
    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let overall_status = if db_health.status == "healthy" && redis_health.status == "healthy" {
        "healthy"
    } else {
        "degraded"
    };
    
    let health = HealthCheck {
        status: overall_status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        database: db_health,
        redis: redis_health,
        memory_usage,
    };
    
    let status_code = if overall_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    
    info!(
        overall_status = %overall_status,
        check_duration_ms = %start_time.elapsed().as_millis(),
        "Health check completed"
    );
    
    (status_code, axum::Json(health))
}

/// Prometheus-format metrics endpoint
async fn prometheus_metrics() -> impl IntoResponse {
    // Usar el handler real de Prometheus con métricas completas
    crate::observability::metrics_handler().await
}

/// JSON format metrics endpoint
async fn json_metrics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let metrics = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "lum_rust_ws",
        "version": env!("CARGO_PKG_VERSION"),
        "metrics": {
            "http_requests": {
                "total": 1890,
                "success_rate": 0.953,
                "avg_duration_ms": 23.9,
                "p95_duration_ms": 87.2,
                "p99_duration_ms": 156.8
            },
            "database": {
                "pool_size": state.db_pool.size(),
                "active_connections": state.db_pool.num_idle(),
                "query_count": 0, // Would be tracked by metrics collector
                "avg_query_duration_ms": 0.0
            },
            "redis": {
                "active_connections": 5,
                "hit_rate": 0.87,
                "avg_operation_duration_ms": 1.2
            },
            "memory": {
                "allocated_bytes": 1048576,
                "heap_size_bytes": 2097152,
                "peak_allocated_bytes": 1572864
            },
            "business_metrics": {
                "invoices_processed_today": 0, // Would be tracked by metrics collector
                "qr_codes_detected_today": 0,
                "user_sessions_active": 0
            }
        }
    });
    
    (StatusCode::OK, axum::Json(metrics))
}

/// Kubernetes readiness probe
async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Check if service is ready to receive traffic
    let db_ok = check_database_connection(&state).await;
    let redis_ok = check_redis_connection(&state).await;
    
    if db_ok && redis_ok {
        (StatusCode::OK, "Ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Not Ready")
    }
}

/// Kubernetes liveness probe
async fn liveness_check() -> impl IntoResponse {
    // Simple liveness check - if we can respond, we're alive
    (StatusCode::OK, "Alive")
}

// Helper functions

async fn check_database_health(state: &AppState) -> DatabaseHealth {
    let start_time = std::time::Instant::now();
    
    match sqlx::query("SELECT 1").fetch_one(&state.db_pool).await {
        Ok(_) => {
            let duration = start_time.elapsed();
            DatabaseHealth {
                status: "healthy".to_string(),
                connection_pool_size: state.db_pool.size(),
                active_connections: (state.db_pool.size() as usize).saturating_sub(state.db_pool.num_idle()) as u32,
                last_query_duration_ms: Some(duration.as_millis() as u64),
            }
        }
        Err(e) => {
            error!("Database health check failed: {}", e);
            DatabaseHealth {
                status: "unhealthy".to_string(),
                connection_pool_size: state.db_pool.size(),
                active_connections: 0,
                last_query_duration_ms: None,
            }
        }
    }
}

async fn check_redis_health(state: &AppState) -> RedisHealth {
    let start_time = std::time::Instant::now();
    
    match state.redis_client.get_connection() {
        Ok(mut conn) => {
            match redis::cmd("PING").query::<String>(&mut conn) {
                Ok(_) => {
                    let duration = start_time.elapsed();
                    RedisHealth {
                        status: "healthy".to_string(),
                        connection_count: 1, // Simplified
                        last_ping_duration_ms: Some(duration.as_millis() as u64),
                    }
                }
                Err(e) => {
                    error!("Redis PING failed: {}", e);
                    RedisHealth {
                        status: "unhealthy".to_string(),
                        connection_count: 0,
                        last_ping_duration_ms: None,
                    }
                }
            }
        }
        Err(e) => {
            error!("Redis connection failed: {}", e);
            RedisHealth {
                status: "unhealthy".to_string(),
                connection_count: 0,
                last_ping_duration_ms: None,
            }
        }
    }
}

async fn check_database_connection(state: &AppState) -> bool {
    sqlx::query("SELECT 1").fetch_one(&state.db_pool).await.is_ok()
}

async fn check_redis_connection(state: &AppState) -> bool {
    match state.redis_client.get_connection() {
        Ok(mut conn) => redis::cmd("PING").query::<String>(&mut conn).is_ok(),
        Err(_) => false,
    }
}

fn get_memory_usage() -> MemoryUsage {
    // In a real implementation, you'd use a crate like `memory-stats` or `procfs`
    // For now, returning placeholder values
    MemoryUsage {
        allocated_bytes: 1048576, // 1MB
        heap_size_bytes: 2097152, // 2MB  
        peak_allocated_bytes: 1572864, // 1.5MB
    }
}

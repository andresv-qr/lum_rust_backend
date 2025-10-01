use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub success_rate: f64,
    pub concurrent_requests: usize,
    pub uptime_seconds: u64,
    pub recommendations: Vec<String>,
    pub qr_detection: DomainMetrics,
    pub ocr_processing: DomainMetrics,
    pub webhook_processing: DomainMetrics,
    pub api_requests: DomainMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainMetrics {
    pub active_permits: usize,
    pub max_permits: usize,
    pub total_requests: u64,
    pub average_latency_ms: f64,
    pub utilization_percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub qr_cache: CacheStats,
    pub ocr_cache: CacheStats,
    pub user_session_cache: CacheStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub stats: CacheMetrics,
    pub l1_capacity: usize,
    pub l1_size: usize,
    pub l2_connected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub l1_size: usize,
    pub l2_connected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResetResponse {
    pub message: String,
    pub timestamp: String,
}

/// Get comprehensive performance metrics
pub async fn get_performance_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PerformanceMetrics>, StatusCode> {
    info!("📊 Performance metrics requested");
    
    match state.performance_manager.get_metrics().await {
        Ok(metrics) => {
            let performance_metrics = PerformanceMetrics {
                total_requests: metrics.total_requests,
                successful_requests: metrics.successful_requests,
                failed_requests: metrics.failed_requests,
                average_latency_ms: metrics.average_latency_ms,
                success_rate: if metrics.total_requests > 0 {
                    (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
                } else {
                    0.0
                },
                concurrent_requests: metrics.concurrent_requests,
                uptime_seconds: metrics.uptime_seconds,
                recommendations: generate_health_recommendations(&metrics),
                qr_detection: DomainMetrics {
                    active_permits: metrics.qr_detection.active_permits,
                    max_permits: metrics.qr_detection.max_permits,
                    total_requests: metrics.qr_detection.total_requests,
                    average_latency_ms: metrics.qr_detection.average_latency_ms,
                    utilization_percentage: if metrics.qr_detection.max_permits > 0 {
                        (metrics.qr_detection.active_permits as f64 / metrics.qr_detection.max_permits as f64) * 100.0
                    } else {
                        0.0
                    },
                },
                ocr_processing: DomainMetrics {
                    active_permits: metrics.ocr_processing.active_permits,
                    max_permits: metrics.ocr_processing.max_permits,
                    total_requests: metrics.ocr_processing.total_requests,
                    average_latency_ms: metrics.ocr_processing.average_latency_ms,
                    utilization_percentage: if metrics.ocr_processing.max_permits > 0 {
                        (metrics.ocr_processing.active_permits as f64 / metrics.ocr_processing.max_permits as f64) * 100.0
                    } else {
                        0.0
                    },
                },
                webhook_processing: DomainMetrics {
                    active_permits: metrics.webhook_processing.active_permits,
                    max_permits: metrics.webhook_processing.max_permits,
                    total_requests: metrics.webhook_processing.total_requests,
                    average_latency_ms: metrics.webhook_processing.average_latency_ms,
                    utilization_percentage: if metrics.webhook_processing.max_permits > 0 {
                        (metrics.webhook_processing.active_permits as f64 / metrics.webhook_processing.max_permits as f64) * 100.0
                    } else {
                        0.0
                    },
                },
                api_requests: DomainMetrics {
                    active_permits: metrics.api_requests.active_permits,
                    max_permits: metrics.api_requests.max_permits,
                    total_requests: metrics.api_requests.total_requests,
                    average_latency_ms: metrics.api_requests.average_latency_ms,
                    utilization_percentage: if metrics.api_requests.max_permits > 0 {
                        (metrics.api_requests.active_permits as f64 / metrics.api_requests.max_permits as f64) * 100.0
                    } else {
                        0.0
                    },
                },
            };
            
            Ok(Json(performance_metrics))
        }
        Err(e) => {
            warn!("❌ Failed to get performance metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get cache statistics for all cache managers
pub async fn get_cache_statistics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CacheStatistics>, StatusCode> {
    info!("📈 Cache statistics requested");
    
    match state.performance_manager.get_cache_stats().await {
        Ok(cache_stats) => {
            let statistics = CacheStatistics {
                qr_cache: CacheStats {
                    stats: CacheMetrics {
                        hits: cache_stats.qr_cache.hits,
                        misses: cache_stats.qr_cache.misses,
                        hit_rate: cache_stats.qr_cache.hit_rate,
                        l1_size: cache_stats.qr_cache.l1_size,
                        l2_connected: cache_stats.qr_cache.l2_connected,
                    },
                    l1_capacity: 1000, // Default capacity
                    l1_size: cache_stats.qr_cache.l1_size,
                    l2_connected: cache_stats.qr_cache.l2_connected,
                },
                ocr_cache: CacheStats {
                    stats: CacheMetrics {
                        hits: cache_stats.ocr_cache.hits,
                        misses: cache_stats.ocr_cache.misses,
                        hit_rate: cache_stats.ocr_cache.hit_rate,
                        l1_size: cache_stats.ocr_cache.l1_size,
                        l2_connected: cache_stats.ocr_cache.l2_connected,
                    },
                    l1_capacity: 500, // Default capacity
                    l1_size: cache_stats.ocr_cache.l1_size,
                    l2_connected: cache_stats.ocr_cache.l2_connected,
                },
                user_session_cache: CacheStats {
                    stats: CacheMetrics {
                        hits: cache_stats.user_session_cache.hits,
                        misses: cache_stats.user_session_cache.misses,
                        hit_rate: cache_stats.user_session_cache.hit_rate,
                        l1_size: cache_stats.user_session_cache.l1_size,
                        l2_connected: cache_stats.user_session_cache.l2_connected,
                    },
                    l1_capacity: 2000, // Default capacity
                    l1_size: cache_stats.user_session_cache.l1_size,
                    l2_connected: cache_stats.user_session_cache.l2_connected,
                },
            };
            
            Ok(Json(statistics))
        }
        Err(e) => {
            warn!("❌ Failed to get cache statistics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Reset performance metrics (admin endpoint)
pub async fn reset_performance_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ResetResponse>, StatusCode> {
    info!("🔄 Performance metrics reset requested");
    
    match state.performance_manager.reset_metrics().await {
        Ok(_) => {
            let response = ResetResponse {
                message: "Performance metrics have been reset successfully".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            
            info!("✅ Performance metrics reset completed");
            Ok(Json(response))
        }
        Err(e) => {
            warn!("❌ Failed to reset performance metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Generate health recommendations based on current metrics
fn generate_health_recommendations(metrics: &crate::shared::performance::PerformanceMetrics) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    // Check average latency
    if metrics.average_latency_ms > 100.0 {
        recommendations.push("⚠️ High average latency detected. Consider optimizing slow operations.".to_string());
    }
    
    // Check success rate
    let success_rate = if metrics.total_requests > 0 {
        (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
    } else {
        100.0
    };
    
    if success_rate < 95.0 {
        recommendations.push("⚠️ Low success rate detected. Check error logs for issues.".to_string());
    }
    
    // Check QR detection utilization
    let qr_utilization = if metrics.qr_detection.max_permits > 0 {
        (metrics.qr_detection.active_permits as f64 / metrics.qr_detection.max_permits as f64) * 100.0
    } else {
        0.0
    };
    
    if qr_utilization > 80.0 {
        recommendations.push("⚠️ QR detection semaphore highly utilized. Consider increasing limits.".to_string());
    }
    
    // Check OCR processing utilization
    let ocr_utilization = if metrics.ocr_processing.max_permits > 0 {
        (metrics.ocr_processing.active_permits as f64 / metrics.ocr_processing.max_permits as f64) * 100.0
    } else {
        0.0
    };
    
    if ocr_utilization > 80.0 {
        recommendations.push("⚠️ OCR processing semaphore highly utilized. Consider increasing limits.".to_string());
    }
    
    // Check webhook processing utilization
    let webhook_utilization = if metrics.webhook_processing.max_permits > 0 {
        (metrics.webhook_processing.active_permits as f64 / metrics.webhook_processing.max_permits as f64) * 100.0
    } else {
        0.0
    };
    
    if webhook_utilization > 80.0 {
        recommendations.push("⚠️ Webhook processing semaphore highly utilized. Consider increasing limits.".to_string());
    }
    
    if recommendations.is_empty() {
        recommendations.push("✅ System is performing well. No immediate recommendations.".to_string());
    }
    
    recommendations
}

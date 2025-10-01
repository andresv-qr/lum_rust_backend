use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;
use chrono::{DateTime, Utc};

use crate::state::AppState;
use crate::webhook::{DeduplicationStats};

/// Comprehensive webhook statistics for monitoring and observability
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookStats {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub message_deduplication: DeduplicationStats,
    pub performance: WebhookPerformanceStats,
    pub system_info: SystemInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPerformanceStats {
    pub active_permits: usize,
    pub max_permits: usize,
    pub total_requests: u64,
    pub average_latency_ms: f64,
    pub utilization_percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub service_name: String,
    pub version: String,
    pub rust_version: String,
    pub uptime_info: String,
}

/// GET /webhook-stats - Comprehensive webhook statistics endpoint
/// More detailed than Python's version with performance metrics integration
pub async fn get_webhook_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<WebhookStats>, StatusCode> {
    match collect_webhook_stats(&state).await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            error!("❌ Error collecting webhook stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Collect comprehensive webhook statistics
async fn collect_webhook_stats(state: &AppState) -> Result<WebhookStats, Box<dyn std::error::Error + Send + Sync>> {
    // Get message deduplication stats
    let deduplication_stats = state.message_deduplicator.get_stats();
    
    // Get performance metrics from our performance manager
    let performance_metrics = state.performance_manager.get_metrics().await?;
    let webhook_perf = WebhookPerformanceStats {
        active_permits: performance_metrics.webhook_processing.active_permits,
        max_permits: performance_metrics.webhook_processing.max_permits,
        total_requests: performance_metrics.webhook_processing.total_requests,
        average_latency_ms: performance_metrics.webhook_processing.average_latency_ms,
        utilization_percentage: if performance_metrics.webhook_processing.max_permits > 0 {
            (performance_metrics.webhook_processing.active_permits as f64 / 
             performance_metrics.webhook_processing.max_permits as f64) * 100.0
        } else {
            0.0
        },
    };
    
    // System information
    let system_info = SystemInfo {
        service_name: "lum-whatsapp-bot-v4".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: "1.75+".to_string(),
        uptime_info: "Available via /health endpoint".to_string(),
    };
    
    Ok(WebhookStats {
        status: "ok".to_string(),
        timestamp: Utc::now(),
        message_deduplication: deduplication_stats,
        performance: webhook_perf,
        system_info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_webhook_stats_serialization() {
        let stats = WebhookStats {
            status: "ok".to_string(),
            timestamp: Utc::now(),
            message_deduplication: DeduplicationStats {
                total_entries: 100,
                valid_entries: 95,
                expired_entries: 5,
                ttl_seconds: 300,
                max_entries: 10000,
                memory_usage_estimate_kb: 64,
            },
            performance: WebhookPerformanceStats {
                active_permits: 5,
                max_permits: 100,
                total_requests: 1000,
                average_latency_ms: 25.5,
                utilization_percentage: 5.0,
            },
            system_info: SystemInfo {
                service_name: "test".to_string(),
                version: "0.1.0".to_string(),
                rust_version: "1.75".to_string(),
                uptime_info: "test".to_string(),
            },
        };
        
        // Should serialize without errors
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("message_deduplication"));
        assert!(json.contains("performance"));
    }
}

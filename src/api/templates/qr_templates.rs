use serde::{Deserialize, Serialize};

/// Query templates for qr domain
pub struct QrQueryTemplates;

impl QrQueryTemplates {
    /// qr_health_check - health check operation (no DB query needed)
    pub fn qr_health_check_query() -> &'static str {
        "SELECT 'ok' as status, 'qr_detection' as service, 'enabled' as hybrid_pipeline, true as python_fallback, NOW()::text as timestamp"
    }
    
    pub fn qr_health_check_cache_key_prefix() -> &'static str {
        "qr_qr_health_check_$1"
    }
    
    pub fn qr_health_check_cache_ttl() -> u64 {
        60
    }
}

/// Response model for qr_health_check
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct QrResponse {
    pub status: String,
    pub service: String,
    pub hybrid_pipeline: String,
    pub python_fallback: bool,
    pub timestamp: String,
}

/// Request model for qr_health_check (for POST endpoints)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QrRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filters: Option<serde_json::Value>,
}

/// Cache invalidation patterns for qr
pub struct QrCachePatterns;

impl QrCachePatterns {
    pub fn invalidate_patterns(id: i64) -> Vec<String> {
        vec![
            format!("qr_*_{}", id),
            format!("qr_list_*"),
            format!("qr_stats_*"),
        ]
    }
}

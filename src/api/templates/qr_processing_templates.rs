use serde::{Deserialize, Serialize};

/// Query templates for QR processing status and monitoring
pub struct QrProcessingQueryTemplates;

impl QrProcessingQueryTemplates {
    /// get_qr_processing_status - get detailed QR processing status
    pub fn get_qr_processing_status_query() -> &'static str {
        "SELECT 
            COUNT(*) as total_processed,
            COUNT(CASE WHEN cufe IS NOT NULL THEN 1 END) as successful_extractions,
            COUNT(CASE WHEN created_at >= NOW() - INTERVAL '24 hours' THEN 1 END) as processed_today,
            AVG(EXTRACT(EPOCH FROM (updated_at - created_at))) as avg_processing_time_seconds
         FROM invoice_header 
         WHERE user_id = $1"
    }
    
    /// get_mef_pending_status - get pending processing status
    pub fn get_mef_pending_status_query() -> &'static str {
        "SELECT 
            COUNT(*) as total_pending,
            COUNT(CASE WHEN type_document = 'QR_INVOICE' THEN 1 END) as qr_pending,
            COUNT(CASE WHEN reception_date >= NOW() - INTERVAL '24 hours' THEN 1 END) as pending_today,
            MIN(reception_date) as oldest_pending
         FROM mef_pending 
         WHERE user_id = $1"
    }
    
    /// get_recent_qr_activity - get recent QR processing activity
    pub fn get_recent_qr_activity_query() -> &'static str {
        "SELECT 
            'processed' as status,
            cufe as identifier,
            issuer_name,
            tot_amount,
            created_at as timestamp,
            'success' as result_type
         FROM invoice_header 
         WHERE user_id = $1 AND created_at >= NOW() - INTERVAL '7 days'
         UNION ALL
         SELECT 
            'pending' as status,
            COALESCE(url, 'N/A') as identifier,
            'Pending Processing' as issuer_name,
            NULL as tot_amount,
            reception_date as timestamp,
            'pending' as result_type
         FROM mef_pending 
         WHERE user_id = $1 AND reception_date >= NOW() - INTERVAL '7 days'
         ORDER BY timestamp DESC 
         LIMIT 20"
    }
    
    /// Cache key prefix for qr_processing operations
    pub fn get_cache_key_prefix() -> &'static str {
        "qr_processing"
    }
}

/// Response model for QR processing status
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct QrProcessingStatusResponse {
    pub total_processed: i64,
    pub successful_extractions: i64,
    pub processed_today: i64,
    pub avg_processing_time_seconds: Option<f64>,
    pub success_rate: f64,
    pub processing_health: String,
}

/// Response model for MEF pending status
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct MefPendingStatusResponse {
    pub total_pending: i64,
    pub qr_pending: i64,
    pub pending_today: i64,
    pub oldest_pending: Option<chrono::DateTime<chrono::Utc>>,
    pub pending_health: String,
}

/// Response model for recent QR activity
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct QrActivityResponse {
    pub status: String,
    pub identifier: String,
    pub issuer_name: Option<String>,
    pub tot_amount: Option<f64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub result_type: String,
}

/// Combined QR processing overview response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QrProcessingOverviewResponse {
    pub processing_status: QrProcessingStatusResponse,
    pub pending_status: MefPendingStatusResponse,
    pub recent_activity: Vec<QrActivityResponse>,
    pub web_scraping_health: String,
    pub python_fallback_available: bool,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Cache invalidation patterns for qr_processing
pub struct QrProcessingCachePatterns;

impl QrProcessingCachePatterns {
    pub fn invalidate_patterns(user_id: i64) -> Vec<String> {
        vec![
            format!("qr_processing_*_{}", user_id),
            format!("qr_processing_status_{}", user_id),
            "qr_processing_*".to_string(),
        ]
    }
}

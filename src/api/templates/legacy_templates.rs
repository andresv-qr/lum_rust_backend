use serde::{Deserialize, Serialize};

/// Template for user-related queries (migrated from old templates.rs)
pub struct UserQueryTemplates;

impl UserQueryTemplates {
    /// Get user by ID with caching
    pub fn get_user_by_id_query() -> &'static str {
        "SELECT user_id, whatsapp_id, email, created_at, is_verified, subscription_status FROM users WHERE user_id = $1"
    }
    
    pub fn get_user_by_id_cache_key_prefix() -> &'static str {
        "user"
    }
    
    pub fn get_user_by_id_cache_ttl() -> u64 {
        300 // 5 minutes
    }

    /// Get users with filters
    pub fn get_users_filtered_query() -> &'static str {
        "SELECT user_id, whatsapp_id, email, created_at, is_verified, subscription_status FROM users WHERE ($1::text IS NULL OR email ILIKE '%' || $1 || '%') ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    }
    
    pub fn get_users_filtered_cache_key_prefix() -> &'static str {
        "users_filtered"
    }
    
    pub fn get_users_filtered_cache_ttl() -> u64 {
        60 // 1 minute (shorter for lists)
    }

    /// Get user balance
    pub fn get_user_balance_query() -> &'static str {
        "SELECT balance FROM user_balances WHERE user_id = $1"
    }
    
    pub fn get_user_balance_cache_key_prefix() -> &'static str {
        "user_balance"
    }
    
    pub fn get_user_balance_cache_ttl() -> u64 {
        30 // 30 seconds (financial data)
    }
}

/// Template for invoice-related queries
pub struct InvoiceQueryTemplates;

impl InvoiceQueryTemplates {
    /// Get invoice by ID
    pub fn get_invoice_by_id_query() -> &'static str {
        "SELECT invoice_id, user_id, file_path, ocr_text, processed_at, status FROM invoices WHERE invoice_id = $1"
    }
    
    pub fn get_invoice_by_id_cache_key_prefix() -> &'static str {
        "invoice"
    }
    
    pub fn get_invoice_by_id_cache_ttl() -> u64 {
        600 // 10 minutes
    }

    /// Get user invoices
    pub fn get_user_invoices_query() -> &'static str {
        "SELECT invoice_id, user_id, file_path, ocr_text, processed_at, status FROM invoices WHERE user_id = $1 ORDER BY processed_at DESC LIMIT $2 OFFSET $3"
    }
    
    pub fn get_user_invoices_cache_key_prefix() -> &'static str {
        "user_invoices"
    }
    
    pub fn get_user_invoices_cache_ttl() -> u64 {
        120 // 2 minutes
    }

    /// Get invoice statistics
    pub fn get_invoice_stats_query() -> &'static str {
        "SELECT COUNT(*) as total, COUNT(CASE WHEN status = 'processed' THEN 1 END) as processed, COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed FROM invoices WHERE user_id = $1"
    }
    
    pub fn get_invoice_stats_cache_key_prefix() -> &'static str {
        "invoice_stats"
    }
    
    pub fn get_invoice_stats_cache_ttl() -> u64 {
        300 // 5 minutes
    }
}

/// Template for QR detection queries
pub struct QrQueryTemplates;

impl QrQueryTemplates {
    /// Get QR detection history
    pub fn get_qr_history_query() -> &'static str {
        "SELECT qr_id, user_id, image_hash, qr_content, detection_method, processed_at FROM qr_detections WHERE user_id = $1 ORDER BY processed_at DESC LIMIT $2 OFFSET $3"
    }
    
    pub fn get_qr_history_cache_key_prefix() -> &'static str {
        "qr_history"
    }
    
    pub fn get_qr_history_cache_ttl() -> u64 {
        180 // 3 minutes
    }

    /// Get QR detection statistics
    pub fn get_qr_stats_query() -> &'static str {
        "SELECT COUNT(*) as total, COUNT(CASE WHEN detection_method = 'rqrr' THEN 1 END) as rqrr_count, COUNT(CASE WHEN detection_method = 'quircs' THEN 1 END) as quircs_count FROM qr_detections WHERE user_id = $1"
    }
    
    pub fn get_qr_stats_cache_key_prefix() -> &'static str {
        "qr_stats"
    }
    
    pub fn get_qr_stats_cache_ttl() -> u64 {
        300 // 5 minutes
    }
}

/// Template for analytics and reporting queries
pub struct AnalyticsQueryTemplates;

impl AnalyticsQueryTemplates {
    /// Get daily usage statistics
    pub fn get_daily_usage_query() -> &'static str {
        "SELECT DATE(created_at) as date, COUNT(*) as requests FROM api_logs WHERE user_id = $1 AND created_at >= $2 GROUP BY DATE(created_at) ORDER BY date DESC"
    }
    
    pub fn get_daily_usage_cache_key_prefix() -> &'static str {
        "daily_usage"
    }
    
    pub fn get_daily_usage_cache_ttl() -> u64 {
        1800 // 30 minutes
    }

    /// Get system performance metrics
    pub fn get_system_metrics_query() -> &'static str {
        "SELECT endpoint, AVG(response_time_ms) as avg_response_time, COUNT(*) as request_count FROM api_logs WHERE created_at >= NOW() - INTERVAL '1 hour' GROUP BY endpoint"
    }
    
    pub fn get_system_metrics_cache_key_prefix() -> &'static str {
        "system_metrics"
    }
    
    pub fn get_system_metrics_cache_ttl() -> u64 {
        300 // 5 minutes
    }
}

/// Response models for common queries (migrated from old templates.rs)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserResponse {
    pub user_id: i64,
    pub whatsapp_id: String,
    pub email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_verified: bool,
    pub subscription_status: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserBalanceResponse {
    pub balance: i32,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct InvoiceResponse {
    pub invoice_id: i64,
    pub user_id: i64,
    pub file_path: String,
    pub ocr_text: Option<String>,
    pub processed_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct InvoiceStatsResponse {
    pub total: i64,
    pub processed: Option<i64>,
    pub failed: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct QrHistoryResponse {
    pub qr_id: i64,
    pub user_id: i64,
    pub image_hash: String,
    pub qr_content: String,
    pub detection_method: String,
    pub processed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct QrStatsResponse {
    pub total: i64,
    pub rqrr_count: Option<i64>,
    pub quircs_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct DailyUsageResponse {
    pub date: chrono::NaiveDate,
    pub requests: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct SystemMetricsResponse {
    pub endpoint: String,
    pub avg_response_time: Option<f64>,
    pub request_count: i64,
}

/// Cache invalidation patterns for different domains
pub struct CacheInvalidationPatterns;

impl CacheInvalidationPatterns {
    pub fn user_patterns(user_id: i64) -> Vec<String> {
        vec![
            format!("user_{}", user_id),
            format!("users_filtered_*"),
            format!("user_balance_{}", user_id),
        ]
    }
    
    pub fn invoice_patterns(user_id: i64) -> Vec<String> {
        vec![
            format!("invoice_*"),
            format!("user_invoices_{}", user_id),
            format!("invoice_stats_{}", user_id),
        ]
    }
    
    pub fn qr_patterns(user_id: i64) -> Vec<String> {
        vec![
            format!("qr_history_{}", user_id),
            format!("qr_stats_{}", user_id),
        ]
    }
}

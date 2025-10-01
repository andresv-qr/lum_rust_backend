use serde::{Deserialize, Serialize};

/// Query templates for email_check domain
pub struct EmailCheckQueryTemplates;

impl EmailCheckQueryTemplates {
    /// check_email_availability - email existence check operation
    pub fn check_email_availability_query() -> &'static str {
        "SELECT id FROM public.dim_users WHERE email = $1"
    }
    
    /// Cache key prefix for email_check operations
    pub fn get_cache_key_prefix() -> &'static str {
        "email_check"
    }
    
    /// Cache key prefix for check_email_availability operation
    pub fn get_check_email_availability_cache_key_prefix() -> &'static str {
        "email_check_availability"
    }
    
    pub fn check_email_availability_cache_key_prefix() -> &'static str {
        "email_check_check_email_availability_write"
    }
    
    pub fn check_email_availability_cache_ttl() -> u64 {
        60
    }
}

/// Response model for check_email_availability
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct EmailCheckResponse {
    pub exists: bool,
    pub message: String,
}

/// Request model for check_email_availability (for POST endpoints)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailCheckRequest {
    pub email: String,
}

/// Cache invalidation patterns for email_check
pub struct EmailCheckCachePatterns;

impl EmailCheckCachePatterns {
    pub fn invalidate_patterns(id: i64) -> Vec<String> {
        vec![
            format!("email_check_*_{}", id),
            format!("email_check_list_*"),
            format!("email_check_stats_*"),
        ]
    }
}

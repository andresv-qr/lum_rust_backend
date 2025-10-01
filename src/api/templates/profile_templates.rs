use serde::{Deserialize, Serialize};

/// Query templates for profile domain
pub struct ProfileQueryTemplates;

impl ProfileQueryTemplates {
    /// get_user_profile - single operation with caching
    pub fn get_user_profile_query() -> &'static str {
        "SELECT user_id, whatsapp_id, email, created_at, is_verified, subscription_status FROM users WHERE user_id = $1"
    }
    
    pub fn get_user_profile_cache_key_prefix() -> &'static str {
        "profile_get_user_profile_$1"
    }
    
    pub fn get_user_profile_cache_ttl() -> u64 {
        300
    }
}

/// Response model for get_user_profile
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct ProfileResponse {
    pub user_id: i64,
    pub whatsapp_id: String,
    pub email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_verified: bool,
    pub subscription_status: String,
}

/// Request model for get_user_profile (for POST endpoints)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filters: Option<serde_json::Value>,
}

/// Cache invalidation patterns for profile
pub struct ProfileCachePatterns;

impl ProfileCachePatterns {
    pub fn invalidate_patterns(id: i64) -> Vec<String> {
        vec![
            format!("profile_*_{}", id),
            format!("profile_list_*"),
            format!("profile_stats_*"),
        ]
    }
}

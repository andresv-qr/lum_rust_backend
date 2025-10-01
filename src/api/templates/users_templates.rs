use serde::{Deserialize, Serialize};

/// Query templates for users domain
pub struct UsersQueryTemplates;

impl UsersQueryTemplates {
    /// get_user_profile - single operation with caching
    pub fn get_user_profile_query() -> &'static str {
        "SELECT * FROM users WHERE id = $1"
    }
    
    pub fn get_user_profile_cache_key_prefix() -> &'static str {
        "users_get_user_profile_$1"
    }
    
    pub fn get_user_profile_cache_ttl() -> u64 {
        600
    }
}

/// Response model for get_user_profile
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UsersResponse {
    pub id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    // TODO: Add specific fields for your users table
}

/// Request model for get_user_profile (for POST endpoints)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsersRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filters: Option<serde_json::Value>,
}

/// Cache invalidation patterns for users
pub struct UsersCachePatterns;

impl UsersCachePatterns {
    pub fn invalidate_patterns(id: i64) -> Vec<String> {
        vec![
            format!("users_*_{}", id),
            format!("users_list_*"),
            format!("users_stats_*"),
        ]
    }
}

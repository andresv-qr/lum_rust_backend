use serde::{Deserialize, Serialize};

/// Query templates for lumis_balance domain
pub struct LumisBalanceQueryTemplates;

impl LumisBalanceQueryTemplates {
    /// get_user_lumis_balance - get user's current Lumis balance
    pub fn get_user_lumis_balance_query() -> &'static str {
        "SELECT lumis_balance FROM dim_users WHERE ws_id = $1"
    }
    
    /// Cache key prefix for lumis_balance operations
    pub fn get_cache_key_prefix() -> &'static str {
        "lumis_balance"
    }
    
    /// Cache key prefix for get_user_lumis_balance operation
    pub fn get_user_lumis_balance_cache_key_prefix() -> &'static str {
        "lumis_balance_user"
    }
}

/// Response model for get_user_lumis_balance
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct LumisBalanceResponse {
    pub lumis_balance: i32,
    pub formatted_balance: String,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request model for get_user_lumis_balance (for POST endpoints)
#[derive(Debug, Deserialize)]
pub struct LumisBalanceRequest {
    pub user_id: Option<i64>,
    pub whatsapp_id: Option<String>,
}

/// Cache invalidation patterns for lumis_balance
pub struct LumisBalanceCachePatterns;

impl LumisBalanceCachePatterns {
    pub fn invalidate_patterns(user_id: i64) -> Vec<String> {
        vec![
            format!("lumis_balance_*_{}", user_id),
            format!("lumis_balance_user_{}", user_id),
            "lumis_balance_*".to_string(),
        ]
    }
}

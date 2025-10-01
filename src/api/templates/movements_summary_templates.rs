use serde::{Deserialize, Serialize};

/// Query templates for movements_summary domain
pub struct MovementsSummaryQueryTemplates;

impl MovementsSummaryQueryTemplates {
    /// get_user_movements_summary - get user's transaction/activity summary
    pub fn get_user_movements_summary_query() -> &'static str {
        "SELECT 
            COUNT(*) as total_transactions,
            COALESCE(SUM(CASE WHEN amount > 0 THEN amount ELSE 0 END), 0) as total_earned,
            COALESCE(SUM(CASE WHEN amount < 0 THEN ABS(amount) ELSE 0 END), 0) as total_spent,
            COUNT(CASE WHEN created_at >= NOW() - INTERVAL '30 days' THEN 1 END) as recent_activity,
            MAX(created_at) as last_activity
         FROM user_transactions 
         WHERE user_id = $1"
    }
    
    /// get_recent_movements - get user's recent transactions
    pub fn get_recent_movements_query() -> &'static str {
        "SELECT 
            transaction_type,
            amount,
            description,
            created_at,
            status
         FROM user_transactions 
         WHERE user_id = $1 
         ORDER BY created_at DESC 
         LIMIT 10"
    }
    
    /// Cache key prefix for movements_summary operations
    pub fn get_cache_key_prefix() -> &'static str {
        "movements_summary"
    }
    
    /// Cache key prefix for get_user_movements_summary operation
    pub fn get_user_movements_summary_cache_key_prefix() -> &'static str {
        "movements_summary_user"
    }
}

/// Response model for get_user_movements_summary
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct MovementsSummaryResponse {
    pub total_transactions: i64,
    pub total_earned: i32,
    pub total_spent: i32,
    pub recent_activity: i64,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub net_balance: i32,
    pub activity_level: String,
}

/// Response model for recent movements
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct RecentMovementResponse {
    pub transaction_type: String,
    pub amount: i32,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
}

/// Request model for movements summary (for POST endpoints)
#[derive(Debug, Deserialize)]
pub struct MovementsSummaryRequest {
    pub user_id: Option<i64>,
    pub days_back: Option<i32>,
    pub include_recent: Option<bool>,
}

/// Cache invalidation patterns for movements_summary
pub struct MovementsSummaryCachePatterns;

impl MovementsSummaryCachePatterns {
    pub fn invalidate_patterns(user_id: i64) -> Vec<String> {
        vec![
            format!("movements_summary_*_{}", user_id),
            format!("movements_summary_user_{}", user_id),
            "movements_summary_*".to_string(),
        ]
    }
}

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Response model for rewards balance query
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct RewardsBalanceResponse {
    pub user_id: i64,
    pub balance: i32,
    pub latest_update: DateTime<Utc>,
}

/// Query templates for rewards balance operations
pub struct RewardsBalanceQueryTemplates;

impl RewardsBalanceQueryTemplates {
    /// Get user rewards balance query
    pub fn get_user_balance_query() -> &'static str {
        "SELECT user_id, balance, latest_update FROM rewards.fact_balance_points WHERE user_id = $1"
    }

    /// Get all rewards balances query (for admin/debugging)
    pub fn get_all_balances_query() -> &'static str {
        "SELECT user_id, balance, latest_update FROM rewards.fact_balance_points ORDER BY latest_update DESC LIMIT 100"
    }

    /// Cache key prefix for user balance
    pub fn get_user_balance_cache_key_prefix() -> &'static str {
        "rewards_balance_user"
    }

    /// Cache key prefix for all balances
    pub fn get_all_balances_cache_key() -> &'static str {
        "rewards_balance_all"
    }
}

/// Helper functions for rewards balance operations
pub struct RewardsBalanceHelpers;

impl RewardsBalanceHelpers {
    /// Format balance message for user-friendly display
    pub fn format_balance_message(balance: i32) -> String {
        match balance {
            0 => "🎯 No tienes puntos de recompensa aún. ¡Empieza a ganar puntos!".to_string(),
            1..=10 => format!("🌟 Tienes {} puntos de recompensa. ¡Sigue así!", balance),
            11..=50 => format!("🏆 ¡Excelente! Tienes {} puntos de recompensa.", balance),
            51..=100 => format!("🎉 ¡Increíble! Tienes {} puntos de recompensa.", balance),
            _ => format!("👑 ¡Eres un campeón! Tienes {} puntos de recompensa.", balance),
        }
    }

    /// Check if balance is recent (updated within last 30 days)
    pub fn is_balance_recent(latest_update: DateTime<Utc>) -> bool {
        let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
        latest_update > thirty_days_ago
    }
}

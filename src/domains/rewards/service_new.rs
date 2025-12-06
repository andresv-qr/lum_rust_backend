use crate::{
    api::common::Result,
    models::rewards::{UserInvoiceSummary, UserSummaryResponse, UserSummaryQuery, PerformanceMetrics, TrendAnalysis},
};
use chrono::{DateTime, Utc};
use shared::cache::CacheManager;
use shared::database::DatabaseManager;
use shared::auth::AuthManager;
use shared::service_client::ServiceClient;
use shared::utils::UtilsManager;
use sqlx::{types::Json, PgPool};
use std::sync::Arc;

#[derive(Debug, sqlx::FromRow)]
pub struct Redemption {
    pub redem_id: Option<String>,
    pub quantity: Option<i32>,
    pub date: Option<DateTime<Utc>>,
    #[sqlx(rename = "condition1")]
    pub condition: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Reward {
    pub id: i32,
    pub name: Option<String>,
    pub points: Option<i32>,
}

pub struct RewardsService {
    pub db: Arc<DatabaseManager>,
    pub cache: Arc<CacheManager>,
    pub auth: Arc<AuthManager>,
    pub service_client: Arc<ServiceClient>,
    pub utils: Arc<UtilsManager>,
}

impl RewardsService {
    pub fn new(
        db: Arc<DatabaseManager>,
        cache: Arc<CacheManager>,
        auth: Arc<AuthManager>,
        service_client: Arc<ServiceClient>,
        utils: Arc<UtilsManager>,
    ) -> Self {
        Self {
            db,
            cache,
            auth,
            service_client,
            utils,
        }
    }

    pub async fn get_user_summary(
        &self,
        user_id: i32,
        query_params: Option<UserSummaryQuery>,
    ) -> Result<UserSummaryResponse> {
        // Query user invoice summary
        let summary = self.get_user_invoice_summary(user_id).await?;
        
        // Calculate performance metrics
        let performance_metrics = self.calculate_performance_metrics(&summary).await?;
        
        // Calculate trends if requested
        let trends = if query_params.as_ref().and_then(|q| q.include_trends).unwrap_or(false) {
            self.calculate_trend_analysis(&summary).await?
        } else {
            TrendAnalysis::default()
        };

        Ok(UserSummaryResponse {
            summary,
            performance_metrics,
            trends,
        })
    }

    async fn get_user_invoice_summary(&self, user_id: i32) -> Result<UserInvoiceSummary> {
        let query = r#"
            SELECT user_id, total_facturas, total_monto, total_items, 
                   n_descuentos, total_descuento, top_emisores, 
                   top_categorias, serie_mensual, updated_at, 
                   comparativo_categoria
            FROM rewards.user_invoice_summary 
            WHERE user_id = $1
        "#;

        let summary = sqlx::query_as::<_, UserInvoiceSummary>(query)
            .bind(user_id)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| crate::api::common::ApiError::NotFound("User summary not found".to_string()))?;

        Ok(summary)
    }

    async fn calculate_performance_metrics(&self, summary: &UserInvoiceSummary) -> Result<PerformanceMetrics> {
        // Basic performance metrics calculation
        let mut metrics = PerformanceMetrics::default();
        
        if let Some(total_monto) = summary.total_monto {
            metrics.spending_tier = if total_monto > 1000.0 {
                "Premium".to_string()
            } else if total_monto > 500.0 {
                "Standard".to_string()
            } else {
                "Basic".to_string()
            };
            
            metrics.lumis_efficiency = total_monto / 100.0; // Simple calculation
        }
        
        if let Some(total_facturas) = summary.total_facturas {
            metrics.invoice_frequency_score = total_facturas as f64 / 30.0; // Invoices per month
        }

        Ok(metrics)
    }

    async fn calculate_trend_analysis(&self, summary: &UserInvoiceSummary) -> Result<TrendAnalysis> {
        let mut trends = TrendAnalysis::default();
        
        if let Some(total_facturas) = summary.total_facturas {
            trends.avg_monthly_invoices = total_facturas as f64;
            trends.monthly_trend = if total_facturas > 10 {
                "increasing".to_string()
            } else {
                "stable".to_string()
            };
        }

        Ok(trends)
    }
}

pub async fn get_reward_by_id(pool: &PgPool, redemption_id: i32) -> Result<Option<Reward>> {
    let reward = sqlx::query_as::<_, Reward>(
        "SELECT id, name, points FROM rewards.dim_redemptions WHERE id = $1"
    )
    .bind(redemption_id)
    .fetch_optional(pool)
    .await?;
    Ok(reward)
}

pub async fn redeem_reward(pool: &PgPool, user_id: i64, reward: &Reward) -> Result<()> {
    let mut tx = pool.begin().await?;

    let balance_record = sqlx::query!(
        "SELECT balance::integer FROM rewards.fact_balance_points WHERE user_id = $1 FOR UPDATE",
        user_id as i32
    )
    .fetch_optional(&mut *tx)
    .await?;

    let current_balance = balance_record
        .map(|r| r.balance.unwrap_or(0))
        .unwrap_or(0);

    let required_points = reward.points.unwrap_or(0);
    if current_balance < required_points {
        return Err(crate::api::common::ApiError::BadRequest("Insufficient points".to_string()));
    }

    let new_balance = current_balance - required_points;

    sqlx::query!(
        "UPDATE rewards.fact_balance_points SET balance = $1, updated_at = NOW() WHERE user_id = $2",
        new_balance,
        user_id as i32
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO rewards.fact_redemptions_legacy (user_id, reward_id, points_used, created_at) VALUES ($1, $2, $3, NOW())",
        user_id as i32,
        reward.id,
        required_points
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

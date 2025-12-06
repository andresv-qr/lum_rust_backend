// ============================================================================
// MERCHANT ANALYTICS API - Dashboard de métricas para merchants
// ============================================================================

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use chrono::{DateTime, Utc, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::error;

use crate::{
    middleware::auth::MerchantClaims,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// Date range: "today", "week", "month", "custom"
    pub range: Option<String>,
    /// Start date for custom range (ISO 8601)
    pub start_date: Option<String>,
    /// End date for custom range (ISO 8601)
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MerchantAnalytics {
    pub summary: AnalyticsSummary,
    pub redemptions_by_day: Vec<DailyRedemptions>,
    pub peak_hours: Vec<HourlyRedemptions>,
    pub popular_offers: Vec<OfferStats>,
    pub average_confirmation_time: f64, // in minutes
    pub expiration_rate: f64, // percentage
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSummary {
    pub total_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub pending_redemptions: i64,
    pub expired_redemptions: i64,
    pub cancelled_redemptions: i64,
    pub total_lumis: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyRedemptions {
    pub date: NaiveDate,
    pub count: i64,
    pub lumis: i64,
}

#[derive(Debug, Serialize)]
pub struct HourlyRedemptions {
    pub hour: i32,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct OfferStats {
    pub offer_id: String,
    pub offer_name: String,
    pub redemption_count: i64,
    pub total_lumis: i64,
}

/// Get merchant analytics
/// 
/// # Endpoint
/// GET /api/v1/merchant/analytics
/// 
/// # Query Parameters
/// - range: "today", "week", "month", "custom" (default: "week")
/// - start_date: ISO 8601 date for custom range
/// - end_date: ISO 8601 date for custom range
/// 
/// # Authentication
/// Requires merchant JWT token
/// 
/// # Returns
/// - 200 OK: Analytics data
/// - 401 Unauthorized: Invalid merchant token
/// - 500 Internal Server Error: Database error
pub async fn get_merchant_analytics(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<MerchantAnalytics>, ApiError> {
    let merchant_id = uuid::Uuid::parse_str(&merchant.sub)
        .map_err(|_| ApiError::BadRequest("Invalid merchant ID".to_string()))?;

    // Determinar rango de fechas
    let (start_date, end_date) = match query.range.as_deref() {
        Some("today") => {
            let today = Utc::now().date_naive();
            let start = today.and_hms_opt(0, 0, 0).unwrap();
            let end = today.and_hms_opt(23, 59, 59).unwrap();
            (DateTime::from_naive_utc_and_offset(start, Utc), 
             DateTime::from_naive_utc_and_offset(end, Utc))
        }
        Some("month") => {
            let end = Utc::now();
            let start = end - Duration::days(30);
            (start, end)
        }
        Some("custom") => {
            let start = query.start_date
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .ok_or_else(|| ApiError::BadRequest("Invalid start_date".to_string()))?
                .with_timezone(&Utc);
            let end = query.end_date
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .ok_or_else(|| ApiError::BadRequest("Invalid end_date".to_string()))?
                .with_timezone(&Utc);
            (start, end)
        }
        _ => {
            // Default: last 7 days
            let end = Utc::now();
            let start = end - Duration::days(7);
            (start, end)
        }
    };

    // 1. Summary stats
    let summary = get_summary_stats(&state.db_pool, merchant_id, start_date, end_date).await?;

    // 2. Redemptions by day
    let redemptions_by_day = get_daily_redemptions(&state.db_pool, merchant_id, start_date, end_date).await?;

    // 3. Peak hours
    let peak_hours = get_peak_hours(&state.db_pool, merchant_id, start_date, end_date).await?;

    // 4. Popular offers
    let popular_offers = get_popular_offers(&state.db_pool, merchant_id, start_date, end_date).await?;

    // 5. Average confirmation time
    let avg_confirmation_time = get_avg_confirmation_time(&state.db_pool, merchant_id, start_date, end_date).await?;

    // 6. Expiration rate
    let expiration_rate = calculate_expiration_rate(&summary);

    Ok(Json(MerchantAnalytics {
        summary,
        redemptions_by_day,
        peak_hours,
        popular_offers,
        average_confirmation_time: avg_confirmation_time,
        expiration_rate,
    }))
}

async fn get_summary_stats(
    db: &PgPool,
    merchant_id: uuid::Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<AnalyticsSummary, ApiError> {
    let result = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE redemption_status = 'confirmed') as confirmed,
            COUNT(*) FILTER (WHERE redemption_status = 'pending') as pending,
            COUNT(*) FILTER (WHERE redemption_status = 'expired') as expired,
            COUNT(*) FILTER (WHERE redemption_status = 'cancelled') as cancelled,
            COALESCE(SUM(lumis_spent), 0) as total_lumis
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
          AND ur.created_at BETWEEN $2 AND $3
        "#,
        merchant_id,
        start_date,
        end_date
    )
    .fetch_one(db)
    .await
    .map_err(|e| {
        error!("Database error getting summary stats: {}", e);
        ApiError::InternalError("Error al obtener estadísticas".to_string())
    })?;

    Ok(AnalyticsSummary {
        total_redemptions: result.total.unwrap_or(0),
        confirmed_redemptions: result.confirmed.unwrap_or(0),
        pending_redemptions: result.pending.unwrap_or(0),
        expired_redemptions: result.expired.unwrap_or(0),
        cancelled_redemptions: result.cancelled.unwrap_or(0),
        total_lumis: result.total_lumis.unwrap_or(0) as i64,
    })
}

async fn get_daily_redemptions(
    db: &PgPool,
    merchant_id: uuid::Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<DailyRedemptions>, ApiError> {
    let results = sqlx::query_as!(
        DailyRedemptions,
        r#"
        SELECT 
            DATE(ur.created_at) as "date!",
            COUNT(*) as "count!",
            COALESCE(SUM(ur.lumis_spent), 0) as "lumis!"
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
          AND ur.created_at BETWEEN $2 AND $3
        GROUP BY DATE(ur.created_at)
        ORDER BY DATE(ur.created_at)
        "#,
        merchant_id,
        start_date,
        end_date
    )
    .fetch_all(db)
    .await
    .map_err(|e| {
        error!("Database error getting daily redemptions: {}", e);
        ApiError::InternalError("Error al obtener redenciones diarias".to_string())
    })?;

    Ok(results)
}

async fn get_peak_hours(
    db: &PgPool,
    merchant_id: uuid::Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<HourlyRedemptions>, ApiError> {
    let results = sqlx::query_as!(
        HourlyRedemptions,
        r#"
        SELECT 
            EXTRACT(HOUR FROM ur.created_at)::integer as "hour!",
            COUNT(*) as "count!"
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
          AND ur.created_at BETWEEN $2 AND $3
        GROUP BY EXTRACT(HOUR FROM ur.created_at)
        ORDER BY EXTRACT(HOUR FROM ur.created_at)
        "#,
        merchant_id,
        start_date,
        end_date
    )
    .fetch_all(db)
    .await
    .map_err(|e| {
        error!("Database error getting peak hours: {}", e);
        ApiError::InternalError("Error al obtener horarios pico".to_string())
    })?;

    Ok(results)
}

async fn get_popular_offers(
    db: &PgPool,
    merchant_id: uuid::Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<OfferStats>, ApiError> {
    let results = sqlx::query!(
        r#"
        SELECT 
            ro.offer_id::text,
            ro.name_friendly as offer_name,
            COUNT(*) as redemption_count,
            COALESCE(SUM(ur.lumis_spent), 0) as total_lumis
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
          AND ur.created_at BETWEEN $2 AND $3
        GROUP BY ro.offer_id, ro.name_friendly
        ORDER BY COUNT(*) DESC
        LIMIT 10
        "#,
        merchant_id,
        start_date,
        end_date
    )
    .fetch_all(db)
    .await
    .map_err(|e| {
        error!("Database error getting popular offers: {}", e);
        ApiError::InternalError("Error al obtener ofertas populares".to_string())
    })?;

    Ok(results
        .into_iter()
        .map(|r| OfferStats {
            offer_id: r.offer_id.unwrap_or_default(),
            offer_name: r.offer_name.unwrap_or_else(|| "N/A".to_string()),
            redemption_count: r.redemption_count.unwrap_or(0),
            total_lumis: r.total_lumis.unwrap_or(0) as i64,
        })
        .collect())
}

async fn get_avg_confirmation_time(
    db: &PgPool,
    merchant_id: uuid::Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<f64, ApiError> {
    let result = sqlx::query!(
        r#"
        SELECT 
            AVG(EXTRACT(EPOCH FROM (ur.validated_at - ur.created_at)) / 60.0) as avg_minutes
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
          AND ur.redemption_status = 'confirmed'
          AND ur.validated_at IS NOT NULL
          AND ur.created_at BETWEEN $2 AND $3
        "#,
        merchant_id,
        start_date,
        end_date
    )
    .fetch_one(db)
    .await
    .map_err(|e| {
        error!("Database error getting avg confirmation time: {}", e);
        ApiError::InternalError("Error al calcular tiempo promedio".to_string())
    })?;

    // Convert Decimal to f64
    let avg_time = result.avg_minutes
        .and_then(|d| d.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);
    
    Ok(avg_time)
}

fn calculate_expiration_rate(summary: &AnalyticsSummary) -> f64 {
    if summary.total_redemptions == 0 {
        return 0.0;
    }
    (summary.expired_redemptions as f64 / summary.total_redemptions as f64) * 100.0
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "success": false,
            "error": message,
        }));

        (status, body).into_response()
    }
}

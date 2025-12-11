// ============================================================================
// MERCHANT DASHBOARD - Estadísticas y métricas para comercios
// ============================================================================

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

use crate::{
    middleware::auth::MerchantClaims,
    state::AppState,
};

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    /// Período: today, week, month, year, all (default: month)
    pub period: Option<String>,
    /// Fecha inicio personalizada (ISO 8601)
    pub start_date: Option<String>,
    /// Fecha fin personalizada (ISO 8601)
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub success: bool,
    pub merchant_name: String,
    pub period: String,
    pub overview: DashboardOverview,
    pub trends: DashboardTrends,
    pub top_offers: Vec<TopOffer>,
    pub recent_redemptions: Vec<RecentRedemption>,
}

#[derive(Debug, Serialize, Default)]
pub struct DashboardOverview {
    pub total_offers: i64,
    pub active_offers: i64,
    pub total_redemptions: i64,
    pub pending_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub cancelled_redemptions: i64,
    pub expired_redemptions: i64,
    pub total_lumis_redeemed: i64,
    pub average_lumis_per_redemption: f64,
    pub conversion_rate: f64,  // confirmed / total
}

#[derive(Debug, Serialize, Default)]
pub struct DashboardTrends {
    pub redemptions_today: i64,
    pub redemptions_this_week: i64,
    pub redemptions_this_month: i64,
    pub week_over_week_change: f64,  // Porcentaje de cambio
    pub month_over_month_change: f64,
    pub daily_breakdown: Vec<DailyStats>,
}

#[derive(Debug, Serialize)]
pub struct DailyStats {
    pub date: String,
    pub redemptions: i64,
    pub lumis: i64,
}

#[derive(Debug, Serialize)]
pub struct TopOffer {
    pub offer_id: String,
    pub offer_name: String,
    pub lumis_cost: i32,
    pub total_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub stock_remaining: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct RecentRedemption {
    pub redemption_id: String,
    pub redemption_code: String,
    pub offer_name: String,
    pub lumis_spent: i32,
    pub status: String,
    pub created_at: String,
    pub confirmed_at: Option<String>,
}

// ============================================================================
// Endpoints
// ============================================================================

/// Dashboard principal del merchant
/// GET /api/v1/merchant/dashboard
pub async fn merchant_dashboard(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Query(params): Query<DashboardQuery>,
) -> Result<Json<DashboardResponse>, ApiError> {
    let merchant_id = merchant.get_merchant_id()
        .ok_or_else(|| ApiError::Unauthorized("ID de comercio no válido".to_string()))?;
    
    let period = params.period.as_deref().unwrap_or("month");
    
    // Calcular rango de fechas según el período
    let (start_date, end_date) = calculate_date_range(period, &params)?;
    
    // 1. Overview de estadísticas
    let overview = get_overview(&state.db_pool, merchant_id, &start_date, &end_date).await?;
    
    // 2. Tendencias
    let trends = get_trends(&state.db_pool, merchant_id).await?;
    
    // 3. Top ofertas
    let top_offers = get_top_offers(&state.db_pool, merchant_id, &start_date, &end_date).await?;
    
    // 4. Redenciones recientes
    let recent_redemptions = get_recent_redemptions(&state.db_pool, merchant_id, 10).await?;
    
    Ok(Json(DashboardResponse {
        success: true,
        merchant_name: merchant.merchant_name.clone(),
        period: period.to_string(),
        overview,
        trends,
        top_offers,
        recent_redemptions,
    }))
}

/// Obtener solo las estadísticas generales
/// GET /api/v1/merchant/stats
pub async fn merchant_stats(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Query(params): Query<DashboardQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let merchant_id = merchant.get_merchant_id()
        .ok_or_else(|| ApiError::Unauthorized("ID de comercio no válido".to_string()))?;
    
    let period = params.period.as_deref().unwrap_or("month");
    let (start_date, end_date) = calculate_date_range(period, &params)?;
    
    let overview = get_overview(&state.db_pool, merchant_id, &start_date, &end_date).await?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "merchant_name": merchant.merchant_name,
        "period": period,
        "statistics": overview
    })))
}

/// Obtener redenciones pendientes por confirmar
/// GET /api/v1/merchant/pending
pub async fn pending_redemptions(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Query(params): Query<PendingQuery>,
) -> Result<Json<PendingResponse>, ApiError> {
    let merchant_id = merchant.get_merchant_id()
        .ok_or_else(|| ApiError::Unauthorized("ID de comercio no válido".to_string()))?;
    
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    
    let pending: Vec<PendingRedemption> = sqlx::query_as(
        r#"
        SELECT 
            ur.redemption_id::text,
            ur.redemption_code,
            ro.name_friendly as offer_name,
            ur.lumis_spent,
            ur.created_at::text,
            ur.code_expires_at::text,
            EXTRACT(EPOCH FROM (ur.code_expires_at - NOW()))::int as seconds_until_expiry
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
          AND ur.redemption_status = 'pending'
          AND ur.code_expires_at > NOW()
        ORDER BY ur.code_expires_at ASC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(merchant_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to get pending redemptions: {}", e);
        ApiError::InternalError("Error al obtener redenciones pendientes".to_string())
    })?;
    
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
          AND ur.redemption_status = 'pending'
          AND ur.code_expires_at > NOW()
        "#
    )
    .bind(merchant_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);
    
    Ok(Json(PendingResponse {
        success: true,
        pending,
        total,
        limit,
        offset,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PendingQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct PendingResponse {
    pub success: bool,
    pub pending: Vec<PendingRedemption>,
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PendingRedemption {
    pub redemption_id: String,
    pub redemption_code: String,
    pub offer_name: String,
    pub lumis_spent: i32,
    pub created_at: String,
    pub code_expires_at: String,
    pub seconds_until_expiry: i32,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn calculate_date_range(
    period: &str,
    params: &DashboardQuery,
) -> Result<(String, String), ApiError> {
    use chrono::{Duration, Utc};
    
    // Si hay fechas personalizadas, usarlas
    if let (Some(start), Some(end)) = (&params.start_date, &params.end_date) {
        return Ok((start.clone(), end.clone()));
    }
    
    let now = Utc::now();
    let end = now.format("%Y-%m-%d 23:59:59").to_string();
    
    let start = match period {
        "today" => now.format("%Y-%m-%d 00:00:00").to_string(),
        "week" => (now - Duration::days(7)).format("%Y-%m-%d 00:00:00").to_string(),
        "month" => (now - Duration::days(30)).format("%Y-%m-%d 00:00:00").to_string(),
        "year" => (now - Duration::days(365)).format("%Y-%m-%d 00:00:00").to_string(),
        "all" => "2020-01-01 00:00:00".to_string(),
        _ => (now - Duration::days(30)).format("%Y-%m-%d 00:00:00").to_string(),
    };
    
    Ok((start, end))
}

async fn get_overview(
    pool: &sqlx::PgPool,
    merchant_id: uuid::Uuid,
    start_date: &str,
    end_date: &str,
) -> Result<DashboardOverview, ApiError> {
    #[derive(sqlx::FromRow)]
    struct StatsRow {
        total_offers: Option<i64>,
        active_offers: Option<i64>,
        total_redemptions: Option<i64>,
        pending_redemptions: Option<i64>,
        confirmed_redemptions: Option<i64>,
        cancelled_redemptions: Option<i64>,
        expired_redemptions: Option<i64>,
        total_lumis_redeemed: Option<i64>,
    }
    
    let stats: StatsRow = sqlx::query_as(
        r#"
        SELECT 
            COUNT(DISTINCT ro.offer_id)::bigint as total_offers,
            COUNT(DISTINCT ro.offer_id) FILTER (WHERE ro.is_active = true)::bigint as active_offers,
            COUNT(ur.redemption_id)::bigint as total_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'pending')::bigint as pending_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed')::bigint as confirmed_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'cancelled')::bigint as cancelled_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'expired')::bigint as expired_redemptions,
            COALESCE(SUM(ur.lumis_spent) FILTER (WHERE ur.redemption_status = 'confirmed'), 0)::bigint as total_lumis_redeemed
        FROM rewards.redemption_offers ro
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
            AND ur.created_at >= $2::timestamp
            AND ur.created_at <= $3::timestamp
        WHERE ro.merchant_id = $1
        "#
    )
    .bind(merchant_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!("Failed to get overview: {}", e);
        ApiError::InternalError("Error al obtener estadísticas".to_string())
    })?;
    
    let total = stats.total_redemptions.unwrap_or(0);
    let confirmed = stats.confirmed_redemptions.unwrap_or(0);
    let lumis = stats.total_lumis_redeemed.unwrap_or(0);
    
    Ok(DashboardOverview {
        total_offers: stats.total_offers.unwrap_or(0),
        active_offers: stats.active_offers.unwrap_or(0),
        total_redemptions: total,
        pending_redemptions: stats.pending_redemptions.unwrap_or(0),
        confirmed_redemptions: confirmed,
        cancelled_redemptions: stats.cancelled_redemptions.unwrap_or(0),
        expired_redemptions: stats.expired_redemptions.unwrap_or(0),
        total_lumis_redeemed: lumis,
        average_lumis_per_redemption: if confirmed > 0 { lumis as f64 / confirmed as f64 } else { 0.0 },
        conversion_rate: if total > 0 { (confirmed as f64 / total as f64) * 100.0 } else { 0.0 },
    })
}

async fn get_trends(
    pool: &sqlx::PgPool,
    merchant_id: uuid::Uuid,
) -> Result<DashboardTrends, ApiError> {
    // Redenciones por período
    let today: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1 AND ur.created_at::date = CURRENT_DATE
        "#
    )
    .bind(merchant_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    let this_week: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1 AND ur.created_at >= CURRENT_DATE - INTERVAL '7 days'
        "#
    )
    .bind(merchant_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    let this_month: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1 AND ur.created_at >= CURRENT_DATE - INTERVAL '30 days'
        "#
    )
    .bind(merchant_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    let last_week: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1 
          AND ur.created_at >= CURRENT_DATE - INTERVAL '14 days'
          AND ur.created_at < CURRENT_DATE - INTERVAL '7 days'
        "#
    )
    .bind(merchant_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    let last_month: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1 
          AND ur.created_at >= CURRENT_DATE - INTERVAL '60 days'
          AND ur.created_at < CURRENT_DATE - INTERVAL '30 days'
        "#
    )
    .bind(merchant_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    
    // Daily breakdown (últimos 7 días)
    let daily: Vec<DailyStatsRow> = sqlx::query_as(
        r#"
        SELECT 
            ur.created_at::date::text as date,
            COUNT(*)::bigint as redemptions,
            COALESCE(SUM(ur.lumis_spent), 0)::bigint as lumis
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1 AND ur.created_at >= CURRENT_DATE - INTERVAL '7 days'
        GROUP BY ur.created_at::date
        ORDER BY ur.created_at::date DESC
        "#
    )
    .bind(merchant_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    
    let daily_breakdown: Vec<DailyStats> = daily.into_iter().map(|d| DailyStats {
        date: d.date,
        redemptions: d.redemptions,
        lumis: d.lumis,
    }).collect();
    
    let wow_change = if last_week > 0 {
        ((this_week as f64 - last_week as f64) / last_week as f64) * 100.0
    } else if this_week > 0 {
        100.0
    } else {
        0.0
    };
    
    let mom_change = if last_month > 0 {
        ((this_month as f64 - last_month as f64) / last_month as f64) * 100.0
    } else if this_month > 0 {
        100.0
    } else {
        0.0
    };
    
    Ok(DashboardTrends {
        redemptions_today: today,
        redemptions_this_week: this_week,
        redemptions_this_month: this_month,
        week_over_week_change: wow_change,
        month_over_month_change: mom_change,
        daily_breakdown,
    })
}

#[derive(sqlx::FromRow)]
struct DailyStatsRow {
    date: String,
    redemptions: i64,
    lumis: i64,
}

async fn get_top_offers(
    pool: &sqlx::PgPool,
    merchant_id: uuid::Uuid,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<TopOffer>, ApiError> {
    let offers: Vec<TopOfferRow> = sqlx::query_as(
        r#"
        SELECT 
            ro.offer_id::text,
            COALESCE(ro.name_friendly, ro.name) as offer_name,
            COALESCE(ro.lumis_cost, ro.points) as lumis_cost,
            COUNT(ur.redemption_id)::bigint as total_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed')::bigint as confirmed_redemptions,
            ro.stock_quantity
        FROM rewards.redemption_offers ro
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
            AND ur.created_at >= $2::timestamp
            AND ur.created_at <= $3::timestamp
        WHERE ro.merchant_id = $1
        GROUP BY ro.offer_id, ro.name_friendly, ro.name, ro.lumis_cost, ro.points, ro.stock_quantity
        ORDER BY total_redemptions DESC
        LIMIT 10
        "#
    )
    .bind(merchant_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!("Failed to get top offers: {}", e);
        ApiError::InternalError("Error al obtener ofertas top".to_string())
    })?;
    
    Ok(offers.into_iter().map(|o| TopOffer {
        offer_id: o.offer_id,
        offer_name: o.offer_name,
        lumis_cost: o.lumis_cost,
        total_redemptions: o.total_redemptions,
        confirmed_redemptions: o.confirmed_redemptions,
        stock_remaining: o.stock_quantity,
    }).collect())
}

#[derive(sqlx::FromRow)]
struct TopOfferRow {
    offer_id: String,
    offer_name: String,
    lumis_cost: i32,
    total_redemptions: i64,
    confirmed_redemptions: i64,
    stock_quantity: Option<i32>,
}

async fn get_recent_redemptions(
    pool: &sqlx::PgPool,
    merchant_id: uuid::Uuid,
    limit: i32,
) -> Result<Vec<RecentRedemption>, ApiError> {
    let recent: Vec<RecentRedemptionRow> = sqlx::query_as(
        r#"
        SELECT 
            ur.redemption_id::text,
            ur.redemption_code,
            COALESCE(ro.name_friendly, ro.name) as offer_name,
            ur.lumis_spent,
            ur.redemption_status as status,
            ur.created_at::text,
            ur.validated_at::text as confirmed_at
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ro.merchant_id = $1
        ORDER BY ur.created_at DESC
        LIMIT $2
        "#
    )
    .bind(merchant_id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!("Failed to get recent redemptions: {}", e);
        ApiError::InternalError("Error al obtener redenciones recientes".to_string())
    })?;
    
    Ok(recent.into_iter().map(|r| RecentRedemption {
        redemption_id: r.redemption_id,
        redemption_code: r.redemption_code,
        offer_name: r.offer_name,
        lumis_spent: r.lumis_spent,
        status: r.status,
        created_at: r.created_at,
        confirmed_at: r.confirmed_at,
    }).collect())
}

#[derive(sqlx::FromRow)]
struct RecentRedemptionRow {
    redemption_id: String,
    redemption_code: String,
    offer_name: String,
    lumis_spent: i32,
    status: String,
    created_at: String,
    confirmed_at: Option<String>,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        
        let body = Json(serde_json::json!({
            "success": false,
            "error": message,
        }));
        
        (status, body).into_response()
    }
}

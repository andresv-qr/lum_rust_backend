// ============================================================================
// MERCHANT STATS - Estadísticas de redenciones para comercio
// ============================================================================

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension,
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    middleware::auth::MerchantClaims,
    state::AppState,
};

/// Response with merchant statistics
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub success: bool,
    pub stats: MerchantStats,
}

/// Merchant statistics
#[derive(Debug, Serialize)]
pub struct MerchantStats {
    pub total_redemptions: i64,
    pub pending_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub today_redemptions: i64,
    pub this_week_redemptions: i64,
    pub this_month_redemptions: i64,
    pub total_lumis_redeemed: i64,
    pub recent_redemptions: Vec<RecentRedemption>,
}

/// Recent redemption item
#[derive(Debug, Serialize)]
pub struct RecentRedemption {
    pub redemption_id: String,
    pub redemption_code: String,
    pub offer_name: String,
    pub lumis_spent: i32,
    pub status: String,
    pub created_at: String,
    pub validated_at: Option<String>,
}

/// Get merchant statistics
/// 
/// # Endpoint
/// GET /api/v1/merchant/stats
/// 
/// # Authentication
/// Requires merchant JWT token
/// 
/// # Returns
/// - 200 OK: Statistics data
/// - 401 Unauthorized: Invalid merchant token
/// - 500 Internal Server Error: Database error
pub async fn get_merchant_stats(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
) -> Result<Json<StatsResponse>, ApiError> {
    info!("Fetching merchant statistics for: {} ({})", 
          merchant.merchant_name, merchant.sub);
    
    // Get aggregate stats
    let stats_query = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total_redemptions,
            COUNT(*) FILTER (WHERE redemption_status = 'pending') as pending_redemptions,
            COUNT(*) FILTER (WHERE redemption_status = 'confirmed') as confirmed_redemptions,
            COUNT(*) FILTER (WHERE DATE(created_at) = CURRENT_DATE) as today_redemptions,
            COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days') as this_week_redemptions,
            COUNT(*) FILTER (WHERE DATE_TRUNC('month', created_at) = DATE_TRUNC('month', CURRENT_DATE)) as this_month_redemptions,
            COALESCE(SUM(lumis_spent) FILTER (WHERE redemption_status = 'confirmed'), 0) as total_lumis_redeemed
        FROM rewards.user_redemptions
        "#
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch merchant stats: {}", e);
        ApiError::InternalError("Error al consultar estadísticas".to_string())
    })?;
    
    // Get recent redemptions (last 10)
    let recent = sqlx::query!(
        r#"
        SELECT 
            ur.redemption_id::text,
            ur.redemption_code,
            ur.redemption_status,
            ur.lumis_spent,
            ur.created_at,
            ur.validated_at,
            ro.name_friendly as offer_name
        FROM rewards.user_redemptions ur
        INNER JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        ORDER BY ur.created_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch recent redemptions: {}", e);
        ApiError::InternalError("Error al consultar redenciones recientes".to_string())
    })?;
    
    let recent_redemptions = recent
        .into_iter()
        .map(|r| RecentRedemption {
            redemption_id: r.redemption_id.unwrap_or_else(|| "N/A".to_string()),
            redemption_code: r.redemption_code,
            offer_name: r.offer_name.unwrap_or_else(|| "N/A".to_string()),
            lumis_spent: r.lumis_spent,
            status: r.redemption_status,
            created_at: r.created_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "N/A".to_string()),
            validated_at: r.validated_at.map(|dt| dt.to_rfc3339()),
        })
        .collect();
    
    info!("Successfully fetched merchant statistics");
    
    Ok(Json(StatsResponse {
        success: true,
        stats: MerchantStats {
            total_redemptions: stats_query.total_redemptions.unwrap_or(0),
            pending_redemptions: stats_query.pending_redemptions.unwrap_or(0),
            confirmed_redemptions: stats_query.confirmed_redemptions.unwrap_or(0),
            today_redemptions: stats_query.today_redemptions.unwrap_or(0),
            this_week_redemptions: stats_query.this_week_redemptions.unwrap_or(0),
            this_month_redemptions: stats_query.this_month_redemptions.unwrap_or(0),
            total_lumis_redeemed: stats_query.total_lumis_redeemed.unwrap_or(0),
            recent_redemptions,
        },
    }))
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
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

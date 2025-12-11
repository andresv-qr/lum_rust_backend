// ============================================================================
// REPORTS & EXPORTS - Generación de reportes y exportación de datos
// ============================================================================

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
    body::Body,
};
use axum::http::header;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

use crate::{
    middleware::auth::{JwtClaims, MerchantClaims},
    state::AppState,
};

// ============================================================================
// Request Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    /// Tipo de reporte: redemptions, offers, merchants, summary
    pub report_type: String,
    /// Formato: json, csv
    pub format: Option<String>,
    /// Fecha inicio (YYYY-MM-DD)
    pub start_date: Option<String>,
    /// Fecha fin (YYYY-MM-DD)
    pub end_date: Option<String>,
    /// Estado de redención (para filtrar)
    pub status: Option<String>,
    /// Merchant ID (para admin)
    pub merchant_id: Option<uuid::Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// Fecha inicio
    pub start_date: Option<String>,
    /// Fecha fin
    pub end_date: Option<String>,
    /// Estado
    pub status: Option<String>,
}

// ============================================================================
// Response Models
// ============================================================================

#[derive(Debug, Serialize)]
pub struct RedemptionReportItem {
    pub redemption_id: String,
    pub redemption_code: String,
    pub user_id: i32,
    pub offer_name: String,
    pub merchant_name: String,
    pub lumis_spent: i32,
    pub status: String,
    pub created_at: String,
    pub validated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OfferReportItem {
    pub offer_id: String,
    pub name: String,
    pub merchant_name: String,
    pub lumis_cost: i32,
    pub stock_quantity: Option<i32>,
    pub is_active: bool,
    pub total_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub revenue_lumis: i64,
}

#[derive(Debug, Serialize)]
pub struct MerchantReportItem {
    pub merchant_id: String,
    pub merchant_name: String,
    pub is_active: bool,
    pub total_offers: i64,
    pub active_offers: i64,
    pub total_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub total_lumis: i64,
}

#[derive(Debug, Serialize)]
pub struct SummaryReport {
    pub period: String,
    pub total_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub pending_redemptions: i64,
    pub cancelled_redemptions: i64,
    pub expired_redemptions: i64,
    pub total_lumis_redeemed: i64,
    pub total_offers: i64,
    pub active_offers: i64,
    pub total_merchants: i64,
    pub conversion_rate: f64,
    pub avg_lumis_per_redemption: f64,
}

// ============================================================================
// Admin Reports
// ============================================================================

/// Generar reporte (Admin)
/// GET /api/v1/admin/reports
pub async fn admin_generate_report(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Query(params): Query<ReportQuery>,
) -> Result<Response, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    let format = params.format.as_deref().unwrap_or("json");
    let (start_date, end_date) = get_date_range(&params.start_date, &params.end_date);

    match params.report_type.as_str() {
        "redemptions" => {
            let data = get_redemptions_report(&state.db_pool, &start_date, &end_date, params.status.as_deref(), params.merchant_id).await?;
            format_response(data, format, "redemptions_report")
        }
        "offers" => {
            let data = get_offers_report(&state.db_pool, &start_date, &end_date, params.merchant_id).await?;
            format_response(data, format, "offers_report")
        }
        "merchants" => {
            let data = get_merchants_report(&state.db_pool, &start_date, &end_date).await?;
            format_response(data, format, "merchants_report")
        }
        "summary" => {
            let data = get_summary_report(&state.db_pool, &start_date, &end_date).await?;
            Ok(Json(serde_json::json!({
                "success": true,
                "report": data
            })).into_response())
        }
        _ => Err(ApiError::BadRequest("Tipo de reporte no válido".to_string()))
    }
}

/// Exportar redenciones a CSV (Admin)
/// GET /api/v1/admin/export/redemptions
pub async fn admin_export_redemptions(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Query(params): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    let (start_date, end_date) = get_date_range(&params.start_date, &params.end_date);
    let data = get_redemptions_report(&state.db_pool, &start_date, &end_date, params.status.as_deref(), None).await?;
    
    let csv = generate_redemptions_csv(&data)?;
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"redemptions_{}_{}.csv\"", start_date, end_date))
        .body(Body::from(csv))
        .unwrap())
}

// ============================================================================
// Merchant Reports
// ============================================================================

/// Generar reporte (Merchant)
/// GET /api/v1/merchant/reports
pub async fn merchant_generate_report(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Query(params): Query<ReportQuery>,
) -> Result<Response, ApiError> {
    let merchant_id = merchant.get_merchant_id()
        .ok_or_else(|| ApiError::Unauthorized("ID de comercio no válido".to_string()))?;

    let format = params.format.as_deref().unwrap_or("json");
    let (start_date, end_date) = get_date_range(&params.start_date, &params.end_date);

    match params.report_type.as_str() {
        "redemptions" => {
            let data = get_redemptions_report(&state.db_pool, &start_date, &end_date, params.status.as_deref(), Some(merchant_id)).await?;
            format_response(data, format, "redemptions_report")
        }
        "offers" => {
            let data = get_offers_report(&state.db_pool, &start_date, &end_date, Some(merchant_id)).await?;
            format_response(data, format, "offers_report")
        }
        "summary" => {
            let data = get_merchant_summary(&state.db_pool, merchant_id, &start_date, &end_date).await?;
            Ok(Json(serde_json::json!({
                "success": true,
                "merchant_name": merchant.merchant_name,
                "report": data
            })).into_response())
        }
        _ => Err(ApiError::BadRequest("Tipo de reporte no válido".to_string()))
    }
}

/// Exportar redenciones a CSV (Merchant)
/// GET /api/v1/merchant/export/redemptions
pub async fn merchant_export_redemptions(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Query(params): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    let merchant_id = merchant.get_merchant_id()
        .ok_or_else(|| ApiError::Unauthorized("ID de comercio no válido".to_string()))?;

    let (start_date, end_date) = get_date_range(&params.start_date, &params.end_date);
    let data = get_redemptions_report(&state.db_pool, &start_date, &end_date, params.status.as_deref(), Some(merchant_id)).await?;
    
    let csv = generate_redemptions_csv(&data)?;
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}_redemptions_{}_{}.csv\"", merchant.merchant_name.replace(" ", "_"), start_date, end_date))
        .body(Body::from(csv))
        .unwrap())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn is_admin(user_id: i32) -> bool {
    let admin_ids: Vec<i32> = std::env::var("ADMIN_USER_IDS")
        .unwrap_or_else(|_| "1,2,3".to_string())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    admin_ids.contains(&user_id)
}

fn get_date_range(start: &Option<String>, end: &Option<String>) -> (String, String) {
    use chrono::{Duration, Utc};
    
    let end_date = end.clone().unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let start_date = start.clone().unwrap_or_else(|| {
        (Utc::now() - Duration::days(30)).format("%Y-%m-%d").to_string()
    });
    
    (start_date, end_date)
}

async fn get_redemptions_report(
    pool: &sqlx::PgPool,
    start_date: &str,
    end_date: &str,
    status: Option<&str>,
    merchant_id: Option<uuid::Uuid>,
) -> Result<Vec<RedemptionReportItem>, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        redemption_id: String,
        redemption_code: String,
        user_id: i32,
        offer_name: String,
        merchant_name: String,
        lumis_spent: i32,
        status: String,
        created_at: String,
        validated_at: Option<String>,
    }

    let mut query = String::from(
        r#"
        SELECT 
            ur.redemption_id::text,
            ur.redemption_code,
            ur.user_id,
            COALESCE(ro.name_friendly, ro.name) as offer_name,
            COALESCE(ro.merchant_name, 'N/A') as merchant_name,
            ur.lumis_spent,
            ur.redemption_status as status,
            ur.created_at::text,
            ur.validated_at::text
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ur.created_at >= $1::date
          AND ur.created_at <= $2::date + INTERVAL '1 day'
        "#
    );

    if status.is_some() {
        query.push_str(" AND ur.redemption_status = $3");
    }
    if merchant_id.is_some() {
        let param = if status.is_some() { "$4" } else { "$3" };
        query.push_str(&format!(" AND ro.merchant_id = {}", param));
    }
    
    query.push_str(" ORDER BY ur.created_at DESC");

    let rows: Vec<Row> = if let Some(status) = status {
        if let Some(mid) = merchant_id {
            sqlx::query_as(&query)
                .bind(start_date)
                .bind(end_date)
                .bind(status)
                .bind(mid)
                .fetch_all(pool)
                .await
        } else {
            sqlx::query_as(&query)
                .bind(start_date)
                .bind(end_date)
                .bind(status)
                .fetch_all(pool)
                .await
        }
    } else {
        if let Some(mid) = merchant_id {
            sqlx::query_as(&query)
                .bind(start_date)
                .bind(end_date)
                .bind(mid)
                .fetch_all(pool)
                .await
        } else {
            sqlx::query_as(&query)
                .bind(start_date)
                .bind(end_date)
                .fetch_all(pool)
                .await
        }
    }.map_err(|e| {
        error!("Failed to get redemptions report: {}", e);
        ApiError::InternalError("Error al generar reporte".to_string())
    })?;

    Ok(rows.into_iter().map(|r| RedemptionReportItem {
        redemption_id: r.redemption_id,
        redemption_code: r.redemption_code,
        user_id: r.user_id,
        offer_name: r.offer_name,
        merchant_name: r.merchant_name,
        lumis_spent: r.lumis_spent,
        status: r.status,
        created_at: r.created_at,
        validated_at: r.validated_at,
    }).collect())
}

async fn get_offers_report(
    pool: &sqlx::PgPool,
    start_date: &str,
    end_date: &str,
    merchant_id: Option<uuid::Uuid>,
) -> Result<Vec<OfferReportItem>, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        offer_id: String,
        name: String,
        merchant_name: String,
        lumis_cost: i32,
        stock_quantity: Option<i32>,
        is_active: bool,
        total_redemptions: i64,
        confirmed_redemptions: i64,
        revenue_lumis: i64,
    }

    let query = if merchant_id.is_some() {
        r#"
        SELECT 
            ro.offer_id::text,
            COALESCE(ro.name_friendly, ro.name) as name,
            COALESCE(ro.merchant_name, 'N/A') as merchant_name,
            COALESCE(ro.lumis_cost, ro.points) as lumis_cost,
            ro.stock_quantity,
            ro.is_active,
            COUNT(ur.redemption_id)::bigint as total_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed')::bigint as confirmed_redemptions,
            COALESCE(SUM(ur.lumis_spent) FILTER (WHERE ur.redemption_status = 'confirmed'), 0)::bigint as revenue_lumis
        FROM rewards.redemption_offers ro
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
            AND ur.created_at >= $1::date
            AND ur.created_at <= $2::date + INTERVAL '1 day'
        WHERE ro.merchant_id = $3
        GROUP BY ro.offer_id, ro.name_friendly, ro.name, ro.merchant_name, ro.lumis_cost, ro.points, ro.stock_quantity, ro.is_active
        ORDER BY total_redemptions DESC
        "#
    } else {
        r#"
        SELECT 
            ro.offer_id::text,
            COALESCE(ro.name_friendly, ro.name) as name,
            COALESCE(ro.merchant_name, 'N/A') as merchant_name,
            COALESCE(ro.lumis_cost, ro.points) as lumis_cost,
            ro.stock_quantity,
            ro.is_active,
            COUNT(ur.redemption_id)::bigint as total_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed')::bigint as confirmed_redemptions,
            COALESCE(SUM(ur.lumis_spent) FILTER (WHERE ur.redemption_status = 'confirmed'), 0)::bigint as revenue_lumis
        FROM rewards.redemption_offers ro
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
            AND ur.created_at >= $1::date
            AND ur.created_at <= $2::date + INTERVAL '1 day'
        GROUP BY ro.offer_id, ro.name_friendly, ro.name, ro.merchant_name, ro.lumis_cost, ro.points, ro.stock_quantity, ro.is_active
        ORDER BY total_redemptions DESC
        "#
    };

    let rows: Vec<Row> = if let Some(mid) = merchant_id {
        sqlx::query_as(query)
            .bind(start_date)
            .bind(end_date)
            .bind(mid)
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as(query)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(pool)
            .await
    }.map_err(|e| {
        error!("Failed to get offers report: {}", e);
        ApiError::InternalError("Error al generar reporte".to_string())
    })?;

    Ok(rows.into_iter().map(|r| OfferReportItem {
        offer_id: r.offer_id,
        name: r.name,
        merchant_name: r.merchant_name,
        lumis_cost: r.lumis_cost,
        stock_quantity: r.stock_quantity,
        is_active: r.is_active,
        total_redemptions: r.total_redemptions,
        confirmed_redemptions: r.confirmed_redemptions,
        revenue_lumis: r.revenue_lumis,
    }).collect())
}

async fn get_merchants_report(
    pool: &sqlx::PgPool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<MerchantReportItem>, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        merchant_id: String,
        merchant_name: String,
        is_active: bool,
        total_offers: i64,
        active_offers: i64,
        total_redemptions: i64,
        confirmed_redemptions: i64,
        total_lumis: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT 
            m.merchant_id::text,
            m.merchant_name,
            COALESCE(m.is_active, true) as is_active,
            COUNT(DISTINCT ro.offer_id)::bigint as total_offers,
            COUNT(DISTINCT ro.offer_id) FILTER (WHERE ro.is_active = true)::bigint as active_offers,
            COUNT(ur.redemption_id)::bigint as total_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed')::bigint as confirmed_redemptions,
            COALESCE(SUM(ur.lumis_spent) FILTER (WHERE ur.redemption_status = 'confirmed'), 0)::bigint as total_lumis
        FROM rewards.merchants m
        LEFT JOIN rewards.redemption_offers ro ON m.merchant_id = ro.merchant_id
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
            AND ur.created_at >= $1::date
            AND ur.created_at <= $2::date + INTERVAL '1 day'
        GROUP BY m.merchant_id, m.merchant_name, m.is_active
        ORDER BY total_lumis DESC
        "#
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!("Failed to get merchants report: {}", e);
        ApiError::InternalError("Error al generar reporte".to_string())
    })?;

    Ok(rows.into_iter().map(|r| MerchantReportItem {
        merchant_id: r.merchant_id,
        merchant_name: r.merchant_name,
        is_active: r.is_active,
        total_offers: r.total_offers,
        active_offers: r.active_offers,
        total_redemptions: r.total_redemptions,
        confirmed_redemptions: r.confirmed_redemptions,
        total_lumis: r.total_lumis,
    }).collect())
}

async fn get_summary_report(
    pool: &sqlx::PgPool,
    start_date: &str,
    end_date: &str,
) -> Result<SummaryReport, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        total_redemptions: Option<i64>,
        confirmed_redemptions: Option<i64>,
        pending_redemptions: Option<i64>,
        cancelled_redemptions: Option<i64>,
        expired_redemptions: Option<i64>,
        total_lumis_redeemed: Option<i64>,
    }

    let stats: Row = sqlx::query_as(
        r#"
        SELECT 
            COUNT(*)::bigint as total_redemptions,
            COUNT(*) FILTER (WHERE redemption_status = 'confirmed')::bigint as confirmed_redemptions,
            COUNT(*) FILTER (WHERE redemption_status = 'pending')::bigint as pending_redemptions,
            COUNT(*) FILTER (WHERE redemption_status = 'cancelled')::bigint as cancelled_redemptions,
            COUNT(*) FILTER (WHERE redemption_status = 'expired')::bigint as expired_redemptions,
            COALESCE(SUM(lumis_spent) FILTER (WHERE redemption_status = 'confirmed'), 0)::bigint as total_lumis_redeemed
        FROM rewards.user_redemptions
        WHERE created_at >= $1::date
          AND created_at <= $2::date + INTERVAL '1 day'
        "#
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!("Failed to get summary: {}", e);
        ApiError::InternalError("Error al generar resumen".to_string())
    })?;

    let offers: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, COUNT(*) FILTER (WHERE is_active = true)::bigint FROM rewards.redemption_offers"
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0, 0));

    let merchants: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM rewards.merchants"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let total = stats.total_redemptions.unwrap_or(0);
    let confirmed = stats.confirmed_redemptions.unwrap_or(0);
    let lumis = stats.total_lumis_redeemed.unwrap_or(0);

    Ok(SummaryReport {
        period: format!("{} to {}", start_date, end_date),
        total_redemptions: total,
        confirmed_redemptions: confirmed,
        pending_redemptions: stats.pending_redemptions.unwrap_or(0),
        cancelled_redemptions: stats.cancelled_redemptions.unwrap_or(0),
        expired_redemptions: stats.expired_redemptions.unwrap_or(0),
        total_lumis_redeemed: lumis,
        total_offers: offers.0,
        active_offers: offers.1,
        total_merchants: merchants,
        conversion_rate: if total > 0 { (confirmed as f64 / total as f64) * 100.0 } else { 0.0 },
        avg_lumis_per_redemption: if confirmed > 0 { lumis as f64 / confirmed as f64 } else { 0.0 },
    })
}

async fn get_merchant_summary(
    pool: &sqlx::PgPool,
    merchant_id: uuid::Uuid,
    start_date: &str,
    end_date: &str,
) -> Result<SummaryReport, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        total_redemptions: Option<i64>,
        confirmed_redemptions: Option<i64>,
        pending_redemptions: Option<i64>,
        cancelled_redemptions: Option<i64>,
        expired_redemptions: Option<i64>,
        total_lumis_redeemed: Option<i64>,
        total_offers: Option<i64>,
        active_offers: Option<i64>,
    }

    let stats: Row = sqlx::query_as(
        r#"
        SELECT 
            COUNT(ur.redemption_id)::bigint as total_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed')::bigint as confirmed_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'pending')::bigint as pending_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'cancelled')::bigint as cancelled_redemptions,
            COUNT(ur.redemption_id) FILTER (WHERE ur.redemption_status = 'expired')::bigint as expired_redemptions,
            COALESCE(SUM(ur.lumis_spent) FILTER (WHERE ur.redemption_status = 'confirmed'), 0)::bigint as total_lumis_redeemed,
            COUNT(DISTINCT ro.offer_id)::bigint as total_offers,
            COUNT(DISTINCT ro.offer_id) FILTER (WHERE ro.is_active = true)::bigint as active_offers
        FROM rewards.redemption_offers ro
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
            AND ur.created_at >= $2::date
            AND ur.created_at <= $3::date + INTERVAL '1 day'
        WHERE ro.merchant_id = $1
        "#
    )
    .bind(merchant_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!("Failed to get merchant summary: {}", e);
        ApiError::InternalError("Error al generar resumen".to_string())
    })?;

    let total = stats.total_redemptions.unwrap_or(0);
    let confirmed = stats.confirmed_redemptions.unwrap_or(0);
    let lumis = stats.total_lumis_redeemed.unwrap_or(0);

    Ok(SummaryReport {
        period: format!("{} to {}", start_date, end_date),
        total_redemptions: total,
        confirmed_redemptions: confirmed,
        pending_redemptions: stats.pending_redemptions.unwrap_or(0),
        cancelled_redemptions: stats.cancelled_redemptions.unwrap_or(0),
        expired_redemptions: stats.expired_redemptions.unwrap_or(0),
        total_lumis_redeemed: lumis,
        total_offers: stats.total_offers.unwrap_or(0),
        active_offers: stats.active_offers.unwrap_or(0),
        total_merchants: 1,
        conversion_rate: if total > 0 { (confirmed as f64 / total as f64) * 100.0 } else { 0.0 },
        avg_lumis_per_redemption: if confirmed > 0 { lumis as f64 / confirmed as f64 } else { 0.0 },
    })
}

fn format_response<T: Serialize>(data: Vec<T>, format: &str, filename: &str) -> Result<Response, ApiError> {
    match format {
        "csv" => {
            let csv = generate_csv(&data)?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
                .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.csv\"", filename))
                .body(Body::from(csv))
                .unwrap())
        }
        _ => {
            Ok(Json(serde_json::json!({
                "success": true,
                "data": data,
                "count": data.len()
            })).into_response())
        }
    }
}

fn generate_csv<T: Serialize>(data: &[T]) -> Result<String, ApiError> {
    if data.is_empty() {
        return Ok(String::new());
    }
    
    // Convertir a JSON para obtener las keys
    let json: Vec<serde_json::Value> = data.iter()
        .map(|item| serde_json::to_value(item).unwrap_or_default())
        .collect();
    
    if let Some(first) = json.first() {
        if let serde_json::Value::Object(map) = first {
            let headers: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
            let mut csv = headers.join(",") + "\n";
            
            for row in &json {
                if let serde_json::Value::Object(m) = row {
                    let values: Vec<String> = headers.iter()
                        .map(|h| {
                            m.get(*h)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => format!("\"{}\"", s.replace("\"", "\"\"")),
                                    serde_json::Value::Null => String::new(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default()
                        })
                        .collect();
                    csv.push_str(&values.join(","));
                    csv.push('\n');
                }
            }
            
            return Ok(csv);
        }
    }
    
    Ok(String::new())
}

fn generate_redemptions_csv(data: &[RedemptionReportItem]) -> Result<String, ApiError> {
    let mut csv = "ID,Codigo,Usuario,Oferta,Comercio,Lumis,Estado,Creado,Validado\n".to_string();
    
    for item in data {
        csv.push_str(&format!(
            "{},{},{},\"{}\",\"{}\",{},{},{},{}\n",
            item.redemption_id,
            item.redemption_code,
            item.user_id,
            item.offer_name.replace("\"", "\"\""),
            item.merchant_name.replace("\"", "\"\""),
            item.lumis_spent,
            item.status,
            item.created_at,
            item.validated_at.as_deref().unwrap_or("")
        ));
    }
    
    Ok(csv)
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        
        let body = Json(serde_json::json!({
            "success": false,
            "error": message,
        }));
        
        (status, body).into_response()
    }
}

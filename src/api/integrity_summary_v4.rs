use axum::{
    extract::{Extension, State},
    http::StatusCode,
    middleware::from_fn,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::api::common::ApiResponse;
use crate::middleware::auth::{extract_current_user, CurrentUser};
use crate::state::AppState;

/// Summary de integridad por recurso
#[derive(Debug, Serialize)]
pub struct ResourceIntegritySummary {
    pub total_count: i64,
    pub global_hash: i64,
    pub last_update: Option<chrono::NaiveDateTime>,
    pub snapshot_time: chrono::DateTime<chrono::Utc>,
}

/// Response completo con todos los recursos
#[derive(Debug, Serialize)]
pub struct IntegritySummaryResponse {
    pub products: ResourceIntegritySummary,
    pub issuers: ResourceIntegritySummary,
    pub headers: ResourceIntegritySummary,
    pub details: ResourceIntegritySummary,
}

/// Create router for integrity summary
pub fn create_integrity_summary_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/integrity-summary", get(get_integrity_summary))
        .layer(from_fn(extract_current_user))
}

/// GET /api/v4/invoices/integrity-summary
/// 
/// Endpoint ligero para validación de integridad global (1 vez al día)
/// Lee las Materialized Views actualizadas cada noche a las 3 AM UTC
pub async fn get_integrity_summary(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<IntegritySummaryResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let user_id = current_user.user_id;
    
    info!("🔍 Fetching integrity summary for user_id: {} [{}]", user_id, request_id);

    // Query products integrity (< 5ms - index scan)
    let products = sqlx::query_as::<_, (i64, i64, Option<chrono::NaiveDateTime>, chrono::DateTime<chrono::Utc>)>(
        "SELECT total_count, global_hash, last_update, snapshot_time 
         FROM user_product_integrity_daily 
         WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to fetch products integrity: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .unwrap_or((0, 0, None, chrono::Utc::now()));

    // Query issuers integrity
    let issuers = sqlx::query_as::<_, (i64, i64, Option<chrono::NaiveDateTime>, chrono::DateTime<chrono::Utc>)>(
        "SELECT total_count, global_hash, last_update, snapshot_time 
         FROM user_issuer_integrity_daily 
         WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to fetch issuers integrity: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .unwrap_or((0, 0, None, chrono::Utc::now()));

    // Query headers integrity
    let headers = sqlx::query_as::<_, (i64, i64, Option<chrono::NaiveDateTime>, chrono::DateTime<chrono::Utc>)>(
        "SELECT total_count, global_hash, last_update, snapshot_time 
         FROM user_header_integrity_daily 
         WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to fetch headers integrity: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .unwrap_or((0, 0, None, chrono::Utc::now()));

    // Query details integrity
    let details = sqlx::query_as::<_, (i64, i64, Option<chrono::NaiveDateTime>, chrono::DateTime<chrono::Utc>)>(
        "SELECT total_count, global_hash, last_update, snapshot_time 
         FROM user_detail_integrity_daily 
         WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to fetch details integrity: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .unwrap_or((0, 0, None, chrono::Utc::now()));

    let response = IntegritySummaryResponse {
        products: ResourceIntegritySummary {
            total_count: products.0,
            global_hash: products.1,
            last_update: products.2,
            snapshot_time: products.3,
        },
        issuers: ResourceIntegritySummary {
            total_count: issuers.0,
            global_hash: issuers.1,
            last_update: issuers.2,
            snapshot_time: issuers.3,
        },
        headers: ResourceIntegritySummary {
            total_count: headers.0,
            global_hash: headers.1,
            last_update: headers.2,
            snapshot_time: headers.3,
        },
        details: ResourceIntegritySummary {
            total_count: details.0,
            global_hash: details.1,
            last_update: details.2,
            snapshot_time: details.3,
        },
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        "✅ Integrity summary fetched for user {} - products: {}, issuers: {}, headers: {}, details: {} in {}ms [{}]",
        user_id,
        response.products.total_count,
        response.issuers.total_count,
        response.headers.total_count,
        response.details.total_count,
        execution_time,
        request_id
    );

    Ok(Json(ApiResponse::success(
        response,
        request_id,
        Some(execution_time),
        false,
    )))
}

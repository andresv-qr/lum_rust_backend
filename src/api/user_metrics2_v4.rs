use axum::{extract::{Query, State, Extension}, http::StatusCode, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info, warn};
use chrono::{DateTime, Utc};

use axum::middleware::from_fn;
use crate::{state::AppState, middleware::{CurrentUser, extract_current_user}};

#[derive(Debug, Deserialize)]
pub struct UserMetrics2Params {
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInvoiceSummaryMetrics {
    pub user_id: i64,
    pub total_facturas: i64,
    pub total_monto: Option<f64>,
    pub total_items: i64,
    pub n_descuentos: i64,
    pub total_descuento: Option<f64>,
    pub top_emisores: Value,
    pub top_categorias: Value,
    pub serie_mensual: Value,
    pub updated_at: DateTime<Utc>,
    pub comparativo_categoria: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMetrics2Response {
    pub data: UserInvoiceSummaryMetrics,
}

const METRICS2_CACHE_TTL_SECONDS: usize = crate::cache_ttl::TTL_METRICS as usize; // reuse metrics TTL

pub async fn get_user_invoice_summary_metrics(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserMetrics2Params>,
) -> Result<Json<UserMetrics2Response>, StatusCode> {
    if params.user_id <= 0 { return Err(StatusCode::BAD_REQUEST); }

    if current_user.user_id != params.user_id { return Err(StatusCode::FORBIDDEN); }

    // Redis cache lookup
    let cache_key = crate::cache_key::metrics_user_id(params.user_id);
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(cached_json)) = redis::cmd("GET").arg(&cache_key).query_async::<Option<String>>(&mut conn).await {
            match serde_json::from_str::<UserMetrics2Response>(&cached_json) {
                Ok(resp) => {
                    info!("metrics2 cache HIT for user_id {}", params.user_id);
                    return Ok(Json(resp));
                },
                Err(e) => {
                    warn!("Failed to deserialize cached metrics2 for user_id {}: {}", params.user_id, e);
                }
            }
        } else {
            info!("metrics2 cache MISS for user_id {}", params.user_id);
        }
    } else {
        warn!("metrics2 cache skipped (Redis connection unavailable)");
    }

    // NOTE: We assume json/jsonb columns for the *_arrays; we'll map them directly to serde_json::Value
    // Empty columns (NULL) will become Value::Null automatically via Option unwrap_or(Value::Null)
    let query = r#"SELECT user_id, total_facturas, total_monto, total_items, n_descuentos, total_descuento,
                          top_emisores, top_categorias, serie_mensual, updated_at, comparativo_categoria
                   FROM rewards.user_invoice_summary
                   WHERE user_id = $1
                   LIMIT 1"#;

    let row_opt = sqlx::query(query)
        .bind(params.user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| {
            error!("DB error querying user_invoice_summary for user_id {}: {}", params.user_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let row = match row_opt { Some(r) => r, None => return Err(StatusCode::NOT_FOUND) };

    // Extract with graceful fallbacks
    let metrics = UserInvoiceSummaryMetrics {
        user_id: row.get::<i64, _>("user_id"),
        total_facturas: row.try_get::<i64, _>("total_facturas").unwrap_or(0),
        total_monto: row.try_get::<Option<f64>, _>("total_monto").unwrap_or(None),
        total_items: row.try_get::<i64, _>("total_items").unwrap_or(0),
        n_descuentos: row.try_get::<i64, _>("n_descuentos").unwrap_or(0),
        total_descuento: row.try_get::<Option<f64>, _>("total_descuento").unwrap_or(None),
        top_emisores: row.try_get::<Value, _>("top_emisores").unwrap_or(Value::Null),
        top_categorias: row.try_get::<Value, _>("top_categorias").unwrap_or(Value::Null),
        serie_mensual: row.try_get::<Value, _>("serie_mensual").unwrap_or(Value::Null),
        updated_at: row.get::<DateTime<Utc>, _>("updated_at"),
        comparativo_categoria: row.try_get::<Value, _>("comparativo_categoria").unwrap_or(Value::Null),
    };

    info!("Retrieved invoice summary metrics (metrics2) for user_id {}", metrics.user_id);

    let response = UserMetrics2Response { data: metrics };

    // Store in cache (best effort)
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(serialized) = serde_json::to_string(&response) {
            if let Err(e) = redis::cmd("SETEX")
                .arg(&cache_key)
                .arg(METRICS2_CACHE_TTL_SECONDS)
                .arg(serialized)
                .query_async::<()>(&mut conn).await {
                warn!("Failed to cache metrics2 for user_id {}: {}", params.user_id, e);
            }
        }
    }

    Ok(Json(response))
}

pub fn create_user_metrics2_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/users/metrics2", get(get_user_invoice_summary_metrics))
        .route_layer(from_fn(extract_current_user))
}

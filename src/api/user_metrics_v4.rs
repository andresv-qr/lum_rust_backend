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
pub struct UserMetricsParams {
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMetrics {
    pub user_id: i64,
    pub total_invoices: i64,
    pub total_amount: Option<f64>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserMetricsResponse {
    pub data: UserMetrics,
}

const METRICS_CACHE_TTL_SECONDS: usize = crate::cache_ttl::TTL_METRICS as usize;

pub async fn get_user_metrics(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserMetricsParams>,
) -> Result<Json<UserMetricsResponse>, StatusCode> {
    if params.user_id <= 0 { 
        return Err(StatusCode::BAD_REQUEST); 
    }

    if current_user.user_id != params.user_id { 
        return Err(StatusCode::FORBIDDEN); 
    }

    // Redis cache lookup
    let cache_key = crate::cache_key::metrics_user_id(params.user_id);
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(cached_json)) = redis::cmd("GET").arg(&cache_key).query_async::<_, Option<String>>(&mut conn).await {
            match serde_json::from_str::<UserMetricsResponse>(&cached_json) {
                Ok(resp) => {
                    info!("metrics cache HIT for user_id {}", params.user_id);
                    return Ok(Json(resp));
                },
                Err(e) => {
                    warn!("Failed to deserialize cached metrics for user_id {}: {}", params.user_id, e);
                }
            }
        } else {
            info!("metrics cache MISS for user_id {}", params.user_id);
        }
    } else {
        warn!("metrics cache skipped (Redis connection unavailable)");
    }

    // Query basic user metrics
    let query = r#"SELECT user_id, 
                          COALESCE(total_facturas, 0) as total_invoices,
                          total_monto as total_amount,
                          updated_at
                   FROM rewards.user_invoice_summary
                   WHERE user_id = $1
                   LIMIT 1"#;

    let row_opt = sqlx::query(query)
        .bind(params.user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| {
            error!("DB error querying user metrics for user_id {}: {}", params.user_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let row = match row_opt { 
        Some(r) => r, 
        None => return Err(StatusCode::NOT_FOUND) 
    };

    let metrics = UserMetrics {
        user_id: row.get::<i64, _>("user_id"),
        total_invoices: row.try_get::<i64, _>("total_invoices").unwrap_or(0),
        total_amount: row.try_get::<Option<f64>, _>("total_amount").unwrap_or(None),
        updated_at: row.get::<DateTime<Utc>, _>("updated_at"),
    };

    info!("Retrieved user metrics for user_id {}", metrics.user_id);

    let response = UserMetricsResponse { data: metrics };

    // Store in cache (best effort)
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(serialized) = serde_json::to_string(&response) {
            if let Err(e) = redis::cmd("SETEX")
                .arg(&cache_key)
                .arg(METRICS_CACHE_TTL_SECONDS)
                .arg(serialized)
                .query_async::<_, ()>(&mut conn).await {
                warn!("Failed to cache metrics for user_id {}: {}", params.user_id, e);
            }
        }
    }

    Ok(Json(response))
}

pub fn create_user_metrics_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/users/metrics", get(get_user_metrics))
        .route_layer(from_fn(extract_current_user))
}

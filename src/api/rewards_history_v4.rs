use axum::{
    extract::{State, Extension, Query},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::Row;

use axum::middleware::from_fn;
use crate::{
    state::AppState,
    middleware::{CurrentUser, extract_current_user},
    api::common::ApiResponse,
};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RewardsHistoryItem {
    pub source_type: String,
    pub user_id: i32,
    pub name_friendly: Option<String>,
    pub description_friendly: Option<String>,
    pub quantity: Option<sqlx::types::Decimal>, // Changed to Decimal to match NUMERIC
    pub date: Option<DateTime<Utc>>, // Changed to DateTime<Utc> to match TIMESTAMPTZ
    pub img: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RewardsHistoryParams {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub source_type_filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RewardsHistoryResponse {
    pub items: Vec<RewardsHistoryItem>,
    pub pagination: PaginationInfo,
    pub summary: HistorySummary,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
    pub has_next: bool,
    pub has_previous: bool,
}

#[derive(Debug, Serialize)]
pub struct HistorySummary {
    pub total_items: i64,
    pub total_acumulaciones: i64,
    pub total_redenciones: i64,
    pub sum_quantity: Option<sqlx::types::Decimal>, // Changed to Decimal to match NUMERIC
}

/// GET /api/v4/rewards/history - Obtener historial de acumulaciones y redenciones
pub async fn get_rewards_history(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<RewardsHistoryParams>,
) -> Result<Json<ApiResponse<RewardsHistoryResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    info!("Fetching rewards history for user_id: {}", current_user.user_id);

    // Parámetros con valores por defecto
    let limit = params.limit.unwrap_or(50).min(500).max(1); // Max 500, min 1
    let offset = params.offset.unwrap_or(0).max(0); // Min 0

    // Construcción dinámica de la query con filtros
    let mut where_conditions = vec!["user_id = $1".to_string()];
    let mut param_count = 1;

    // Filtro por fecha desde
    if params.date_from.is_some() {
        param_count += 1;
        where_conditions.push(format!("date >= ${}", param_count));
    }

    // Filtro por fecha hasta
    if params.date_to.is_some() {
        param_count += 1;
        where_conditions.push(format!("date <= ${}", param_count));
    }

    // Filtro por tipo de fuente
    if params.source_type_filter.is_some() {
        param_count += 1;
        where_conditions.push(format!("source_type ILIKE ${}", param_count));
    }

    let where_clause = where_conditions.join(" AND ");

    // Query principal para obtener los items
    let items_query = format!(r#"
        SELECT 
            'Acumulación' as source_type,
            user_id,
            name_friendly,
            description_friendly,
            quantity,
            date,
            img
        FROM rewards.vw_hist_accum_redem
        WHERE {}
        ORDER BY date DESC, user_id DESC
        LIMIT {} OFFSET {}
    "#, where_clause, limit, offset);

    // Query para contar el total
    let count_query = format!(r#"
        SELECT COUNT(*) as total
        FROM rewards.vw_hist_accum_redem
        WHERE {}
    "#, where_clause);

    // Query para estadísticas de resumen
    let summary_query = format!(r#"
        SELECT 
            COUNT(*) as total_items,
            COUNT(CASE WHEN source_type = 'Acumulación' THEN 1 END) as total_acumulaciones,
            COUNT(CASE WHEN source_type = 'Redención' THEN 1 END) as total_redenciones,
            COALESCE(SUM(quantity), 0) as sum_quantity
        FROM rewards.vw_hist_accum_redem
        WHERE {}
    "#, where_clause);

    // Construir query con parámetros
    let items_query_builder = sqlx::query_as::<_, RewardsHistoryItem>(&items_query)
        .bind(current_user.user_id as i32);
    
    let count_query_builder = sqlx::query_scalar::<_, i64>(&count_query)
        .bind(current_user.user_id as i32);
    
    let summary_query_builder = sqlx::query(&summary_query)
        .bind(current_user.user_id as i32);

    // Ejecutar queries en paralelo
    let (items_result, total_result, summary_result) = if let Some(source_type_filter) = &params.source_type_filter {
        let filter_pattern = format!("%{}%", source_type_filter);
        
        let items_query_builder = if params.date_from.is_some() && params.date_to.is_some() {
            items_query_builder.bind(&params.date_from).bind(&params.date_to).bind(&filter_pattern)
        } else if params.date_from.is_some() {
            items_query_builder.bind(&params.date_from).bind(&filter_pattern)
        } else if params.date_to.is_some() {
            items_query_builder.bind(&params.date_to).bind(&filter_pattern)
        } else {
            items_query_builder.bind(&filter_pattern)
        };

        let count_query_builder = if params.date_from.is_some() && params.date_to.is_some() {
            count_query_builder.bind(&params.date_from).bind(&params.date_to).bind(&filter_pattern)
        } else if params.date_from.is_some() {
            count_query_builder.bind(&params.date_from).bind(&filter_pattern)
        } else if params.date_to.is_some() {
            count_query_builder.bind(&params.date_to).bind(&filter_pattern)
        } else {
            count_query_builder.bind(&filter_pattern)
        };

        let summary_query_builder = if params.date_from.is_some() && params.date_to.is_some() {
            summary_query_builder.bind(&params.date_from).bind(&params.date_to).bind(&filter_pattern)
        } else if params.date_from.is_some() {
            summary_query_builder.bind(&params.date_from).bind(&filter_pattern)
        } else if params.date_to.is_some() {
            summary_query_builder.bind(&params.date_to).bind(&filter_pattern)
        } else {
            summary_query_builder.bind(&filter_pattern)
        };

        tokio::join!(
            items_query_builder.fetch_all(&state.db_pool),
            count_query_builder.fetch_one(&state.db_pool),
            summary_query_builder.fetch_one(&state.db_pool)
        )
    } else {
        let items_query_builder = if params.date_from.is_some() && params.date_to.is_some() {
            items_query_builder.bind(&params.date_from).bind(&params.date_to)
        } else if params.date_from.is_some() {
            items_query_builder.bind(&params.date_from)
        } else if params.date_to.is_some() {
            items_query_builder.bind(&params.date_to)
        } else {
            items_query_builder
        };

        let count_query_builder = if params.date_from.is_some() && params.date_to.is_some() {
            count_query_builder.bind(&params.date_from).bind(&params.date_to)
        } else if params.date_from.is_some() {
            count_query_builder.bind(&params.date_from)
        } else if params.date_to.is_some() {
            count_query_builder.bind(&params.date_to)
        } else {
            count_query_builder
        };

        let summary_query_builder = if params.date_from.is_some() && params.date_to.is_some() {
            summary_query_builder.bind(&params.date_from).bind(&params.date_to)
        } else if params.date_from.is_some() {
            summary_query_builder.bind(&params.date_from)
        } else if params.date_to.is_some() {
            summary_query_builder.bind(&params.date_to)
        } else {
            summary_query_builder
        };

        tokio::join!(
            items_query_builder.fetch_all(&state.db_pool),
            count_query_builder.fetch_one(&state.db_pool),
            summary_query_builder.fetch_one(&state.db_pool)
        )
    };

    match (items_result, total_result, summary_result) {
        (Ok(items), Ok(total), Ok(summary_row)) => {
            let total_acumulaciones: i64 = summary_row.try_get("total_acumulaciones").unwrap_or(0);
            let total_redenciones: i64 = summary_row.try_get("total_redenciones").unwrap_or(0);
            let total_items: i64 = summary_row.try_get("total_items").unwrap_or(0);
            let sum_quantity_raw: Option<sqlx::types::Decimal> = summary_row.try_get("sum_quantity").ok();

            let pagination = PaginationInfo {
                total,
                limit,
                offset,
                has_next: offset + limit < total as i32,
                has_previous: offset > 0,
            };

            let summary = HistorySummary {
                total_items,
                total_acumulaciones,
                total_redenciones,
                sum_quantity: sum_quantity_raw,
            };

            let response_data = RewardsHistoryResponse {
                items,
                pagination,
                summary,
            };

            let elapsed = start_time.elapsed();
            info!(
                "Rewards history retrieved in {:?}ms for user_id: {} - {} items", 
                elapsed.as_millis(), 
                current_user.user_id,
                response_data.items.len()
            );

            let response = ApiResponse {
                success: true,
                data: Some(response_data),
                error: None,
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: Some(elapsed.as_millis() as u64),
                cached: false,
            };

            Ok(Json(response))
        },
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
            error!("Database error fetching rewards history for user_id {}: {}", current_user.user_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Router para rewards history v4
pub fn create_rewards_history_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/rewards/history", get(get_rewards_history))
        .route_layer(from_fn(extract_current_user))
}

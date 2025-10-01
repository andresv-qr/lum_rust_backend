use axum::{
    extract::{State, Extension, Query},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use tracing::{info, error};
use uuid::Uuid;
use sqlx::Row;

use axum::middleware::from_fn;
use crate::{
    state::AppState,
    middleware::{CurrentUser, extract_current_user},
    api::common::ApiResponse,
    api::templates::user_products_templates::{
        UserProductsQueryTemplates, 
        UserProductsResponse, 
        UserProductsRequest,
        UserProductsPagedResponse,
        PaginationInfo
    },
};

/// Create user_products v4 router
pub fn create_user_products_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/invoices/products", get(get_user_products))
        .layer(from_fn(extract_current_user))
}

/// GET /api/v4/invoices/products - Get products that a user has purchased
pub async fn get_user_products(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserProductsRequest>,
) -> Result<Json<ApiResponse<UserProductsPagedResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    
    info!("🛒 Fetching user products for user_id: {} [{}]", current_user.user_id, request_id);

    // Parameters with default values following v4 standards
    let limit = params.limit.unwrap_or(20).min(100).max(1); // Max 100, min 1
    let offset = params.offset.unwrap_or(0).max(0); // Min 0

    let user_id = current_user.user_id;

    // Parse update_date_from if provided
    let update_date_filter = if let Some(date_str) = &params.update_date_from {
        // For text-based process_date column, we'll validate the format but pass as string
        match chrono::DateTime::parse_from_rfc3339(date_str) {
            Ok(_) => {
                info!("🗓️ Using update_date filter: {} [{}]", date_str, request_id);
                Some(date_str.clone())
            },
            Err(_) => {
                // Try other date formats
                if chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok() {
                    info!("🗓️ Using update_date filter (date only): {} [{}]", date_str, request_id);
                    Some(date_str.clone())
                } else {
                    error!("❌ Invalid date format '{}'. Use YYYY-MM-DD or ISO 8601 format [{}]", date_str, request_id);
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
    } else {
        None
    };

    // Cache key generation (include filter in cache key)
    let cache_key = format!(
        "{}_{}_{}_{}_{}", 
        UserProductsQueryTemplates::get_user_products_cache_key_prefix(),
        user_id,
        limit,
        offset,
        params.update_date_from.as_deref().unwrap_or("no_date_filter")
    );

    info!("🔍 Cache key: {} [{}]", cache_key, request_id);

    // Execute count query for pagination
    let count_result = if let Some(date_filter) = &update_date_filter {
        let count_query = UserProductsQueryTemplates::get_user_products_count_with_date_filter_query();
        match sqlx::query(count_query)
            .bind(user_id)
            .bind(date_filter)
            .fetch_one(&state.db_pool)
            .await
        {
            Ok(row) => {
                let total: i64 = row.try_get("total").unwrap_or(0);
                total
            },
            Err(e) => {
                error!("❌ Failed to execute count query with date filter: {} [{}]", e, request_id);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        let count_query = UserProductsQueryTemplates::get_user_products_count_query();
        match sqlx::query(count_query)
            .bind(user_id)
            .fetch_one(&state.db_pool)
            .await
        {
            Ok(row) => {
                let total: i64 = row.try_get("total").unwrap_or(0);
                total
            },
            Err(e) => {
                error!("❌ Failed to execute count query: {} [{}]", e, request_id);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    // Execute main query
    let products_result = if let Some(date_filter) = &update_date_filter {
        let main_query = UserProductsQueryTemplates::get_user_products_with_date_filter_query();
        match sqlx::query_as::<_, UserProductsResponse>(main_query)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .bind(date_filter)
            .fetch_all(&state.db_pool)
            .await
        {
            Ok(products) => products,
            Err(e) => {
                error!("❌ Failed to execute products query with date filter: {} [{}]", e, request_id);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        let main_query = UserProductsQueryTemplates::get_user_products_query();
        match sqlx::query_as::<_, UserProductsResponse>(main_query)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db_pool)
            .await
        {
            Ok(products) => products,
            Err(e) => {
                error!("❌ Failed to execute products query: {} [{}]", e, request_id);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    // Calculate pagination info
    let total_pages = if limit > 0 { (count_result + limit - 1) / limit } else { 1 };
    let current_page = if limit > 0 { (offset / limit) + 1 } else { 1 };
    let has_next = offset + limit < count_result;
    let has_previous = offset > 0;

    let pagination = PaginationInfo {
        total: count_result,
        limit,
        offset,
        has_next,
        has_previous,
        total_pages,
        current_page,
    };

    let response = UserProductsPagedResponse {
        products: products_result,
        pagination,
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        "✅ Successfully fetched {} products for user {} (total: {}, date_filter: {}) in {}ms [{}]", 
        response.products.len(),
        user_id,
        count_result,
        params.update_date_from.as_deref().unwrap_or("none"),
        execution_time,
        request_id
    );

    Ok(Json(ApiResponse::success(
        response, 
        request_id, 
        Some(execution_time), 
        false // not cached in this version
    )))
}

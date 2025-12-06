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

use axum::middleware::from_fn;
use crate::{
    state::AppState,
    middleware::{CurrentUser, extract_current_user},
    api::common::{
        ApiResponse,
        IncrementalSyncResponse,
        calculate_checksum,
        get_deleted_items_since,
        extract_max_update_date,
        extract_record_ids,
        validate_date_format,
    },
    api::templates::user_products_templates::{
        UserProductsQueryTemplates, 
        UserProductsResponse, 
        UserProductsRequest,
    },
};

/// Create user_products v4 router
pub fn create_user_products_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/products", get(get_user_products))
        .layer(from_fn(extract_current_user))
}

/// GET /api/v4/invoices/products - Get products that a user has purchased
/// Now with incremental sync support (Nivel 2)
pub async fn get_user_products(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserProductsRequest>,
) -> Result<Json<ApiResponse<IncrementalSyncResponse<UserProductsResponse>>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now().naive_utc();
    
    info!("🛒 Fetching user products for user_id: {} [{}]", current_user.user_id, request_id);

    // Parameters with default values following v4 standards
    let limit = params.limit.unwrap_or(20).min(100).max(1); // Max 100, min 1
    let offset = params.offset.unwrap_or(0).max(0); // Min 0
    let user_id = current_user.user_id;

    // Validate and parse update_date_from if provided
    let update_date_filter = if let Some(date_str) = &params.update_date_from {
        match validate_date_format(date_str) {
            Ok(validated) => {
                info!("🗓️ Using update_date filter: {} [{}]", validated, request_id);
                Some(validated)
            },
            Err(e) => {
                error!("❌ Invalid date format: {} [{}]", e, request_id);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    } else {
        None
    };

    // Execute main query
    let products = if let Some(date_filter) = &update_date_filter {
        let main_query = UserProductsQueryTemplates::get_user_products_with_date_filter_query();
        sqlx::query_as::<_, UserProductsResponse>(main_query)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .bind(date_filter)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                error!("❌ Failed to execute products query with date filter: {} [{}]", e, request_id);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        let main_query = UserProductsQueryTemplates::get_user_products_query();
        sqlx::query_as::<_, UserProductsResponse>(main_query)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                error!("❌ Failed to execute products query: {} [{}]", e, request_id);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    // Get total count (for full dataset, not filtered by date)
    let total_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) as total
        FROM public.dim_product p
        JOIN (
            SELECT DISTINCT d.code, h.issuer_ruc
            FROM public.invoice_detail d
            JOIN public.invoice_header h ON d.cufe = h.cufe
            WHERE h.user_id = $1
        ) u ON p.code = u.code
           AND p.issuer_ruc = u.issuer_ruc
        WHERE p.is_deleted = FALSE
        "#
    )
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    // Get deleted items if update_date_from was provided
    let deleted_items = if let Some(since) = &update_date_filter {
        get_deleted_items_since(&state.db_pool, "dim_product", "code", since).await
    } else {
        vec![]
    };

    // Calculate checksum
    let data_checksum = calculate_checksum(&products)
        .map_err(|e| {
            error!("❌ Failed to calculate checksum: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Extract max_update_date and record_ids
    let max_update_date = extract_max_update_date(&products);
    let record_ids = extract_record_ids(&products);

    // Build pagination info
    let returned_records = products.len();
    let total_pages = if limit > 0 { (total_count + limit - 1) / limit } else { 1 };
    let current_page = if limit > 0 { (offset / limit) + 1 } else { 1 };
    let has_more = (offset + limit) < total_count;

    // Build IncrementalSyncResponse
    let response = IncrementalSyncResponse {
        data: products,
        pagination: crate::api::common::PaginationInfo {
            total_records: total_count,
            returned_records,
            limit,
            offset,
            has_more,
            total_pages,
            current_page,
        },
        sync_metadata: crate::api::common::SyncMetadata {
            max_update_date,
            server_timestamp,
            data_checksum,
            record_ids,
            returned_records,
            deleted_since: crate::api::common::DeletedItems {
                enabled: true,
                count: deleted_items.len(),
                items: deleted_items,
            },
        },
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        "✅ Successfully fetched {} products for user {} (total: {}, deleted: {}, date_filter: {}) in {}ms [{}]", 
        response.data.len(),
        user_id,
        total_count,
        response.sync_metadata.deleted_since.count,
        params.update_date_from.as_deref().unwrap_or("none"),
        execution_time,
        request_id
    );

    Ok(Json(ApiResponse::success(
        response, 
        request_id, 
        Some(execution_time), 
        false
    )))
}

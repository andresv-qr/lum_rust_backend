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
        get_deleted_items_since_utc,
        extract_max_update_date,
        extract_record_ids,
        parse_date_to_utc,
    },
    api::templates::user_issuers_templates::{
        UserIssuersQueryTemplates, 
        UserIssuersResponse, 
        UserIssuersRequest,
    },
};

/// Create user_issuers v4 router
pub fn create_user_issuers_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/issuers", get(get_user_issuers))
        .layer(from_fn(extract_current_user))
}

/// GET /api/v4/invoices/issuers - Get issuers that a user has invoices with
/// Now with incremental sync support (Nivel 2)
pub async fn get_user_issuers(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserIssuersRequest>,
) -> Result<Json<ApiResponse<IncrementalSyncResponse<UserIssuersResponse>>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now();
    
    info!("📋 Fetching user issuers for user_id: {} [{}]", current_user.user_id, request_id);

    // Parameters with default values following v4 standards
    let limit = params.limit.unwrap_or(20).min(100).max(1);
    let offset = params.offset.unwrap_or(0).max(0);
    let user_id = current_user.user_id;

    // Validate and parse update_date_from if provided (must be DateTime<Utc>)
    // If full_sync=true, ignore update_date_from and return all records
    let update_date_filter: Option<chrono::DateTime<chrono::Utc>> = if params.full_sync {
        info!("🔄 Full sync requested, ignoring update_date_from [{}]", request_id);
        None
    } else if let Some(date_str) = &params.update_date_from {
        match parse_date_to_utc(date_str) {
            Ok(dt) => {
                info!("🗓️ Using update_date filter: {} [{}]", dt, request_id);
                Some(dt)
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
    let issuers = if let Some(date_filter) = &update_date_filter {
        let main_query = UserIssuersQueryTemplates::get_user_issuers_with_date_filter_query();
        sqlx::query_as::<_, UserIssuersResponse>(main_query)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .bind(date_filter)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                error!("❌ Failed to execute issuers query with date filter: {} [{}]", e, request_id);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        let main_query = UserIssuersQueryTemplates::get_user_issuers_query();
        sqlx::query_as::<_, UserIssuersResponse>(main_query)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                error!("❌ Failed to execute issuers query: {} [{}]", e, request_id);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    // Get total count
    let total_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT a.issuer_ruc) as total
        FROM public.dim_issuer a 
        WHERE EXISTS (
            SELECT 1 
            FROM public.invoice_header ih 
            WHERE ih.user_id = $1 
            AND a.issuer_ruc = ih.issuer_ruc 
            AND a.issuer_name = ih.issuer_name
        )
        AND a.is_deleted = FALSE
        "#
    )
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    // Get deleted items if update_date_from was provided
    let deleted_items = match &update_date_filter {
        Some(since) => get_deleted_items_since_utc(&state.db_pool, "dim_issuer", "issuer_ruc", since).await,
        None => Vec::new(),
    };

    // Get dataset version

    // Calculate checksum
    let data_checksum = calculate_checksum(&issuers)
        .map_err(|e| {
            error!("❌ Failed to calculate checksum: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Extract max_update_date and record_ids
    let max_update_date = extract_max_update_date(&issuers);
    let record_ids = extract_record_ids(&issuers);

    // Build pagination info
    let returned_records = issuers.len();
    let total_pages = if limit > 0 { (total_count + limit - 1) / limit } else { 1 };
    let current_page = if limit > 0 { (offset / limit) + 1 } else { 1 };
    let has_more = (offset + limit) < total_count;

    // Build IncrementalSyncResponse
    let response = IncrementalSyncResponse {
        data: issuers,
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
        "✅ Successfully fetched {} issuers for user {} (total: {}, deleted: {}, date_filter: {}) in {}ms [{}]", 
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

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::api::common::{
    ApiResponse, IncrementalSyncResponse, PaginationInfo, SyncMetadata, DeletedItems,
    calculate_checksum, get_deleted_items_since, validate_date_format,
    extract_max_update_date, extract_record_ids, HasUpdateDate, HasId,
};
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;

/// Request parameters for GET /api/v4/invoices/headers
#[derive(Debug, Deserialize, Clone)]
pub struct UserInvoiceHeadersRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub update_date_from: Option<String>,
}

/// Response model for user invoice headers
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserInvoiceHeadersResponse {
    pub cufe: String,
    pub issuer_name: Option<String>,
    pub issuer_ruc: Option<String>,
    pub store_id: Option<String>,
    pub no: Option<String>,
    pub date: Option<chrono::NaiveDateTime>,
    pub tot_amount: Option<f64>,
    pub tot_itbms: Option<f64>,
    pub url: Option<String>,
    pub process_date: Option<chrono::DateTime<chrono::Utc>>,
    pub reception_date: Option<chrono::DateTime<chrono::Utc>>,
    pub r#type: Option<String>,
    pub update_date: chrono::NaiveDateTime,
}

// Implement traits for sync helpers
impl HasUpdateDate for UserInvoiceHeadersResponse {
    fn get_update_date(&self) -> Option<chrono::NaiveDateTime> {
        Some(self.update_date)
    }
}

impl HasId for UserInvoiceHeadersResponse {
    fn get_id(&self) -> Option<String> {
        Some(self.cufe.clone())
    }
}

/// Create router for user invoice headers (incremental sync)
pub fn create_user_invoice_headers_v4_router() -> Router<Arc<AppState>> {
    Router::new().route("/headers", get(get_user_invoice_headers))
}

/// GET /api/v4/invoices/headers - Get invoice headers for a user
/// Now with incremental sync support (Nivel 2)
pub async fn get_user_invoice_headers(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserInvoiceHeadersRequest>,
) -> Result<Json<ApiResponse<IncrementalSyncResponse<UserInvoiceHeadersResponse>>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now().naive_utc();
    
    info!("📋 Fetching user invoice headers for user_id: {} [{}]", current_user.user_id, request_id);

    // Parameters with default values following v4 standards
    let limit = params.limit.unwrap_or(20).min(100).max(1);
    let offset = params.offset.unwrap_or(0).max(0);
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
    let headers = if let Some(date_filter) = &update_date_filter {
        sqlx::query_as::<_, UserInvoiceHeadersResponse>(
            r#"
            SELECT cufe, issuer_name, issuer_ruc, store_id, no, date, tot_amount, tot_itbms,
                   url, process_date, reception_date, type, update_date
            FROM public.invoice_header
            WHERE user_id = $1
              AND is_deleted = FALSE
              AND update_date >= $4::timestamp
            ORDER BY date DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .bind(date_filter)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| {
            error!("❌ Failed to execute headers query with date filter: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        sqlx::query_as::<_, UserInvoiceHeadersResponse>(
            r#"
            SELECT cufe, issuer_name, issuer_ruc, store_id, no, date, tot_amount, tot_itbms,
                   url, process_date, reception_date, type, update_date
            FROM public.invoice_header
            WHERE user_id = $1
              AND is_deleted = FALSE
            ORDER BY date DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| {
            error!("❌ Failed to execute headers query: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    // Get total count
    let total_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) as total
        FROM public.invoice_header
        WHERE user_id = $1
          AND is_deleted = FALSE
        "#
    )
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    // Get deleted items if update_date_from was provided
    let deleted_items = if let Some(since) = &update_date_filter {
        get_deleted_items_since(&state.db_pool, "invoice_header", "cufe", since).await
    } else {
        vec![]
    };

    // Get dataset version

    // Calculate checksum
    let data_checksum = calculate_checksum(&headers)
        .map_err(|e| {
            error!("❌ Failed to calculate checksum: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Extract max_update_date and record_ids
    let max_update_date = extract_max_update_date(&headers);
    let record_ids = extract_record_ids(&headers);

    // Build pagination info
    let returned_records = headers.len();
    let total_pages = if limit > 0 { (total_count + limit - 1) / limit } else { 1 };
    let current_page = if limit > 0 { (offset / limit) + 1 } else { 1 };
    let has_more = (offset + limit) < total_count;

    // Build IncrementalSyncResponse
    let response = IncrementalSyncResponse {
        data: headers,
        pagination: PaginationInfo {
            total_records: total_count,
            returned_records,
            limit,
            offset,
            has_more,
            total_pages,
            current_page,
        },
        sync_metadata: SyncMetadata {
            max_update_date,
            server_timestamp,
            data_checksum,
            record_ids,
            returned_records,
            deleted_since: DeletedItems {
                enabled: true,
                count: deleted_items.len(),
                items: deleted_items,
            },
        },
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        "✅ Successfully fetched {} invoice headers for user {} (total: {}, deleted: {}, date_filter: {}) in {}ms [{}]", 
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

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::api::common::{
    ApiResponse, IncrementalSyncResponse, PaginationInfo, SyncMetadata, DeletedItems,
    calculate_checksum, get_deleted_items_since_utc, parse_date_to_utc,
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
    /// If true, returns ALL records ignoring update_date_from (full resync)
    #[serde(default)]
    pub full_sync: bool,
}

/// Response model for user invoice headers
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserInvoiceHeadersResponse {
    pub cufe: String,
    pub issuer_name: Option<String>,
    pub issuer_ruc: Option<String>,
    pub store_id: Option<String>,
    pub no: Option<String>,
    pub date: Option<chrono::DateTime<chrono::Utc>>,
    pub tot_amount: Option<f64>,
    pub tot_itbms: Option<f64>,
    pub url: Option<String>,
    pub process_date: Option<chrono::DateTime<chrono::Utc>>,
    pub reception_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub invoice_type: Option<String>,
    pub update_date: chrono::DateTime<chrono::Utc>,
}

// Implement traits for sync helpers
impl HasUpdateDate for UserInvoiceHeadersResponse {
    fn get_update_date(&self) -> Option<chrono::DateTime<chrono::Utc>> {
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
    Router::new()
        .route("/headers", get(get_user_invoice_headers))
        .route("/headers/recovery", post(recovery_invoice_headers))
}

/// Request body for POST /api/v4/invoices/headers/recovery
#[derive(Debug, Deserialize)]
pub struct RecoveryRequest {
    /// List of CUFEs the client already has locally
    pub known_cufes: Vec<String>,
    /// Maximum number of missing records to return
    pub limit: Option<i64>,
}

/// Response for recovery endpoint
#[derive(Debug, Serialize)]
pub struct RecoveryResponse<T> {
    /// Records that the client is missing
    pub missing_records: Vec<T>,
    /// CUFEs that the client has but were deleted on server
    pub deleted_cufes: Vec<String>,
    /// Total missing count (may be more than returned if limit applied)
    pub total_missing: i64,
    /// Server timestamp for reference
    pub server_timestamp: chrono::DateTime<chrono::Utc>,
}

/// POST /api/v4/invoices/headers/recovery - Recovery sync by CUFE comparison
/// 
/// Client sends list of known CUFEs, server returns:
/// - missing_records: Records the client doesn't have
/// - deleted_cufes: CUFEs the client has but were deleted on server
/// 
/// Performance: Uses LEFT JOIN with VALUES for O(n) instead of O(n²) with NOT IN
/// 
/// Use this when:
/// - Client suspects data corruption
/// - Incremental sync reports inconsistencies
/// - User reports "missing invoices"
pub async fn recovery_invoice_headers(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<RecoveryRequest>,
) -> Result<Json<ApiResponse<RecoveryResponse<UserInvoiceHeadersResponse>>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now();
    let user_id = current_user.user_id;
    
    let known_count = payload.known_cufes.len();
    let limit = payload.limit.unwrap_or(100).min(500).max(1); // Max 500 for recovery
    
    info!(
        "🔄 Recovery sync requested for user_id: {}, known_cufes: {} [{}]", 
        user_id, known_count, request_id
    );

    // Validate payload size (prevent abuse)
    if known_count > 10000 {
        warn!("❌ Recovery request too large: {} CUFEs [{}]", known_count, request_id);
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Use CTE with single query for atomicity and performance
    // This prevents race conditions between count and fetch
    if known_count == 0 {
        // Client has nothing - optimized single query with count
        #[derive(sqlx::FromRow)]
        struct RecoveryQueryResult {
            cufe: String,
            issuer_name: Option<String>,
            issuer_ruc: Option<String>,
            store_id: Option<String>,
            no: Option<String>,
            date: Option<chrono::DateTime<chrono::Utc>>,
            tot_amount: Option<f64>,
            tot_itbms: Option<f64>,
            url: Option<String>,
            process_date: Option<chrono::DateTime<chrono::Utc>>,
            reception_date: Option<chrono::DateTime<chrono::Utc>>,
            #[sqlx(rename = "type")]
            invoice_type: Option<String>,
            update_date: chrono::DateTime<chrono::Utc>,
            total_count: i64,
        }
        
        let results = sqlx::query_as::<_, RecoveryQueryResult>(
            r#"
            WITH user_headers AS (
                SELECT cufe, issuer_name, issuer_ruc, store_id, no, date, tot_amount, tot_itbms,
                       url, process_date, reception_date, type, update_date
                FROM public.invoice_header
                WHERE user_id = $1 AND is_deleted = FALSE
            ),
            total AS (SELECT COUNT(*) as cnt FROM user_headers)
            SELECT h.cufe, h.issuer_name, h.issuer_ruc, h.store_id, h.no, h.date, 
                   h.tot_amount, h.tot_itbms, h.url, h.process_date, h.reception_date, 
                   h.type, h.update_date, t.cnt as total_count
            FROM user_headers h, total t
            ORDER BY h.update_date DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch all headers for recovery: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let total_missing = results.first().map(|r| r.total_count).unwrap_or(0);
        let missing_records: Vec<UserInvoiceHeadersResponse> = results.into_iter().map(|r| {
            UserInvoiceHeadersResponse {
                cufe: r.cufe,
                issuer_name: r.issuer_name,
                issuer_ruc: r.issuer_ruc,
                store_id: r.store_id,
                no: r.no,
                date: r.date,
                tot_amount: r.tot_amount,
                tot_itbms: r.tot_itbms,
                url: r.url,
                process_date: r.process_date,
                reception_date: r.reception_date,
                invoice_type: r.invoice_type,
                update_date: r.update_date,
            }
        }).collect();

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        info!(
            "✅ Recovery sync (full) completed for user {}: missing={}/{} in {}ms [{}]",
            user_id, missing_records.len(), total_missing, execution_time, request_id
        );

        let response = RecoveryResponse {
            missing_records,
            deleted_cufes: vec![],
            total_missing,
            server_timestamp,
        };

        return Ok(Json(ApiResponse::success(response, request_id, Some(execution_time), false)));
    }

    // Client has some CUFEs - use LEFT JOIN for O(n) performance instead of NOT IN O(n²)
    // Single atomic query with CTE for count + missing + deleted
    #[derive(sqlx::FromRow)]
    struct RecoveryWithDeletedResult {
        cufe: String,
        issuer_name: Option<String>,
        issuer_ruc: Option<String>,
        store_id: Option<String>,
        no: Option<String>,
        date: Option<chrono::DateTime<chrono::Utc>>,
        tot_amount: Option<f64>,
        tot_itbms: Option<f64>,
        url: Option<String>,
        process_date: Option<chrono::DateTime<chrono::Utc>>,
        reception_date: Option<chrono::DateTime<chrono::Utc>>,
        #[sqlx(rename = "type")]
        invoice_type: Option<String>,
        update_date: chrono::DateTime<chrono::Utc>,
        total_missing: i64,
    }

    // Query for missing records with atomic count using window function
    let missing_results = sqlx::query_as::<_, RecoveryWithDeletedResult>(
        r#"
        WITH known_cufes AS (
            SELECT unnest($2::text[]) as cufe
        ),
        missing AS (
            SELECT h.cufe, h.issuer_name, h.issuer_ruc, h.store_id, h.no, h.date, 
                   h.tot_amount, h.tot_itbms, h.url, h.process_date, h.reception_date, 
                   h.type, h.update_date
            FROM public.invoice_header h
            LEFT JOIN known_cufes k ON h.cufe = k.cufe
            WHERE h.user_id = $1 
              AND h.is_deleted = FALSE 
              AND k.cufe IS NULL
        )
        SELECT m.*, COUNT(*) OVER() as total_missing
        FROM missing m
        ORDER BY m.update_date DESC
        LIMIT $3
        "#
    )
    .bind(user_id)
    .bind(&payload.known_cufes)
    .bind(limit)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to find missing headers: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_missing = missing_results.first().map(|r| r.total_missing).unwrap_or(0);
    let missing_records: Vec<UserInvoiceHeadersResponse> = missing_results.into_iter().map(|r| {
        UserInvoiceHeadersResponse {
            cufe: r.cufe,
            issuer_name: r.issuer_name,
            issuer_ruc: r.issuer_ruc,
            store_id: r.store_id,
            no: r.no,
            date: r.date,
            tot_amount: r.tot_amount,
            tot_itbms: r.tot_itbms,
            url: r.url,
            process_date: r.process_date,
            reception_date: r.reception_date,
            invoice_type: r.invoice_type,
            update_date: r.update_date,
        }
    }).collect();

    // Get deleted CUFEs (client has but server deleted) - using INNER JOIN
    let deleted_cufes: Vec<String> = match sqlx::query_scalar(
        r#"
        WITH known_cufes AS (
            SELECT unnest($2::text[]) as cufe
        )
        SELECT h.cufe 
        FROM public.invoice_header h
        INNER JOIN known_cufes k ON h.cufe = k.cufe
        WHERE h.user_id = $1 AND h.is_deleted = TRUE
        "#
    )
    .bind(user_id)
    .bind(&payload.known_cufes)
    .fetch_all(&state.db_pool)
    .await
    {
        Ok(cufes) => cufes,
        Err(e) => {
            warn!("⚠️ Failed to fetch deleted CUFEs (non-critical): {} [{}]", e, request_id);
            Vec::new()
        }
    };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        "✅ Recovery sync completed for user {}: missing={}/{}, deleted={}, known={} in {}ms [{}]",
        user_id,
        missing_records.len(),
        total_missing,
        deleted_cufes.len(),
        known_count,
        execution_time,
        request_id
    );

    let response = RecoveryResponse {
        missing_records,
        deleted_cufes,
        total_missing,
        server_timestamp,
    };

    Ok(Json(ApiResponse::success(
        response,
        request_id,
        Some(execution_time),
        false
    )))
}

/// GET /api/v4/invoices/headers - Get invoice headers for a user
/// Now with incremental sync support (Nivel 2)
/// 
/// Performance optimizations:
/// - Single query with window function for count (avoids separate COUNT query)
/// - Checksum calculated once during serialization
pub async fn get_user_invoice_headers(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserInvoiceHeadersRequest>,
) -> Result<Json<ApiResponse<IncrementalSyncResponse<UserInvoiceHeadersResponse>>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now();
    
    info!("📋 Fetching user invoice headers for user_id: {} [{}]", current_user.user_id, request_id);

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

    // Optimized: Single query with window function for total count
    #[derive(sqlx::FromRow)]
    struct HeaderWithCount {
        cufe: String,
        issuer_name: Option<String>,
        issuer_ruc: Option<String>,
        store_id: Option<String>,
        no: Option<String>,
        date: Option<chrono::DateTime<chrono::Utc>>,
        tot_amount: Option<f64>,
        tot_itbms: Option<f64>,
        url: Option<String>,
        process_date: Option<chrono::DateTime<chrono::Utc>>,
        reception_date: Option<chrono::DateTime<chrono::Utc>>,
        #[sqlx(rename = "type")]
        invoice_type: Option<String>,
        update_date: chrono::DateTime<chrono::Utc>,
        total_count: i64,
    }

    let results = if let Some(date_filter) = &update_date_filter {
        sqlx::query_as::<_, HeaderWithCount>(
            r#"
            WITH filtered AS (
                SELECT cufe, issuer_name, issuer_ruc, store_id, no, date, tot_amount, tot_itbms,
                       url, process_date, reception_date, type, update_date
                FROM public.invoice_header
                WHERE user_id = $1 AND is_deleted = FALSE AND update_date >= $4
            )
            SELECT f.*, COUNT(*) OVER() as total_count
            FROM filtered f
            ORDER BY f.date DESC
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
        sqlx::query_as::<_, HeaderWithCount>(
            r#"
            WITH all_headers AS (
                SELECT cufe, issuer_name, issuer_ruc, store_id, no, date, tot_amount, tot_itbms,
                       url, process_date, reception_date, type, update_date
                FROM public.invoice_header
                WHERE user_id = $1 AND is_deleted = FALSE
            )
            SELECT h.*, COUNT(*) OVER() as total_count
            FROM all_headers h
            ORDER BY h.date DESC
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

    // Extract total_count from first result (window function returns same value for all rows)
    let total_count = results.first().map(|r| r.total_count).unwrap_or(0);
    
    // Convert to response type
    let headers: Vec<UserInvoiceHeadersResponse> = results.into_iter().map(|r| {
        UserInvoiceHeadersResponse {
            cufe: r.cufe,
            issuer_name: r.issuer_name,
            issuer_ruc: r.issuer_ruc,
            store_id: r.store_id,
            no: r.no,
            date: r.date,
            tot_amount: r.tot_amount,
            tot_itbms: r.tot_itbms,
            url: r.url,
            process_date: r.process_date,
            reception_date: r.reception_date,
            invoice_type: r.invoice_type,
            update_date: r.update_date,
        }
    }).collect();

    // Get deleted items only if update_date_from was provided (avoid empty vec allocation)
    let deleted_items = match &update_date_filter {
        Some(since) => get_deleted_items_since_utc(&state.db_pool, "invoice_header", "cufe", since).await,
        None => Vec::new(),
    };

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

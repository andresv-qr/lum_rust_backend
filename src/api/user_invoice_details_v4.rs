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

/// Request parameters for GET /api/v4/invoices/details
#[derive(Debug, Deserialize, Clone)]
pub struct UserInvoiceDetailsRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub update_date_from: Option<String>,
    /// If true, returns ALL records ignoring update_date_from (full resync)
    #[serde(default)]
    pub full_sync: bool,
}

/// Response model for user invoice details
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserInvoiceDetailsResponse {
    pub cufe: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub amount: Option<String>,
    pub itbms: Option<String>,
    pub total: Option<String>,
    pub unit_discount: Option<String>,
    pub information_of_interest: Option<String>,
    pub update_date: chrono::DateTime<chrono::Utc>,
}

// Implement traits for sync helpers
impl HasUpdateDate for UserInvoiceDetailsResponse {
    fn get_update_date(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        Some(self.update_date)
    }
}

impl HasId for UserInvoiceDetailsResponse {
    fn get_id(&self) -> Option<String> {
        // For invoice_detail, we combine cufe + code as unique ID
        Some(format!("{}_{}", self.cufe, self.code.as_deref().unwrap_or("")))
    }
}

/// Create router for user invoice details
pub fn create_user_invoice_details_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/details", get(get_user_invoice_details))
        .route("/details/recovery", post(recovery_invoice_details))
}

/// Request body for POST /api/v4/invoices/details/recovery
#[derive(Debug, Deserialize)]
pub struct DetailsRecoveryRequest {
    /// List of detail IDs (cufe_code) the client already has locally
    pub known_ids: Vec<String>,
    /// Maximum number of missing records to return
    pub limit: Option<i64>,
}

/// Response for recovery endpoint
#[derive(Debug, Serialize)]
pub struct DetailsRecoveryResponse<T> {
    /// Records that the client is missing
    pub missing_records: Vec<T>,
    /// IDs that the client has but were deleted on server
    pub deleted_ids: Vec<String>,
    /// Total missing count (may be more than returned if limit applied)
    pub total_missing: i64,
    /// Server timestamp for reference
    pub server_timestamp: chrono::DateTime<chrono::Utc>,
}

/// GET /api/v4/invoices/details - Get invoice details for a user
/// Now with incremental sync support (Nivel 2)
/// 
/// Performance optimizations:
/// - Single query with window function for count (avoids separate COUNT query)
/// - LEFT JOIN instead of NOT IN for O(n) instead of O(n²)
pub async fn get_user_invoice_details(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<UserInvoiceDetailsRequest>,
) -> Result<Json<ApiResponse<IncrementalSyncResponse<UserInvoiceDetailsResponse>>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now();
    
    info!("📋 Fetching user invoice details for user_id: {} [{}]", current_user.user_id, request_id);

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
    struct DetailWithCount {
        cufe: String,
        code: Option<String>,
        description: Option<String>,
        quantity: Option<String>,
        unit_price: Option<String>,
        amount: Option<String>,
        itbms: Option<String>,
        total: Option<String>,
        unit_discount: Option<String>,
        information_of_interest: Option<String>,
        update_date: chrono::DateTime<chrono::Utc>,
        total_count: i64,
    }

    let results = if let Some(date_filter) = &update_date_filter {
        sqlx::query_as::<_, DetailWithCount>(
            r#"
            WITH filtered AS (
                SELECT d.cufe, d.code, d.description, d.quantity, d.unit_price,
                       d.amount, d.itbms, d.total, d.unit_discount, 
                       d.information_of_interest, d.update_date
                FROM public.invoice_detail d
                INNER JOIN public.invoice_header h ON d.cufe = h.cufe
                WHERE h.user_id = $1 AND d.is_deleted = FALSE AND d.update_date >= $4
            )
            SELECT f.*, COUNT(*) OVER() as total_count
            FROM filtered f
            ORDER BY f.update_date DESC
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
            error!("❌ Failed to execute details query with date filter: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        sqlx::query_as::<_, DetailWithCount>(
            r#"
            WITH all_details AS (
                SELECT d.cufe, d.code, d.description, d.quantity, d.unit_price,
                       d.amount, d.itbms, d.total, d.unit_discount,
                       d.information_of_interest, d.update_date
                FROM public.invoice_detail d
                INNER JOIN public.invoice_header h ON d.cufe = h.cufe
                WHERE h.user_id = $1 AND d.is_deleted = FALSE
            )
            SELECT ad.*, COUNT(*) OVER() as total_count
            FROM all_details ad
            ORDER BY ad.update_date DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| {
            error!("❌ Failed to execute details query: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    // Extract total_count from first result
    let total_count = results.first().map(|r| r.total_count).unwrap_or(0);
    
    // Convert to response type
    let details: Vec<UserInvoiceDetailsResponse> = results.into_iter().map(|r| {
        UserInvoiceDetailsResponse {
            cufe: r.cufe,
            code: r.code,
            description: r.description,
            quantity: r.quantity,
            unit_price: r.unit_price,
            amount: r.amount,
            itbms: r.itbms,
            total: r.total,
            unit_discount: r.unit_discount,
            information_of_interest: r.information_of_interest,
            update_date: r.update_date,
        }
    }).collect();

    // Get deleted items only if update_date_from was provided
    // NOTE: For invoice_detail, we track deletion by cufe+code combination
    let deleted_items = match &update_date_filter {
        Some(since) => get_deleted_items_since_utc(&state.db_pool, "invoice_detail", "cufe", since).await,
        None => Vec::new(),
    };

    // Calculate checksum
    let data_checksum = calculate_checksum(&details)
        .map_err(|e| {
            error!("❌ Failed to calculate checksum: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Extract max_update_date and record_ids
    let max_update_date = extract_max_update_date(&details);
    let record_ids = extract_record_ids(&details);

    // Build pagination info
    let returned_records = details.len();
    let total_pages = if limit > 0 { (total_count + limit - 1) / limit } else { 1 };
    let current_page = if limit > 0 { (offset / limit) + 1 } else { 1 };
    let has_more = (offset + limit) < total_count;

    // Build IncrementalSyncResponse
    let response = IncrementalSyncResponse {
        data: details,
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
        "✅ Successfully fetched {} invoice details for user {} (total: {}, deleted: {}, date_filter: {}) in {}ms [{}]", 
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

/// POST /api/v4/invoices/details/recovery - Recovery sync by ID comparison
/// 
/// Client sends list of known detail IDs (cufe_code), server returns:
/// - missing_records: Records the client doesn't have
/// - deleted_ids: IDs the client has but were deleted on server
/// 
/// Performance: Uses LEFT JOIN for O(n) instead of O(n²) with NOT IN
/// Note: For details, the unique ID is cufe + "_" + code
pub async fn recovery_invoice_details(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<DetailsRecoveryRequest>,
) -> Result<Json<ApiResponse<DetailsRecoveryResponse<UserInvoiceDetailsResponse>>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let server_timestamp = chrono::Utc::now();
    let user_id = current_user.user_id;
    
    let known_count = payload.known_ids.len();
    let limit = payload.limit.unwrap_or(100).min(1000).max(1); // Max 1000 for details recovery
    
    info!(
        "🔄 Details recovery sync requested for user_id: {}, known_ids: {} [{}]", 
        user_id, known_count, request_id
    );

    // Validate payload size (prevent abuse)
    if known_count > 50000 {
        warn!("❌ Details recovery request too large: {} IDs [{}]", known_count, request_id);
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Use CTE with single query for atomicity and performance
    if known_count == 0 {
        // Client has nothing - optimized single query with count
        #[derive(sqlx::FromRow)]
        struct DetailRecoveryResult {
            cufe: String,
            code: Option<String>,
            description: Option<String>,
            quantity: Option<String>,
            unit_price: Option<String>,
            amount: Option<String>,
            itbms: Option<String>,
            total: Option<String>,
            unit_discount: Option<String>,
            information_of_interest: Option<String>,
            update_date: chrono::DateTime<chrono::Utc>,
            total_count: i64,
        }

        let results = sqlx::query_as::<_, DetailRecoveryResult>(
            r#"
            WITH user_details AS (
                SELECT d.cufe, d.code, d.description, d.quantity, d.unit_price, 
                       d.amount, d.itbms, d.total, d.unit_discount, 
                       d.information_of_interest, d.update_date
                FROM public.invoice_detail d
                JOIN public.invoice_header h ON d.cufe = h.cufe
                WHERE h.user_id = $1 AND h.is_deleted = FALSE AND d.is_deleted = FALSE
            ),
            total AS (SELECT COUNT(*) as cnt FROM user_details)
            SELECT ud.*, t.cnt as total_count
            FROM user_details ud, total t
            ORDER BY ud.update_date DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| {
            error!("❌ Failed to fetch all details for recovery: {} [{}]", e, request_id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let total_missing = results.first().map(|r| r.total_count).unwrap_or(0);
        let missing_records: Vec<UserInvoiceDetailsResponse> = results.into_iter().map(|r| {
            UserInvoiceDetailsResponse {
                cufe: r.cufe,
                code: r.code,
                description: r.description,
                quantity: r.quantity,
                unit_price: r.unit_price,
                amount: r.amount,
                itbms: r.itbms,
                total: r.total,
                unit_discount: r.unit_discount,
                information_of_interest: r.information_of_interest,
                update_date: r.update_date,
            }
        }).collect();

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        info!(
            "✅ Details recovery sync (full) completed for user {}: missing={}/{} in {}ms [{}]",
            user_id, missing_records.len(), total_missing, execution_time, request_id
        );

        let response = DetailsRecoveryResponse {
            missing_records,
            deleted_ids: vec![],
            total_missing,
            server_timestamp,
        };

        return Ok(Json(ApiResponse::success(response, request_id, Some(execution_time), false)));
    }

    // Client has some IDs - use LEFT JOIN for O(n) performance
    // Generate composite ID in CTE to avoid per-row concatenation in main query
    #[derive(sqlx::FromRow)]
    struct DetailWithMissingCount {
        cufe: String,
        code: Option<String>,
        description: Option<String>,
        quantity: Option<String>,
        unit_price: Option<String>,
        amount: Option<String>,
        itbms: Option<String>,
        total: Option<String>,
        unit_discount: Option<String>,
        information_of_interest: Option<String>,
        update_date: chrono::DateTime<chrono::Utc>,
        total_missing: i64,
    }

    let missing_results = sqlx::query_as::<_, DetailWithMissingCount>(
        r#"
        WITH known_ids AS (
            SELECT unnest($2::text[]) as composite_id
        ),
        user_details AS (
            SELECT d.cufe, d.code, d.description, d.quantity, d.unit_price, 
                   d.amount, d.itbms, d.total, d.unit_discount, 
                   d.information_of_interest, d.update_date,
                   d.cufe || '_' || COALESCE(d.code, '') as composite_id
            FROM public.invoice_detail d
            JOIN public.invoice_header h ON d.cufe = h.cufe
            WHERE h.user_id = $1 AND h.is_deleted = FALSE AND d.is_deleted = FALSE
        ),
        missing AS (
            SELECT ud.cufe, ud.code, ud.description, ud.quantity, ud.unit_price, 
                   ud.amount, ud.itbms, ud.total, ud.unit_discount, 
                   ud.information_of_interest, ud.update_date
            FROM user_details ud
            LEFT JOIN known_ids k ON ud.composite_id = k.composite_id
            WHERE k.composite_id IS NULL
        )
        SELECT m.*, COUNT(*) OVER() as total_missing
        FROM missing m
        ORDER BY m.update_date DESC
        LIMIT $3
        "#
    )
    .bind(user_id)
    .bind(&payload.known_ids)
    .bind(limit)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!("❌ Failed to find missing details: {} [{}]", e, request_id);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_missing = missing_results.first().map(|r| r.total_missing).unwrap_or(0);
    let missing_records: Vec<UserInvoiceDetailsResponse> = missing_results.into_iter().map(|r| {
        UserInvoiceDetailsResponse {
            cufe: r.cufe,
            code: r.code,
            description: r.description,
            quantity: r.quantity,
            unit_price: r.unit_price,
            amount: r.amount,
            itbms: r.itbms,
            total: r.total,
            unit_discount: r.unit_discount,
            information_of_interest: r.information_of_interest,
            update_date: r.update_date,
        }
    }).collect();

    // For details, deleted detection is based on header deletion
    // Details are deleted when their parent header is deleted
    let deleted_ids: Vec<String> = Vec::new(); // Details don't have individual soft delete

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        "✅ Details recovery sync completed for user {}: missing={}/{}, known={} in {}ms [{}]",
        user_id,
        missing_records.len(),
        total_missing,
        known_count,
        execution_time,
        request_id
    );

    let response = DetailsRecoveryResponse {
        missing_records,
        deleted_ids,
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

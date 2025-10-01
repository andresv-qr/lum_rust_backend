use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::common::{ApiResponse, ApiError, DatabaseService, QueryParams};
use crate::api::templates::{
    InvoiceQueryTemplates,
    UserResponse, UserBalanceResponse, InvoiceResponse, InvoiceStatsResponse,
    QrHistoryResponse, QrStatsResponse, DailyUsageResponse, CacheInvalidationPatterns,
};
use crate::state::AppState;
use crate::{simple_query_handler, simple_single_query_handler};

/// Create V4 API router with new framework
pub fn create_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        // User endpoints
        .route("/api/v4/users/:id", get(get_user_by_id))
        .route("/api/v4/users/:id/balance", get(get_user_balance))
        .route("/api/v4/users/search", post(search_users))
        // Invoice endpoints (moved to invoices_v4 router)
        .route("/api/v4/users/:user_id/invoices", post(get_user_invoices))
        .route("/api/v4/users/:user_id/invoices/stats", get(get_user_invoice_stats))
        // QR endpoints
        .route("/api/v4/users/:user_id/qr/history", post(get_qr_history))
        .route("/api/v4/users/:user_id/qr/stats", get(get_qr_stats))
        // Analytics endpoints
        .route("/api/v4/users/:user_id/analytics/usage", post(get_daily_usage))
        // Write operations (with cache invalidation)
        .route("/api/v4/users/:id/balance/deduct", post(deduct_user_balance))
        // Invoice status updates (moved to invoices_v4 router if needed)
}

// ============================================================================
// READ OPERATIONS WITH CACHING
// ============================================================================

simple_single_query_handler!(get_user_by_id, UserResponse, "SELECT user_id, whatsapp_id, email, created_at, is_verified, subscription_status FROM users WHERE user_id = $1");

simple_single_query_handler!(get_user_balance, UserBalanceResponse, "SELECT balance FROM user_balances WHERE user_id = $1");

simple_query_handler!(search_users, UserResponse, "SELECT user_id, whatsapp_id, email, created_at, is_verified, subscription_status FROM users ORDER BY created_at DESC LIMIT 50");

// Invoice by ID endpoint moved to invoices_v4 router

/// Get user invoices with pagination
pub async fn get_user_invoices(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
    Json(params): Json<QueryParams>,
) -> Result<Json<ApiResponse<Vec<InvoiceResponse>>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let _db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    let _sql = InvoiceQueryTemplates::get_user_invoices_query();
    let pagination = params.pagination.unwrap_or(crate::api::common::PaginationParams { page: 1, limit: 20 });
    let offset = (pagination.page - 1) * pagination.limit;

    let user_id_str = user_id.to_string();
    let limit_str = pagination.limit.to_string();
    let offset_str = offset.to_string();
    let _cache_params = vec![user_id_str.as_str(), limit_str.as_str(), offset_str.as_str()];

    let _cache_key = format!("user_invoices_{}_{}", user_id, pagination.page);
    let sql = "SELECT invoice_id, user_id, file_path, ocr_text, processed_at, status FROM invoices WHERE user_id = $1 ORDER BY processed_at DESC LIMIT $2 OFFSET $3";
    
    let (data, cached) = sqlx::query_as::<_, InvoiceResponse>(sql)
        .bind(user_id)
        .bind(pagination.limit as i64)
        .bind(offset as i64)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query failed: {}", e)))
        .map(|rows| (rows, false))?;

    let execution_time = start_time.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(data, request_id, Some(execution_time), cached)))
}

/// Get user invoice statistics
pub async fn get_user_invoice_stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<Json<ApiResponse<Option<InvoiceStatsResponse>>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    let _sql = InvoiceQueryTemplates::get_invoice_stats_query();
    let user_id_str = user_id.to_string();
    let _cache_params = vec![user_id_str.as_str()];

    let cache_key = format!("invoice_stats_{}", user_id);
    let sql = "SELECT COUNT(*) as total, COUNT(CASE WHEN status = 'processed' THEN 1 END) as processed, COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed FROM invoices WHERE user_id = $1";
    
    let (data, cached) = db_service
        .execute_single_query_with_id::<InvoiceStatsResponse>(sql, user_id, &cache_key)
        .await?;

    let execution_time = start_time.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(data, request_id, Some(execution_time), cached)))
}

simple_query_handler!(get_qr_history, QrHistoryResponse, "SELECT qr_id, user_id, image_hash, qr_content, detection_method, processed_at FROM qr_detections ORDER BY processed_at DESC LIMIT 50");

simple_single_query_handler!(get_qr_stats, QrStatsResponse, "SELECT COUNT(*) as total, COUNT(CASE WHEN detection_method = 'rqrr' THEN 1 END) as rqrr_count, COUNT(CASE WHEN detection_method = 'quircs' THEN 1 END) as quircs_count FROM qr_detections WHERE user_id = $1");

simple_query_handler!(get_daily_usage, DailyUsageResponse, "SELECT DATE(created_at) as date, COUNT(*) as requests FROM api_logs GROUP BY DATE(created_at) ORDER BY date DESC LIMIT 30");

simple_query_handler!(get_system_metrics, crate::api::templates::SystemMetricsResponse, "SELECT endpoint, AVG(response_time_ms) as avg_response_time, COUNT(*) as request_count FROM api_logs WHERE created_at >= NOW() - INTERVAL '1 hour' GROUP BY endpoint");

// ============================================================================
// WRITE OPERATIONS WITH CACHE INVALIDATION
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DeductBalanceRequest {
    pub amount: i32,
    pub reason: String,
}

/// Deduct user balance - Example of write operation with cache invalidation
pub async fn deduct_user_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
    Json(request): Json<DeductBalanceRequest>,
) -> Result<Json<ApiResponse<String>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    // Execute write operation with cache invalidation
    let sql = "UPDATE user_balances SET balance = balance - $1, updated_at = NOW() WHERE user_id = $2 AND balance >= $1";
    let _invalidate_patterns = CacheInvalidationPatterns::user_patterns(user_id);
    // TODO: Implement cache invalidation

    let affected_rows = db_service
        .execute_write_with_params(sql, request.amount, user_id)
        .await?;

    if affected_rows == 0 {
        return Err(ApiError::validation_error("Insufficient balance or user not found"));
    }

    let execution_time = start_time.elapsed().as_millis() as u64;
    let message = format!("Successfully deducted {} Lümis from user {}", request.amount, user_id);
    
    Ok(Json(ApiResponse::success(message, request_id, Some(execution_time), false)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateInvoiceStatusRequest {
    pub status: String,
    pub notes: Option<String>,
}

/// Update invoice status - Example of write operation affecting invoice caches
pub async fn update_invoice_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invoice_id): Path<i64>,
    Json(request): Json<UpdateInvoiceStatusRequest>,
) -> Result<Json<ApiResponse<String>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    // First get the user_id for cache invalidation
    let user_query = "SELECT user_id FROM invoices WHERE invoice_id = $1";
    let user_row: Option<(i64,)> = sqlx::query_as(user_query)
        .bind(invoice_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Failed to get user_id: {}", e)))?;

    let user_id = user_row
        .map(|(uid,)| uid)
        .ok_or_else(|| ApiError::not_found("Invoice"))?;

    // Update invoice status
    let sql = "UPDATE invoices SET status = $1, notes = $2, updated_at = NOW() WHERE invoice_id = $3";
    let _invalidate_patterns = CacheInvalidationPatterns::invoice_patterns(user_id);
    // TODO: Implement cache invalidation

    let status_copy = request.status.clone();
    let affected_rows = db_service
        .execute_write_with_three_params(sql, request.status, request.notes, invoice_id)
        .await?;

    if affected_rows == 0 {
        return Err(ApiError::not_found("Invoice"));
    }

    let execution_time = start_time.elapsed().as_millis() as u64;
    let message = format!("Successfully updated invoice {} status to {}", invoice_id, status_copy);
    
    Ok(Json(ApiResponse::success(message, request_id, Some(execution_time), false)))
}

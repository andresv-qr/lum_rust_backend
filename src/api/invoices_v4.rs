use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tracing::info;

use crate::api::common::{ApiResponse, ApiError, DatabaseService};
use crate::api::templates::invoices_templates::{
    InvoicesQueryTemplates, InvoicesResponse
};
use crate::api::ocr_iterative_v4::{process_ocr_iterative, save_ocr_invoice};
use crate::api::upload_ocr_v4::upload_ocr_invoice;
use crate::api::upload_ocr_retry_v4::upload_ocr_retry;
use crate::middleware::auth::extract_current_user;
use crate::state::AppState;

/// Create invoices v4 router
/// NOTE: Routes are relative - this router is nested under /api/v4/invoices
pub fn create_invoices_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        // Changed from /:id to /by-id/:id to avoid conflicts with other routes like /sync-status
        .route("/by-id/:id", get(get_invoice_details))
        // OCR iterative endpoints (protected by auth)
        .route("/ocr-process", post(process_ocr_iterative))
        .route("/save-ocr", post(save_ocr_invoice))
        // Upload OCR endpoint (protected by auth)
        .route("/upload-ocr", post(upload_ocr_invoice))
        // Upload OCR Retry endpoint - for missing fields (protected by auth)
        .route("/upload-ocr-retry", post(upload_ocr_retry))
        // Apply auth middleware to protected routes
        .layer(axum::middleware::from_fn(extract_current_user))
}

/// get_invoice_details handler - Get single record by ID
pub async fn get_invoice_details(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<InvoicesResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let _db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    let sql = InvoicesQueryTemplates::get_invoice_details_query();
    let cache_key = format!("{}_{}", InvoicesQueryTemplates::get_invoice_details_cache_key_prefix(), id);
    
    // TODO: Check cache first
    info!("Executing query for {}: {}", cache_key, sql);
    
    let result = sqlx::query_as::<_, InvoicesResponse>(sql)
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Query execution failed: {}", e)))?;

    let data = result.ok_or_else(|| ApiError::not_found("Invoices"))?;
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    Ok(Json(ApiResponse::success(data, request_id, Some(execution_time), false)))
}

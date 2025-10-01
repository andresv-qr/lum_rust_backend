use axum::{
    extract::State,
    http::HeaderMap,
    routing::post,
    Json, Router, Extension,
};
use std::sync::Arc;
use tracing::info;

use crate::middleware::auth::CurrentUser;
use crate::api::common::{ApiResponse, ApiError, DatabaseService};
use crate::api::templates::invoice_headers_templates::{
    InvoiceHeadersQueryTemplates, InvoiceHeadersResponse, InvoiceHeadersRequest
};
use crate::state::AppState;

/// Create invoice_headers v4 router
pub fn create_invoice_headers_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/invoice_headers/search", post(get_invoice_headers))
}

/// get_invoice_headers handler - Get multiple records with filters
pub async fn get_invoice_headers(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    headers: HeaderMap,
    Json(request): Json<InvoiceHeadersRequest>,
) -> Result<Json<ApiResponse<Vec<InvoiceHeadersResponse>>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let db_service = DatabaseService::new(
        state.db_pool.clone(),
        state.user_cache.clone()
    );

    let limit = request.limit.unwrap_or(20).min(100);
    let offset = request.offset.unwrap_or(0);
    
    // Log the authenticated user for security audit
    info!("🔐 Invoice headers request from user_id: {} ({})", current_user.user_id, current_user.email);
    
    let cache_key_prefix = InvoiceHeadersQueryTemplates::get_invoice_headers_cache_key_prefix();
    let cache_key = format!("{}_{}_{}_{}_user_{}", cache_key_prefix, limit, offset, 
        serde_json::to_string(&request.filters).unwrap_or_default(), current_user.user_id);
    
    info!("Executing query for user {}: {}", current_user.user_id, InvoiceHeadersQueryTemplates::get_invoice_headers_query());
    
    // Use the new method with parameters to ensure user isolation
    let (results, _cached): (Vec<InvoiceHeadersResponse>, bool) = db_service
        .execute_query_with_params(
            InvoiceHeadersQueryTemplates::get_invoice_headers_query(),
            current_user.user_id,
            offset,
            &cache_key,
        )
        .await?;

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!("📊 Invoice headers retrieved for user {}: {} records in {}ms", 
          current_user.user_id, results.len(), execution_time);
    
    Ok(Json(ApiResponse::success(results, request_id, Some(execution_time), false)))
}

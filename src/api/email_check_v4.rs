use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use std::sync::Arc;
use tracing::info;


use crate::api::common::{ApiResponse, ApiError, DatabaseService};
use crate::api::templates::email_check_templates::{
    EmailCheckQueryTemplates, EmailCheckResponse, EmailCheckRequest
};
use crate::state::AppState;

/// Create email_check v4 router
pub fn create_email_check_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/email_check/:id/check_email_availability", post(check_email_availability))
}

/// check_email_availability handler - Write operation
pub async fn check_email_availability(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(_id): Path<i64>,
    Json(request): Json<EmailCheckRequest>,
) -> Result<Json<ApiResponse<EmailCheckResponse>>, ApiError> {
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

    let _cache_key_prefix = EmailCheckQueryTemplates::get_cache_key_prefix();
    let sql = EmailCheckQueryTemplates::check_email_availability_query();
    
    info!("Executing email check query: {}", sql);
    
    // Check if email exists in database
    let exists_result = sqlx::query(sql)
        .bind(&request.email)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Failed to check email: {}", e)))?;

    let exists = exists_result.is_some();
    let message = if exists {
        "El email ya está registrado en el sistema".to_string()
    } else {
        "El email está disponible para registro".to_string()
    };

    let response_data = EmailCheckResponse { exists, message };

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    Ok(Json(ApiResponse::success(response_data, request_id, Some(execution_time), false)))
}

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    api::{
        common::ApiResponse,
        models::ErrorResponse,
        templates::qr_processing_templates::*,
    },
    state::AppState,
};

/// GET /api/v4/qr_processing/:user_id - Get comprehensive QR processing status
pub async fn get_qr_processing_status(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Result<Json<ApiResponse<QrProcessingOverviewResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        user_id = user_id,
        "📊 Processing QR status request"
    );

    // Note: DatabaseService not needed for direct queries in this case

    // Get processing status
    let processing_status_result = sqlx::query_as::<_, QrProcessingStatusResponse>(
        QrProcessingQueryTemplates::get_qr_processing_status_query()
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await;

    let processing_status = match processing_status_result {
        Ok(Some(mut status)) => {
            // Calculate success rate
            status.success_rate = if status.total_processed > 0 {
                (status.successful_extractions as f64 / status.total_processed as f64) * 100.0
            } else {
                0.0
            };
            
            // Determine processing health
            status.processing_health = if status.success_rate >= 90.0 {
                "excellent".to_string()
            } else if status.success_rate >= 75.0 {
                "good".to_string()
            } else if status.success_rate >= 50.0 {
                "fair".to_string()
            } else {
                "needs_attention".to_string()
            };
            
            status
        },
        Ok(None) => QrProcessingStatusResponse {
            total_processed: 0,
            successful_extractions: 0,
            processed_today: 0,
            avg_processing_time_seconds: None,
            success_rate: 0.0,
            processing_health: "no_data".to_string(),
        },
        Err(e) => {
            warn!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to get QR processing status"
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get QR processing status".to_string(),
                    message: "Database query failed".to_string(),
                    details: Some(format!("Request ID: {}", request_id)),
                }),
            ));
        }
    };

    // Get MEF pending status
    let pending_status_result = sqlx::query_as::<_, MefPendingStatusResponse>(
        QrProcessingQueryTemplates::get_mef_pending_status_query()
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await;

    let pending_status = match pending_status_result {
        Ok(Some(mut status)) => {
            // Determine pending health
            status.pending_health = if status.total_pending == 0 {
                "excellent".to_string()
            } else if status.total_pending <= 5 {
                "good".to_string()
            } else if status.total_pending <= 15 {
                "fair".to_string()
            } else {
                "needs_attention".to_string()
            };
            
            status
        },
        Ok(None) => MefPendingStatusResponse {
            total_pending: 0,
            qr_pending: 0,
            pending_today: 0,
            oldest_pending: None,
            pending_health: "excellent".to_string(),
        },
        Err(e) => {
            warn!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to get MEF pending status"
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get MEF pending status".to_string(),
                    message: "Database query failed".to_string(),
                    details: Some(format!("Request ID: {}", request_id)),
                }),
            ));
        }
    };

    // Get recent activity
    let recent_activity_result = sqlx::query_as::<_, QrActivityResponse>(
        QrProcessingQueryTemplates::get_recent_qr_activity_query()
    )
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await;

    let recent_activity = match recent_activity_result {
        Ok(activity) => activity,
        Err(e) => {
            warn!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to get recent QR activity"
            );
            vec![]
        }
    };

    // Check web scraping and Python fallback health
    let web_scraping_health = "operational".to_string(); // Could be enhanced with actual health checks
    let python_fallback_available = state.qr_service.is_python_available().await;

    let overview_response = QrProcessingOverviewResponse {
        processing_status,
        pending_status,
        recent_activity,
        web_scraping_health,
        python_fallback_available,
        last_updated: chrono::Utc::now(),
    };

    let execution_time = start_time.elapsed();
    
    info!(
        request_id = %request_id,
        user_id = user_id,
        execution_time_ms = execution_time.as_millis(),
        total_processed = overview_response.processing_status.total_processed,
        total_pending = overview_response.pending_status.total_pending,
        "✅ QR processing status retrieved successfully"
    );

    Ok(Json(ApiResponse {
        success: true,
        data: Some(overview_response),
        error: None,
        request_id,
        timestamp: chrono::Utc::now(),
        execution_time_ms: Some(execution_time.as_millis() as u64),
        cached: false,
    }))
}

/// GET /api/v4/qr_processing/:user_id/health - Get QR processing health summary
pub async fn get_qr_processing_health(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        user_id = user_id,
        "🏥 Processing QR health check request"
    );

    // Quick health check query
    let health_result = sqlx::query!(
        "SELECT 
            COUNT(*) as total_processed,
            COUNT(CASE WHEN cufe IS NOT NULL THEN 1 END) as successful_extractions,
            (SELECT COUNT(*) FROM mef_pending WHERE user_id = $1) as total_pending
         FROM invoice_header 
         WHERE user_id = $1",
        user_id
    )
    .fetch_one(&state.db_pool)
    .await;

    let health_data = match health_result {
        Ok(row) => {
            let success_rate = if row.total_processed.unwrap_or(0) > 0 {
                (row.successful_extractions.unwrap_or(0) as f64 / row.total_processed.unwrap_or(1) as f64) * 100.0
            } else {
                0.0
            };
            
            let overall_health = if success_rate >= 90.0 && row.total_pending.unwrap_or(0) <= 5 {
                "excellent"
            } else if success_rate >= 75.0 && row.total_pending.unwrap_or(0) <= 15 {
                "good"
            } else if success_rate >= 50.0 {
                "fair"
            } else {
                "needs_attention"
            };

            serde_json::json!({
                "overall_health": overall_health,
                "success_rate": success_rate,
                "total_processed": row.total_processed,
                "successful_extractions": row.successful_extractions,
                "total_pending": row.total_pending,
                "web_scraping_operational": true,
                "python_fallback_available": state.qr_service.is_python_available().await,
                "last_check": chrono::Utc::now()
            })
        },
        Err(e) => {
            warn!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to get QR health data"
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get QR health data".to_string(),
                    message: "Database query failed".to_string(),
                    details: Some(format!("Request ID: {}", request_id)),
                }),
            ));
        }
    };

    let execution_time = start_time.elapsed();
    
    info!(
        request_id = %request_id,
        user_id = user_id,
        execution_time_ms = execution_time.as_millis(),
        "✅ QR health check completed successfully"
    );

    Ok(Json(ApiResponse {
        success: true,
        data: Some(health_data),
        error: None,
        request_id,
        timestamp: chrono::Utc::now(),
        execution_time_ms: Some(execution_time.as_millis() as u64),
        cached: false,
    }))
}

/// Create the QR processing V4 router
pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:user_id", get(get_qr_processing_status))
        .route("/:user_id/health", get(get_qr_processing_health))
}

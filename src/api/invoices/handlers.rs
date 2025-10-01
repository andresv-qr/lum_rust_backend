use axum::{
    extract::{State, Json},
    response::Json as ResponseJson,
    http::StatusCode,
};
use chrono::Utc;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

// Import AppState instead of using PgPool directly
use crate::state::AppState;

use crate::api::invoices::{
    models::{ProcessInvoiceRequest, ProcessInvoiceResponse},
    validation::{validate_process_request, determine_invoice_type},
    error_handling::{InvoiceProcessingError, create_success_response},
    repository::{check_duplicate_invoice, save_full_invoice},
    logging_service::LoggingService,
    scraper_service::ScraperService,
};

// ============================================================================
// HANDLER FUNCTIONS
// ============================================================================

/// Main endpoint: POST /api/invoices/process
/// 
/// Processes a DGI Panama invoice URL with full validation, scraping,
/// database persistence, and comprehensive logging.
pub async fn process_invoice_handler(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ProcessInvoiceRequest>,
) -> Result<ResponseJson<ProcessInvoiceResponse>, InvoiceProcessingError> {
    let start_time = Utc::now();
    
    info!(
        "Starting invoice processing for user: {}, URL: {}, origin: {}",
        request.user_id, request.url, request.origin
    );
    
    // Extract pool from app state
    let pool = &app_state.db_pool;
    
    // Initialize services
    let logging_service = LoggingService::new(pool.clone());
    let scraper_service = ScraperService::new();
    
    // 1. VALIDATION PHASE
    debug!("Phase 1: Validating request");
    if let Err(validation_errors) = validate_process_request(&request) {
        error!("Validation failed: {:?}", validation_errors);
        
        // Try to create log entry even for validation errors
        if let Ok(log_id) = logging_service.start_processing(
            &request.url,
            &request.origin,
            &request.user_id,
            &request.user_email,
        ).await {
            let _ = logging_service.log_validation_error(log_id, &validation_errors, start_time).await;
        }
        
        return Err(InvoiceProcessingError::ValidationError { errors: validation_errors });
    }
    
    // 2. LOGGING INITIALIZATION
    debug!("Phase 2: Initializing logging");
    let log_id = logging_service.start_processing(
        &request.url,
        &request.origin,
        &request.user_id,
        &request.user_email,
    ).await.map_err(|e| {
        error!("Failed to initialize logging: {:?}", e);
        InvoiceProcessingError::InternalError {
            message: format!("Logging initialization failed: {:?}", e),
        }
    })?;
    
    info!("Created log entry with ID: {}", log_id);
    
    // 3. CALCULATE AUTOMATIC FIELDS
    debug!("Phase 3: Calculating automatic fields");
    let invoice_type = determine_invoice_type(&request.url);
    let reception_date = Utc::now();
    let process_date = Utc::now();
    
    debug!("Determined invoice type: {}", invoice_type);
    
    // 4. WEB SCRAPING PHASE
    debug!("Phase 4: Starting web scraping");
    let (full_invoice_data, fields_extracted, retry_attempts) = match scraper_service
        .scrape_invoice_with_retries(
            &request.url,
            &request.user_id,
            &request.user_email,
            &request.origin,
            &invoice_type,
            reception_date,
            process_date,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Scraping failed: {:?}", e);
            
            // Log the scraping error
            match &e {
                InvoiceProcessingError::ScrapingError { message, error_type, retry_attempts } => {
                    let _ = logging_service.log_scraping_error(
                        log_id, message, error_type.clone(), start_time, *retry_attempts
                    ).await;
                },
                InvoiceProcessingError::TimeoutError { attempts } => {
                    let _ = logging_service.log_timeout_error(log_id, start_time, *attempts).await;
                },
                InvoiceProcessingError::NetworkError { message } => {
                    let _ = logging_service.log_network_error(log_id, message, start_time, 0).await;
                },
                _ => {
                    let _ = logging_service.log_scraping_error(
                        log_id, &e.to_string(), crate::api::invoices::models::ErrorType::Unknown, start_time, 0
                    ).await;
                },
            }
            
            return Err(e);
        }
    };
    
    info!(
        "Successfully scraped invoice data for CUFE: {}, fields: {}, retries: {}",
        full_invoice_data.header.cufe, fields_extracted, retry_attempts
    );
    
    // 5. DUPLICATE CHECK PHASE
    debug!("Phase 5: Checking for duplicates");
    if let Some((original_user, processed_date)) = check_duplicate_invoice(&pool, &full_invoice_data.header.cufe).await? {
        warn!(
            "Duplicate invoice detected: CUFE: {}, original user: {}, processed: {}",
            full_invoice_data.header.cufe, original_user, processed_date
        );
        
        // Log duplicate detection
        let _ = logging_service.log_duplicate(log_id, &full_invoice_data.header.cufe, start_time).await;
        
        return Err(InvoiceProcessingError::DuplicateInvoice {
            cufe: full_invoice_data.header.cufe,
            original_user: Some(original_user),
            processed_date: Some(processed_date),
        });
    }
    
    debug!("No duplicate found, proceeding with save");
    
    // 6. DATABASE PERSISTENCE PHASE (ATOMIC TRANSACTION)
    debug!("Phase 6: Saving to database");
    if let Err(e) = save_full_invoice(&pool, &full_invoice_data).await {
        error!("Database save failed: {:?}", e);
        
        // Log database error
        let _ = logging_service.        log_database_error(log_id, &format!("{:?}", e), start_time).await;
        
        return Err(e);
    }
    
    info!("Successfully saved invoice to database");
    
    // 7. SUCCESS LOGGING
    debug!("Phase 7: Logging success");
    logging_service.log_success(
        log_id,
        &full_invoice_data.header.cufe,
        start_time,
        fields_extracted,
        retry_attempts,
    ).await.map_err(|e| {
        error!("Failed to log success: {:?}", e);
        // Don't fail the request if logging fails
        warn!("Logging error ignored: {:?}", e);
    }).ok();
    
    // 8. BUILD SUCCESS RESPONSE
    let response = create_success_response(
        full_invoice_data.header.cufe,
        full_invoice_data.header.no,
        full_invoice_data.header.issuer_name,
        full_invoice_data.header.tot_amount,
        full_invoice_data.details.len(),
    );
    
    info!(
        "Invoice processing completed successfully in {}ms",
        (Utc::now() - start_time).num_milliseconds()
    );
    
    Ok(ResponseJson(response))
}

// ============================================================================
// HEALTH CHECK AND STATUS ENDPOINTS
// ============================================================================

/// Health check endpoint for the invoice processing service
pub async fn health_check_handler() -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    Ok(ResponseJson(serde_json::json!({
        "status": "healthy",
        "service": "invoice_processing",
        "timestamp": Utc::now(),
        "version": "1.0.0"
    })))
}

/// Get processing statistics for a user
pub async fn user_stats_handler(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<ResponseJson<serde_json::Value>, InvoiceProcessingError> {
    let logging_service = LoggingService::new(app_state.db_pool.clone());
    let stats = logging_service.get_user_stats(&user_id, 30).await?;
    
    Ok(ResponseJson(serde_json::json!({
        "user_id": user_id,
        "period_days": 30,
        "stats": {
            "total_requests": stats.total_requests,
            "successful_requests": stats.successful_requests,
            "duplicate_requests": stats.duplicate_requests,
            "failed_requests": stats.failed_requests,
            "success_rate": if stats.total_requests > 0 {
                (stats.successful_requests as f64 / stats.total_requests as f64) * 100.0
            } else {
                0.0
            },
            "avg_execution_time_ms": stats.avg_execution_time_ms
        }
    })))
}

/// Get system-wide processing statistics
pub async fn system_stats_handler(
    State(app_state): State<Arc<AppState>>,
) -> Result<ResponseJson<serde_json::Value>, InvoiceProcessingError> {
    let logging_service = LoggingService::new(app_state.db_pool.clone());
    let stats = logging_service.get_system_stats(24).await?;
    
    Ok(ResponseJson(serde_json::json!({
        "period_hours": 24,
        "stats": {
            "total_requests": stats.total_requests,
            "successful_requests": stats.successful_requests,
            "duplicate_requests": stats.duplicate_requests,
            "unique_users": stats.unique_users,
            "success_rate": if stats.total_requests > 0 {
                (stats.successful_requests as f64 / stats.total_requests as f64) * 100.0
            } else {
                0.0
            },
            "avg_execution_time_ms": stats.avg_execution_time_ms,
            "max_execution_time_ms": stats.max_execution_time_ms,
            "requests_with_retries": stats.requests_with_retries
        }
    })))
}

// ============================================================================
// ROUTER CONFIGURATION
// ============================================================================

use axum::{
    routing::{get, post},
    Router,
};

/// Creates the router for all invoice processing endpoints
pub fn create_invoice_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/process", post(process_invoice_handler))
        .route("/health", get(health_check_handler))
        .route("/stats/user/:user_id", get(user_stats_handler))
        .route("/stats/system", get(system_stats_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    
    // Note: These tests would require a test database setup
    // For now, they serve as documentation of expected behavior
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_process_invoice_validation_error() {
        // This test would verify validation error handling
    }
    
    #[tokio::test]
    #[ignore] // Requires database setup  
    async fn test_process_invoice_duplicate_detection() {
        // This test would verify duplicate detection
    }
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_process_invoice_success_flow() {
        // This test would verify the complete success flow
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let app = Router::new().route("/health", get(health_check_handler));
        
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

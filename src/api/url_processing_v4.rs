use axum::{
    extract::State,
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use std::sync::Arc;
use tracing::{info, error};
use uuid::Uuid;

use crate::api::common::{ApiError, ApiResponse};
use crate::api::webscraping::scrape_invoice;
use crate::api::database_persistence::persist_scraped_data;
use crate::api::templates::url_processing_templates::ProcessUrlResponse;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct UrlRequest {
    pub url: String,
}

#[axum::debug_handler]
pub async fn process_url_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<UrlRequest>,
) -> Result<Json<ApiResponse<ProcessUrlResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    
    info!("Processing URL request: {}", request.url);
    
    if request.url.trim().is_empty() {
        return Err(ApiError::validation_error("URL is required"));
    }

    // Scrape the invoice
    match scrape_invoice(&state.http_client, &request.url).await {
        Ok(scraping_result) => {
            // Save to database
            let db_result = persist_scraped_data(&state.db_pool, scraping_result, &request.url).await;
            
            let execution_time = start_time.elapsed().as_millis() as u64;
            
            match db_result {
                Ok(process_response) => {
                    let response = ApiResponse {
                        success: true,
                        data: Some(process_response),
                        error: None,
                        request_id,
                        timestamp: chrono::Utc::now(),
                        execution_time_ms: Some(execution_time),
                        cached: false,
                    };
                    Ok(Json(response))
                }
                Err(error_response) => {
                    let response = ApiResponse {
                        success: false,
                        data: Some(error_response),
                        error: None,
                        request_id,
                        timestamp: chrono::Utc::now(),
                        execution_time_ms: Some(execution_time),
                        cached: false,
                    };
                    Ok(Json(response))
                }
            }
        }
        Err(e) => {
            error!("Scraping failed: {}", e);
            Err(ApiError::new("SCRAPING_ERROR", "Failed to scrape invoice data"))
        }
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/process-from-url", post(process_url_handler))
}

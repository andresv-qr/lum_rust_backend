use axum::{extract::Request, middleware::Next, response::Response, http::StatusCode};
use std::time::Duration;
use crate::api::models::ErrorResponse;
use tracing::warn;

// Simple request limits (body size + timeout). Body size to be enforced at extractor layer later.
const GLOBAL_REQUEST_TIMEOUT_SECS: u64 = 30;

pub async fn request_limits_middleware(
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<ErrorResponse>)> {
    let fut = next.run(req);
    match tokio::time::timeout(Duration::from_secs(GLOBAL_REQUEST_TIMEOUT_SECS), fut).await {
        Ok(resp) => Ok(resp),
        Err(_elapsed) => {
            warn!("Request timeout exceeded");
            Err((StatusCode::REQUEST_TIMEOUT, axum::Json(ErrorResponse {
                error: "REQUEST_TIMEOUT".into(),
                message: format!("Request exceeded {}s timeout", GLOBAL_REQUEST_TIMEOUT_SECS),
                details: None,
            })))
        }
    }
}

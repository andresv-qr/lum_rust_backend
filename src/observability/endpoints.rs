// ============================================================================
// PROMETHEUS METRICS ENDPOINT
// ============================================================================

use axum::{http::StatusCode, response::IntoResponse};
use prometheus::{Encoder, TextEncoder};

/// Handler para el endpoint /metrics de Prometheus
pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => {
            let response = String::from_utf8(buffer).unwrap_or_else(|_| String::from(""));
            (StatusCode::OK, response).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode metrics").into_response()
        }
    }
}

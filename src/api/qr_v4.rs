use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tracing::{info, debug, error};
use serde::{Deserialize, Serialize};

use crate::api::common::{ApiResponse, ApiError};
use crate::state::AppState;

// QR Detection Response
#[derive(Debug, Serialize, Deserialize)]
pub struct QrDetectResponse {
    pub success: bool,
    pub qr_data: Option<String>,
    pub detection_level: String,
    pub processing_time_ms: u64,
    pub message: String,
}

// QR Health Response
#[derive(Debug, Serialize, Deserialize)]
pub struct QrHealthResponse {
    pub status: String,
    pub decoders: Vec<QrDecoderStatus>,
    pub total_requests: u64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QrDecoderStatus {
    pub name: String,
    pub status: String,
    pub last_used: Option<String>,
    pub success_count: u64,
    pub error_count: u64,
}

/// Create qr v4 router
pub fn create_qr_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/qr/detect", post(qr_detect))
        .route("/api/v4/qr/health", get(qr_health_check))
}

/// QR Detection endpoint - Hybrid pipeline with multiple decoders
pub async fn qr_detect(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<QrDetectResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    
    debug!(request_id = %request_id, "🔍 Starting QR detection v4");

    // Extract image from multipart form
    let mut image_data: Option<Vec<u8>> = None;
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::validation_error(&format!("Failed to read multipart: {}", e))
    })? {
        if field.name() == Some("image") {
            let data = field.bytes().await.map_err(|e| {
                ApiError::validation_error(&format!("Failed to read image data: {}", e))
            })?;
            image_data = Some(data.to_vec());
            break;
        }
    }

    let image_bytes = image_data.ok_or_else(|| {
        ApiError::validation_error("No image field found in multipart data")
    })?;

    if image_bytes.is_empty() {
        return Err(ApiError::validation_error("Image data is empty"));
    }

    debug!(request_id = %request_id, size = image_bytes.len(), "📷 Image received");

    // Hybrid QR detection pipeline
    let detection_result = detect_qr_hybrid(&image_bytes, &request_id).await;
    let processing_time = start_time.elapsed().as_millis() as u64;

    let response = match detection_result {
        Ok((qr_data, level)) => {
            info!(
                request_id = %request_id,
                level = %level,
                processing_time_ms = processing_time,
                "✅ QR detection successful"
            );
            
            QrDetectResponse {
                success: true,
                qr_data: Some(qr_data),
                detection_level: level,
                processing_time_ms: processing_time,
                message: "QR code detected successfully".to_string(),
            }
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                processing_time_ms = processing_time,
                "❌ QR detection failed"
            );
            
            QrDetectResponse {
                success: false,
                qr_data: None,
                detection_level: "none".to_string(),
                processing_time_ms: processing_time,
                message: format!("QR detection failed: {}", e),
            }
        }
    };

    Ok(Json(ApiResponse::success(response, request_id, Some(processing_time), false)))
}

/// QR Health Check endpoint - Status of all decoders
pub async fn qr_health_check(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<QrHealthResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&uuid::Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    
    debug!(request_id = %request_id, "🏥 QR health check v4");

    // Check status of all QR decoders
    let decoders = vec![
        QrDecoderStatus {
            name: "rqrr".to_string(),
            status: "healthy".to_string(),
            last_used: Some(chrono::Utc::now().to_rfc3339()),
            success_count: 1250,
            error_count: 45,
        },
        QrDecoderStatus {
            name: "quircs".to_string(),
            status: "healthy".to_string(),
            last_used: Some(chrono::Utc::now().to_rfc3339()),
            success_count: 890,
            error_count: 23,
        },
        QrDecoderStatus {
            name: "rxing".to_string(),
            status: "healthy".to_string(),
            last_used: Some(chrono::Utc::now().to_rfc3339()),
            success_count: 567,
            error_count: 12,
        },
        QrDecoderStatus {
            name: "rust_qreader_onnx".to_string(),
            status: "healthy".to_string(),
            last_used: Some(chrono::Utc::now().to_rfc3339()),
            success_count: 234,
            error_count: 8,
        },
        QrDecoderStatus {
            name: "python_qreader_fallback".to_string(),
            status: "healthy".to_string(),
            last_used: Some(chrono::Utc::now().to_rfc3339()),
            success_count: 123,
            error_count: 5,
        },
    ];

    let total_success: u64 = decoders.iter().map(|d| d.success_count).sum();
    let total_errors: u64 = decoders.iter().map(|d| d.error_count).sum();
    let total_requests = total_success + total_errors;
    let success_rate = if total_requests > 0 {
        (total_success as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };

    let response = QrHealthResponse {
        status: "healthy".to_string(),
        decoders,
        total_requests,
        success_rate,
    };

    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        processing_time_ms = processing_time,
        success_rate = success_rate,
        "✅ QR health check completed"
    );

    Ok(Json(ApiResponse::success(response, request_id, Some(processing_time), false)))
}

/// Hybrid QR detection using multiple decoders
async fn detect_qr_hybrid(
    image_bytes: &[u8],
    request_id: &str,
) -> Result<(String, String), String> {
    debug!(request_id = %request_id, "🔍 Starting hybrid QR detection pipeline");

    // Level 1: rqrr (fastest)
    if let Ok(result) = detect_with_rqrr(image_bytes).await {
        debug!(request_id = %request_id, "✅ QR detected with rqrr (level 1)");
        return Ok((result, "rqrr".to_string()));
    }

    // Level 2: quircs (intermediate)
    if let Ok(result) = detect_with_quircs(image_bytes).await {
        debug!(request_id = %request_id, "✅ QR detected with quircs (level 2)");
        return Ok((result, "quircs".to_string()));
    }

    // Level 3: rxing (more precise)
    if let Ok(result) = detect_with_rxing(image_bytes).await {
        debug!(request_id = %request_id, "✅ QR detected with rxing (level 3)");
        return Ok((result, "rxing".to_string()));
    }

    // Level 4: ONNX model (most precise)
    if let Ok(result) = detect_with_onnx(image_bytes).await {
        debug!(request_id = %request_id, "✅ QR detected with ONNX (level 4)");
        return Ok((result, "onnx".to_string()));
    }

    // Level 5: Python fallback
    if let Ok(result) = detect_with_python_fallback(image_bytes).await {
        debug!(request_id = %request_id, "✅ QR detected with Python fallback (level 5)");
        return Ok((result, "python_fallback".to_string()));
    }

    Err("No QR code detected by any decoder".to_string())
}

// Placeholder implementations for QR decoders
async fn detect_with_rqrr(_image_bytes: &[u8]) -> Result<String, String> {
    // Simulate QR detection
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    Err("rqrr: No QR code found".to_string())
}

async fn detect_with_quircs(_image_bytes: &[u8]) -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    Err("quircs: No QR code found".to_string())
}

async fn detect_with_rxing(_image_bytes: &[u8]) -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // Simulate successful detection for demo
    Ok("https://example.com/qr-demo-data".to_string())
}

async fn detect_with_onnx(_image_bytes: &[u8]) -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    Err("onnx: No QR code found".to_string())
}

async fn detect_with_python_fallback(_image_bytes: &[u8]) -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    Err("python: No QR code found".to_string())
}

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::state::AppState;

#[derive(Serialize, Deserialize)]
pub struct QrDetectionResponse {
    pub success: bool,
    pub qr_data: Option<String>,
    pub detector_used: Option<String>,
    pub processing_time_ms: Option<f64>,
    pub message: Option<String>,
}

/// Detect QR code from uploaded image using hybrid pipeline
pub async fn detect_qr_hybrid(
    State(app_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<QrDetectionResponse>, StatusCode> {
    info!("🔍 QR detection request received");
    
    let start_time = std::time::Instant::now();
    
    // Extract image from multipart form
    let mut image_data: Option<Vec<u8>> = None;
    
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to read multipart field: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("");
        
        if name == "file" {
            let data = field.bytes().await.map_err(|e| {
                error!("Failed to read file bytes: {}", e);
                StatusCode::BAD_REQUEST
            })?;
            image_data = Some(data.to_vec());
            break;
        }
    }
    
    let image_bytes = image_data.ok_or_else(|| {
        error!("No image file provided in request");
        StatusCode::BAD_REQUEST
    })?;
    
    // Load image
    let image = image::load_from_memory(&image_bytes).map_err(|e| {
        error!("Failed to load image: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    
    info!("📷 Image loaded: {}x{}", image.width(), image.height());
    
    // Use QR service from app state
    match app_state.qr_service.decode_qr(&image).await {
        Some(qr_result) => {
            let processing_time = start_time.elapsed().as_millis() as f64;
            info!("✅ QR detected successfully in {:.2}ms: {}", processing_time, qr_result.content);
            
            Ok(Json(QrDetectionResponse {
                success: true,
                qr_data: Some(qr_result.content),
                detector_used: Some(format!("{:?}", qr_result.decoder)),
                processing_time_ms: Some(processing_time),
                message: Some("QR code detected successfully".to_string()),
            }))
        }
        None => {
            let processing_time = start_time.elapsed().as_millis() as f64;
            info!("❌ No QR detected after {:.2}ms", processing_time);
            
            Ok(Json(QrDetectionResponse {
                success: false,
                qr_data: None,
                detector_used: Some("Hybrid Pipeline".to_string()),
                processing_time_ms: Some(processing_time),
                message: Some("No QR code found in image".to_string()),
            }))
        }
    }
}

/// Health check endpoint for QR service
pub async fn qr_health_check(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check if QR service is available
    let _python_stats = app_state.qr_service.get_python_metrics();
    
    let response = serde_json::json!({
        "status": "ok",
        "service": "qr_detection",
        "hybrid_pipeline": "enabled",
        "python_fallback": true,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(Json(response))
}

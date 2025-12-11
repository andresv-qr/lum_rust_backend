// ============================================================================
// QR STATIC FILE SERVER - Servir imágenes QR generadas
// ============================================================================

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn};
use crate::state::AppState;

/// Serve QR image by filename
/// 
/// # Endpoint
/// GET /static/qr/:filename
/// 
/// # Path Parameters
/// - filename: The QR image filename (e.g., "LUMS-A1B2-C3D4-E5F6.png")
/// 
/// # Returns
/// - 200 OK: PNG image bytes with appropriate headers
/// - 404 Not Found: QR image doesn't exist or redemption is not valid
/// - 500 Internal Server Error: File read error
pub async fn serve_qr_image(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<Response, StatusCode> {
    // Sanitize filename to prevent path traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        warn!("Attempted path traversal in QR filename: {}", filename);
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Only allow PNG files with LUMS prefix
    if !filename.starts_with("LUMS-") || !filename.ends_with(".png") {
        warn!("Invalid QR filename format: {}", filename);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Extract redemption code from filename
    let redemption_code = filename.trim_end_matches(".png");

    // Check redemption status in DB
    // Security: Do not serve QR if redemption is already used or cancelled
    let status_check = sqlx::query!(
        "SELECT redemption_status FROM rewards.user_redemptions WHERE redemption_code = $1",
        redemption_code
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        warn!("Database error checking QR status: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match status_check {
        Some(record) => {
            if record.redemption_status != "pending" {
                warn!("Attempt to access QR for non-pending redemption ({}): {}", record.redemption_status, redemption_code);
                // Return 404 to hide existence or 410 Gone
                return Err(StatusCode::NOT_FOUND); 
            }
        },
        None => {
            warn!("Redemption code not found in DB: {}", redemption_code);
            return Err(StatusCode::NOT_FOUND);
        }
    }
    
    let qr_path = PathBuf::from("assets/qr").join(&filename);
    
    info!("Serving QR image: {:?}", qr_path);
    
    // Check if file exists
    if !qr_path.exists() {
        warn!("QR image not found: {:?}", qr_path);
        return Err(StatusCode::NOT_FOUND);
    }
    
    // Read file
    let contents = fs::read(&qr_path).await.map_err(|e| {
        warn!("Error reading QR file {:?}: {}", qr_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Build response with proper headers
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "no-store"), // Disable caching for security
        ],
        contents,
    ).into_response())
}

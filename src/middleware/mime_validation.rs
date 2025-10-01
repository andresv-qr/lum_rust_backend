use axum::{
    extract::Request, 
    middleware::Next, 
    response::Response, 
    http::StatusCode
};
use tracing::debug;
use crate::api::models::ErrorResponse;

const MAX_UPLOAD_SIZE: usize = 10 * 1024 * 1024; // 10MB
const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png", 
    "image/gif",
    "image/webp",
    "application/pdf",
    "application/json"
];

pub struct MimeValidator;

impl MimeValidator {
    pub fn is_allowed(content_type: &str) -> bool {
        ALLOWED_MIME_TYPES.iter().any(|&allowed| content_type.starts_with(allowed))
    }

    pub fn validate_magic_bytes(data: &[u8]) -> Option<&'static str> {
        if data.len() < 8 { return None; }

        match data {
            // JPEG
            [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
            // PNG  
            [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some("image/png"),
            // GIF87a or GIF89a
            [0x47, 0x49, 0x46, 0x38, 0x37, 0x61, ..] | 
            [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, ..] => Some("image/gif"),
            // PDF
            [0x25, 0x50, 0x44, 0x46, ..] => Some("application/pdf"),
            // WebP
            d if d.len() >= 12 && &d[0..4] == b"RIFF" && &d[8..12] == b"WEBP" => Some("image/webp"),
            _ => None,
        }
    }
}

pub async fn validate_upload_middleware(
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<ErrorResponse>)> {
    let path = req.uri().path();
    
    // Apply only to upload endpoints
    if !(path.contains("/upload-ocr") || path.contains("/qr/detect")) {
        return Ok(next.run(req).await);
    }

    let headers = req.headers();
    
    // Validate Content-Type header if present
    if let Some(content_type) = headers.get("content-type") {
        if let Ok(ct_str) = content_type.to_str() {
            if ct_str.starts_with("multipart/form-data") {
                // For multipart, validation will happen at extraction time
                debug!("Multipart upload detected, deferring validation");
            } else if !MimeValidator::is_allowed(ct_str) {
                return Err((StatusCode::UNSUPPORTED_MEDIA_TYPE, axum::Json(ErrorResponse {
                    error: "UNSUPPORTED_MEDIA_TYPE".into(),
                    message: format!("Content type '{}' not allowed", ct_str),
                    details: Some(format!("Allowed types: {}", ALLOWED_MIME_TYPES.join(", "))),
                })));
            }
        }
    }

    // Check Content-Length if present (basic size validation)
    if let Some(content_length) = headers.get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                if length > MAX_UPLOAD_SIZE {
                    return Err((StatusCode::PAYLOAD_TOO_LARGE, axum::Json(ErrorResponse {
                        error: "PAYLOAD_TOO_LARGE".into(),
                        message: format!("File size {} bytes exceeds limit of {} bytes", length, MAX_UPLOAD_SIZE),
                        details: None,
                    })));
                }
            }
        }
    }

    Ok(next.run(req).await)
}

// Utility for handlers to validate multipart data
pub fn validate_file_data(filename: &str, data: &[u8], declared_type: Option<&str>) -> Result<(), String> {
    // Validate filename
    if filename.is_empty() || filename.len() > 255 {
        return Err("Invalid filename length".to_string());
    }
    
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Unsafe filename".to_string());
    }

    // Validate file size
    if data.len() > MAX_UPLOAD_SIZE {
        return Err(format!("File too large: {} bytes", data.len()));
    }

    // Validate magic bytes vs declared type
    if let Some(detected_type) = MimeValidator::validate_magic_bytes(data) {
        if let Some(declared) = declared_type {
            if !declared.starts_with(detected_type) {
                return Err(format!("Type mismatch: declared '{}', detected '{}'", declared, detected_type));
            }
        }
        
        if !MimeValidator::is_allowed(detected_type) {
            return Err(format!("File type '{}' not allowed", detected_type));
        }
    } else {
        return Err("Could not detect file type from content".to_string());
    }

    Ok(())
}

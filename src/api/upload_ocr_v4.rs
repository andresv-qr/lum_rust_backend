use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use uuid::Uuid;
use std::sync::Arc;
use serde_json::json;
use tracing::{info, warn, error};

use crate::{
    state::AppState,
    services::ocr_service::{OcrService, OcrProcessRequest, OcrSource, OcrMode},
    api::common::{ApiResponse, ApiError},
    middleware::auth::CurrentUser,
};

/// Upload OCR endpoint handler
/// POST /api/v4/invoices/upload-ocr
pub async fn upload_ocr_invoice(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Upload OCR request from user: {}", current_user.user_id);

    // Extract image and mode from multipart form
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mode: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        
        match field_name.as_str() {
            "image" | "file" => {
                let filename = field.file_name().map(|s| s.to_string());
                match field.bytes().await {
                    Ok(bytes) => {
                        image_bytes = Some(bytes.to_vec());
                        info!("Received image file: {} ({} bytes)", filename.as_deref().unwrap_or("unknown"), bytes.len());
                    }
                    Err(e) => {
                        error!("Error reading multipart field: {}", e);
                        let request_id = Uuid::new_v4().to_string();
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error(
                                ApiError {
                                    code: "FILE_READ_ERROR".to_string(),
                                    message: "Error reading uploaded file".to_string(),
                                    details: None,
                                },
                                request_id,
                            )),
                        ));
                    }
                }
            }
            "mode" => {
                match field.text().await {
                    Ok(text) => {
                        mode = Some(text);
                        info!("Received mode parameter: {}", mode.as_deref().unwrap_or("none"));
                    }
                    Err(e) => {
                        warn!("Error reading mode field: {}, using default mode 1", e);
                    }
                }
            }
            _ => {
                warn!("Unexpected field in multipart: {}", field_name);
            }
        }
    }

    // Validate that we received an image
    let image_data = match image_bytes {
        Some(data) => {
            if data.is_empty() {
                let request_id = Uuid::new_v4().to_string();
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        ApiError {
                            code: "NO_IMAGE_DATA".to_string(),
                            message: "No image data received".to_string(),
                            details: None,
                        },
                        request_id,
                    )),
                ));
            }
            if data.len() > 10 * 1024 * 1024 { // 10MB limit
                let request_id = Uuid::new_v4().to_string();
                return Err((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(ApiResponse::<()>::error(
                        ApiError {
                            code: "FILE_TOO_LARGE".to_string(),
                            message: "Image file too large (max 10MB)".to_string(),
                            details: None,
                        },
                        request_id,
                    )),
                ));
            }
            data
        }
        None => {
            let request_id = Uuid::new_v4().to_string();
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    ApiError {
                        code: "NO_IMAGE_FILE".to_string(),
                        message: "No image file provided. Use 'image' or 'file' field name.".to_string(),
                        details: None,
                    },
                    request_id,
                )),
            ));
        }
    };

    // Validate file type based on magic bytes (basic validation)
    if !is_valid_image_format(&image_data) {
        let request_id = Uuid::new_v4().to_string();
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Json(ApiResponse::<()>::error(
                ApiError {
                    code: "INVALID_FORMAT".to_string(),
                    message: "Invalid image format. Supported: JPEG, PNG, PDF".to_string(),
                    details: None,
                },
                request_id,
            )),
        ));
    }

    // Create OCR processing request
    let user_id = current_user.user_id;

    // Parse mode parameter (default to 1 if not provided or invalid)
    let ocr_mode = match mode.as_deref() {
        Some("2") => {
            info!("Using combined mode (2) for user {}", user_id);
            OcrMode::Combined
        }
        Some("1") | None => {
            info!("Using normal mode (1) for user {}", user_id);
            OcrMode::Normal
        }
        Some(invalid_mode) => {
            warn!("Invalid mode '{}' provided for user {}, defaulting to normal mode", invalid_mode, user_id);
            OcrMode::Normal
        }
    };

    let ocr_request = OcrProcessRequest {
        user_id,
        user_identifier: current_user.email.clone(), // Usar email en lugar de user_id para API calls
        image_bytes: image_data,
        source: OcrSource::Api,
        mode: ocr_mode,
    };

    // Process OCR using the common service
    match OcrService::process_ocr_invoice(state, ocr_request).await {
        Ok(ocr_response) => {
            if ocr_response.success {
                info!("✅ OCR processing successful for user {}: CUFE {}", 
                      current_user.user_id, ocr_response.cufe.as_deref().unwrap_or("unknown"));
                
                let response_data = json!({
                    "success": true,
                    "cufe": ocr_response.cufe,
                    "invoice_number": ocr_response.invoice_number,
                    "issuer_name": ocr_response.issuer_name,
                    "issuer_ruc": ocr_response.issuer_ruc,
                    "issuer_dv": ocr_response.issuer_dv,
                    "issuer_address": ocr_response.issuer_address,
                    "date": ocr_response.date,
                    "total": ocr_response.total,
                    "tot_itbms": ocr_response.tot_itbms,
                    "products": ocr_response.products,
                    "products_count": ocr_response.products.as_ref().map(|p| p.len()).unwrap_or(0),
                    "cost_lumis": ocr_response.cost_lumis,
                    "status": "pending_validation",
                    "message": ocr_response.message,
                    "missing_fields": ocr_response.missing_fields
                });

                let request_id = Uuid::new_v4().to_string();
                Ok(Json(ApiResponse::success(response_data, request_id, None, false)))
            } else {
                warn!("❌ OCR processing failed for user {}: {}", current_user.user_id, ocr_response.message);
                
                // Determine appropriate status code based on error type
                let status_code = if ocr_response.message.contains("ya fue registrada") || ocr_response.message.contains("duplicada") {
                    StatusCode::CONFLICT
                } else if ocr_response.message.contains("Saldo insuficiente") {
                    StatusCode::PAYMENT_REQUIRED
                } else if ocr_response.message.contains("límite") {
                    StatusCode::TOO_MANY_REQUESTS
                } else if ocr_response.message.contains("Validación fallida") 
                    || ocr_response.message.contains("campos requeridos")
                    || ocr_response.message.contains("Campos faltantes")
                    || ocr_response.message.contains("campos obligatorios")
                    || ocr_response.missing_fields.is_some() {
                    // Missing fields is a client-side issue (bad image quality), not a server error
                    StatusCode::OK  // Return 200 with success: false so frontend can handle gracefully
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };

                let error_data = json!({
                    "success": false,
                    "cost_lumis": ocr_response.cost_lumis,
                    "message": ocr_response.message,
                    "cufe": ocr_response.cufe,
                    "invoice_number": ocr_response.invoice_number,
                    "issuer_name": ocr_response.issuer_name,
                    "issuer_ruc": ocr_response.issuer_ruc,
                    "issuer_dv": ocr_response.issuer_dv,
                    "issuer_address": ocr_response.issuer_address,
                    "date": ocr_response.date,
                    "total": ocr_response.total,
                    "tot_itbms": ocr_response.tot_itbms,
                    "products": ocr_response.products,
                    "products_count": ocr_response.products.as_ref().map(|p| p.len()).unwrap_or(0),
                    "missing_fields": ocr_response.missing_fields,
                    "extracted_data": ocr_response.extracted_data
                });

                let error_code = if ocr_response.message.contains("ya fue registrada") || ocr_response.message.contains("duplicada") {
                    "DUPLICATE_INVOICE"
                } else if ocr_response.message.contains("Validación fallida") 
                    || ocr_response.message.contains("campos requeridos")
                    || ocr_response.message.contains("Campos faltantes")
                    || ocr_response.message.contains("campos obligatorios")
                    || ocr_response.missing_fields.is_some() {
                    "MISSING_FIELDS"
                } else if ocr_response.message.contains("límite") {
                    "RATE_LIMIT_EXCEEDED"
                } else {
                    "OCR_PROCESSING_FAILED"
                };

                // For missing fields, return success response with success:false so frontend handles it gracefully
                if error_code == "MISSING_FIELDS" {
                    let response_data = json!({
                        "success": false,
                        "cufe": ocr_response.cufe,
                        "invoice_number": ocr_response.invoice_number,
                        "issuer_name": ocr_response.issuer_name,
                        "issuer_ruc": ocr_response.issuer_ruc,
                        "issuer_dv": ocr_response.issuer_dv,
                        "issuer_address": ocr_response.issuer_address,
                        "date": ocr_response.date,
                        "total": ocr_response.total,
                        "tot_itbms": ocr_response.tot_itbms,
                        "products": ocr_response.products,
                        "products_count": ocr_response.products.as_ref().map(|p| p.len()).unwrap_or(0),
                        "cost_lumis": ocr_response.cost_lumis,
                        "status": "missing_fields",
                        "message": ocr_response.message,
                        "missing_fields": ocr_response.missing_fields,
                        "extracted_data": ocr_response.extracted_data
                    });
                    let request_id = Uuid::new_v4().to_string();
                    return Ok(Json(ApiResponse::success(response_data, request_id, None, false)));
                }

                let request_id = Uuid::new_v4().to_string();
                Err((status_code, Json(ApiResponse::<()>::error(
                    ApiError {
                        code: error_code.to_string(),
                        message: ocr_response.message.clone(),
                        details: Some(error_data),
                    },
                    request_id,
                ))))
            }
        }
        Err(e) => {
            error!("💥 Critical error in OCR processing for user {}: {}", current_user.user_id, e);
            let request_id = Uuid::new_v4().to_string();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    ApiError {
                        code: "INTERNAL_ERROR".to_string(),
                        message: "Internal server error during OCR processing".to_string(),
                        details: None,
                    },
                    request_id,
                )),
            ))
        }
    }
}

/// Basic image format validation using magic bytes
fn is_valid_image_format(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    // Check magic bytes for common formats
    match &data[0..4] {
        // JPEG
        [0xFF, 0xD8, 0xFF, _] => true,
        // PNG
        [0x89, 0x50, 0x4E, 0x47] => true,
        // PDF
        [0x25, 0x50, 0x44, 0x46] => true,
        _ => false,
    }
}
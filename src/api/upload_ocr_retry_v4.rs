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
    services::ocr_service::{OcrService, OcrRetryRequest, ExtractedOcrData},
    api::common::{ApiResponse, ApiError},
    middleware::auth::CurrentUser,
};

/// Upload OCR Retry endpoint handler
/// POST /api/v4/invoices/upload-ocr-retry
/// 
/// Este endpoint permite reintentar la extracción de campos específicos
/// que no se pudieron detectar en la primera imagen.
/// 
/// Campos en multipart form:
/// - image/file: Nueva imagen de la factura (enfocada en campos faltantes)
/// - missing_fields: JSON array de field_keys a buscar (ej: ["ruc", "dv", "products"])
/// - previous_data: JSON object con datos extraídos previamente (de extracted_data del primer OCR)
pub async fn upload_ocr_retry(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("📸 Upload OCR RETRY request from user: {}", current_user.user_id);

    // Extract image, missing_fields and previous_data from multipart form
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut missing_fields_json: Option<String> = None;
    let mut previous_data_json: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        
        match field_name.as_str() {
            "image" | "file" => {
                let filename = field.file_name().map(|s| s.to_string());
                match field.bytes().await {
                    Ok(bytes) => {
                        image_bytes = Some(bytes.to_vec());
                        info!("📷 Received retry image file: {} ({} bytes)", filename.as_deref().unwrap_or("unknown"), bytes.len());
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
            "missing_fields" => {
                match field.text().await {
                    Ok(text) => {
                        missing_fields_json = Some(text);
                        info!("🔍 Received missing_fields parameter");
                    }
                    Err(e) => {
                        warn!("Error reading missing_fields field: {}", e);
                    }
                }
            }
            "previous_data" => {
                match field.text().await {
                    Ok(text) => {
                        previous_data_json = Some(text);
                        info!("📦 Received previous_data parameter");
                    }
                    Err(e) => {
                        warn!("Error reading previous_data field: {}", e);
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

    // Validate file type based on magic bytes
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

    // Validate missing_fields parameter
    let missing_fields: Vec<String> = match missing_fields_json {
        Some(json_str) => {
            match serde_json::from_str::<Vec<String>>(&json_str) {
                Ok(fields) => {
                    if fields.is_empty() {
                        let request_id = Uuid::new_v4().to_string();
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error(
                                ApiError {
                                    code: "EMPTY_MISSING_FIELDS".to_string(),
                                    message: "missing_fields array cannot be empty".to_string(),
                                    details: Some(json!({
                                        "valid_fields": ["ruc", "dv", "invoice_number", "total", "products"],
                                        "example": "[\"ruc\", \"products\"]"
                                    })),
                                },
                                request_id,
                            )),
                        ));
                    }
                    
                    // Validar que los campos sean válidos
                    let valid_fields = ["ruc", "dv", "invoice_number", "total", "products"];
                    for field in &fields {
                        if !valid_fields.contains(&field.as_str()) {
                            let request_id = Uuid::new_v4().to_string();
                            return Err((
                                StatusCode::BAD_REQUEST,
                                Json(ApiResponse::<()>::error(
                                    ApiError {
                                        code: "INVALID_FIELD_KEY".to_string(),
                                        message: format!("Invalid field_key: '{}'. Valid options: {:?}", field, valid_fields),
                                        details: Some(json!({
                                            "invalid_field": field,
                                            "valid_fields": valid_fields
                                        })),
                                    },
                                    request_id,
                                )),
                            ));
                        }
                    }
                    
                    info!("🎯 Looking for specific fields: {:?}", fields);
                    fields
                }
                Err(e) => {
                    let request_id = Uuid::new_v4().to_string();
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::<()>::error(
                            ApiError {
                                code: "INVALID_MISSING_FIELDS_FORMAT".to_string(),
                                message: format!("missing_fields must be a JSON array: {}", e),
                                details: Some(json!({
                                    "expected_format": "[\"ruc\", \"dv\", \"products\"]",
                                    "received": json_str
                                })),
                            },
                            request_id,
                        )),
                    ));
                }
            }
        }
        None => {
            let request_id = Uuid::new_v4().to_string();
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    ApiError {
                        code: "MISSING_FIELDS_REQUIRED".to_string(),
                        message: "missing_fields parameter is required for retry endpoint".to_string(),
                        details: Some(json!({
                            "valid_fields": ["ruc", "dv", "invoice_number", "total", "products"],
                            "example": "[\"ruc\", \"products\"]"
                        })),
                    },
                    request_id,
                )),
            ));
        }
    };

    // Parse previous_data (optional but recommended)
    let previous_data: Option<ExtractedOcrData> = match previous_data_json {
        Some(json_str) => {
            match serde_json::from_str::<ExtractedOcrData>(&json_str) {
                Ok(data) => {
                    info!("📦 Previous data parsed successfully:");
                    info!("  RUC: {:?}", data.ruc);
                    info!("  DV: {:?}", data.dv);
                    info!("  Invoice: {:?}", data.invoice_number);
                    info!("  Total: {:?}", data.total);
                    info!("  Products: {}", data.products.len());
                    Some(data)
                }
                Err(e) => {
                    warn!("⚠️ Could not parse previous_data (will proceed without): {}", e);
                    None
                }
            }
        }
        None => {
            info!("📦 No previous_data provided - retry will only extract new data");
            None
        }
    };

    let user_id = current_user.user_id;
    let retry_request = OcrRetryRequest { 
        missing_fields: missing_fields.clone(),
        previous_data,
    };

    // Process OCR retry using the specialized method
    match OcrService::process_ocr_retry(state, user_id, current_user.email.clone(), image_data, retry_request).await {
        Ok(ocr_response) => {
            if ocr_response.success {
                info!("✅ OCR RETRY successful for user {}: all fields complete!", user_id);
                
                let response_data = json!({
                    "success": true,
                    "retry_mode": true,
                    "searched_fields": missing_fields,
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
                    "message": ocr_response.message,
                    "missing_fields": ocr_response.missing_fields,
                    "extracted_data": ocr_response.extracted_data
                });

                let request_id = Uuid::new_v4().to_string();
                Ok(Json(ApiResponse::success(response_data, request_id, None, false)))
            } else {
                warn!("❌ OCR RETRY incomplete for user {}: {}", user_id, ocr_response.message);
                
                let error_data = json!({
                    "success": false,
                    "retry_mode": true,
                    "searched_fields": missing_fields,
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

                let status_code = StatusCode::UNPROCESSABLE_ENTITY;

                let request_id = Uuid::new_v4().to_string();
                Err((status_code, Json(ApiResponse::<()>::error(
                    ApiError {
                        code: "RETRY_EXTRACTION_INCOMPLETE".to_string(),
                        message: ocr_response.message.clone(),
                        details: Some(error_data),
                    },
                    request_id,
                ))))
            }
        }
        Err(e) => {
            error!("💥 Critical error in OCR retry for user {}: {}", user_id, e);
            let request_id = Uuid::new_v4().to_string();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    ApiError {
                        code: "INTERNAL_ERROR".to_string(),
                        message: "Internal server error during OCR retry processing".to_string(),
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

    match &data[0..4] {
        [0xFF, 0xD8, 0xFF, _] => true,  // JPEG
        [0x89, 0x50, 0x4E, 0x47] => true, // PNG
        [0x25, 0x50, 0x44, 0x46] => true, // PDF
        _ => false,
    }
}

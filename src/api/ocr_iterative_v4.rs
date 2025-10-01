use axum::{
    extract::{State, Multipart},
    http::{StatusCode, HeaderMap},
    response::Json,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use base64::{engine::general_purpose, Engine as _};

use crate::{
    models::ocr::*,
    services::{ocr_session_service::*, ocr_processing_service::*},
    state::AppState,
    middleware::auth::extract_user_from_headers,
};

/// POST /api/v4/invoices/ocr-process
/// Procesa imágenes de facturas de manera iterativa hasta obtener todos los campos requeridos
pub async fn process_ocr_iterative(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<OcrProcessResponse>, StatusCode> {
    // Get current user from headers
    let current_user = extract_user_from_headers(&headers)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    info!("🔍 Iniciando procesamiento OCR iterativo para usuario {}", current_user.user_id);
    
    let _start_time = std::time::Instant::now();
    let mut session_id: Option<String> = None;
    let mut action = OcrAction::Initial;
    let mut missing_fields: Option<Vec<String>> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mime_type = "image/jpeg".to_string();
    
    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Error parsing multipart: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "session_id" => {
                session_id = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "action" => {
                let action_str = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                action = serde_json::from_str(&format!("\"{}\"", action_str))
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
            }
            "missing_fields" => {
                let fields_str = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                missing_fields = serde_json::from_str(&fields_str).ok();
            }
            "image" => {
                // Get content type
                if let Some(content_type) = field.content_type() {
                    mime_type = content_type.to_string();
                }
                
                // Read image bytes
                let bytes = field.bytes().await.map_err(|e| {
                    error!("Error reading image bytes: {}", e);
                    StatusCode::BAD_REQUEST
                })?;
                
                image_bytes = Some(bytes.to_vec());
            }
            _ => {
                // Skip unknown fields
                let _ = field.bytes().await;
            }
        }
    }
    
    // Validate required fields
    let image_data = image_bytes.ok_or_else(|| {
        warn!("Imagen no proporcionada");
        StatusCode::BAD_REQUEST
    })?;
    
    // Validate image size (max 10MB)
    if image_data.len() > 10 * 1024 * 1024 {
        warn!("Imagen demasiado grande: {} bytes", image_data.len());
        return Ok(Json(OcrProcessResponse {
            success: false,
            session_id: session_id.unwrap_or_default(),
            attempt_count: 0,
            max_attempts: 5,
            status: "failed".to_string(),
            detected_fields: InvoiceData::empty(),
            missing_fields: vec![],
            consolidated_image: None,
            message: "Imagen demasiado grande. Máximo 10MB.".to_string(),
            cost: OcrCostInfo { lumis_used: 0, tokens_used: 0 },
        }));
    }
    
    // Get or create session
    let mut session = match action {
        OcrAction::Initial => {
            // Create new session
            OcrSessionService::create_session(&state, current_user.user_id).await.map_err(|e| {
                error!("Error creando sesión OCR: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
        OcrAction::Retry | OcrAction::Consolidate => {
            // Get existing session
            let sid = session_id.ok_or_else(|| {
                warn!("session_id requerido para retry/consolidate");
                StatusCode::BAD_REQUEST
            })?;
            
            OcrSessionService::get_session(&state, &sid).await.map_err(|e| {
                error!("Error obteniendo sesión: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?.ok_or_else(|| {
                warn!("Sesión no encontrada: {}", sid);
                StatusCode::NOT_FOUND
            })?
        }
    };
    
    // Verify session belongs to user
    if session.user_id != current_user.user_id {
        warn!("Usuario {} intentando acceder sesión de usuario {}", current_user.user_id, session.user_id);
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Check if session has reached max attempts
    if session.attempt_count >= session.max_attempts {
        return Ok(Json(OcrProcessResponse {
            success: false,
            session_id: session.session_id.clone(),
            attempt_count: session.attempt_count,
            max_attempts: session.max_attempts,
            status: "manual_review".to_string(),
            detected_fields: session.detected_fields.clone(),
            missing_fields: session.missing_fields.clone(),
            consolidated_image: session.consolidated_image.clone(),
            message: "Se alcanzó el límite de intentos. Esta factura será revisada por nuestro equipo.".to_string(),
            cost: OcrCostInfo { lumis_used: 0, tokens_used: 0 },
        }));
    }
    
    // Handle consolidate action
    if matches!(action, OcrAction::Consolidate) {
        let consolidated_image = OcrSessionService::consolidate_images(&state, &session.session_id).await.map_err(|e| {
            error!("Error consolidando imágenes: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        return Ok(Json(OcrProcessResponse {
            success: true,
            session_id: session.session_id.clone(),
            attempt_count: session.attempt_count,
            max_attempts: session.max_attempts,
            status: "complete".to_string(),
            detected_fields: session.detected_fields.clone(),
            missing_fields: vec![],
            consolidated_image: Some(consolidated_image),
            message: "Imágenes consolidadas exitosamente.".to_string(),
            cost: OcrCostInfo { lumis_used: 0, tokens_used: 0 },
        }));
    }
    
    // Generate appropriate prompt
    let prompt = if matches!(action, OcrAction::Initial) {
        OcrPromptGenerator::generate_initial_prompt()
    } else {
        let focus_fields = missing_fields.clone().unwrap_or_else(|| session.missing_fields.clone());
        OcrPromptGenerator::generate_focused_prompt(&focus_fields, &session.detected_fields)
    };
    
    // Process image with OCR
    let detected_data = match OcrProcessingService::process_image_with_gemini(&image_data, Some(vec![prompt])).await {
        Ok(data) => data,
        Err(e) => {
            error!("Error en procesamiento OCR: {}", e);
            
            // Log failed attempt
            let _ = OcrProcessingService::log_ocr_processing(
                &state,
                current_user.user_id,
                0,
                0.02,
                false,
                "ocr_iterative_v4",
            ).await;
            
            return Ok(Json(OcrProcessResponse {
                success: false,
                session_id: session.session_id.clone(),
                attempt_count: session.attempt_count,
                max_attempts: session.max_attempts,
                status: "failed".to_string(),
                detected_fields: session.detected_fields.clone(),
                missing_fields: session.missing_fields.clone(),
                consolidated_image: None,
                message: format!("Error procesando imagen: {}", e),
                cost: OcrCostInfo { lumis_used: 0, tokens_used: 1000 }, // Estimate
            }));
        }
    };
    
    // Add attempt to session
    let focus_fields = if matches!(action, OcrAction::Retry) {
        missing_fields.clone()
    } else {
        None
    };
    
    session = OcrSessionService::add_attempt(
        &state,
        &session.session_id,
        &image_data,
        &mime_type,
        detected_data,
        focus_fields,
    ).await.map_err(|e| {
        error!("Error agregando intento a sesión: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Determine response status and message
    let (status, message) = match session.state {
        OcrSessionState::Complete => {
            ("complete".to_string(), "Todos los campos fueron detectados correctamente.".to_string())
        }
        OcrSessionState::NeedsRetry { ref missing_fields } => {
            let fields_description = missing_fields.iter()
                .map(|f| get_field_display_name(f))
                .collect::<Vec<_>>()
                .join(", ");
            (
                "needs_retry".to_string(),
                format!("Faltan campos: {}. Sube una foto enfocando estas áreas.", fields_description)
            )
        }
        OcrSessionState::ManualReview => {
            ("manual_review".to_string(), "Se alcanzó el límite de intentos. Esta factura será revisada por nuestro equipo.".to_string())
        }
        _ => {
            ("processing".to_string(), "Procesando...".to_string())
        }
    };
    
    // Generate consolidated image if complete
    let consolidated_image = if session.is_complete() {
        match OcrSessionService::consolidate_images(&state, &session.session_id).await {
            Ok(img) => Some(img),
            Err(e) => {
                warn!("Error consolidando imágenes: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Log successful attempt
    let _ = OcrProcessingService::log_ocr_processing(
        &state,
        current_user.user_id,
        1200, // Estimate tokens used
        0.05,
        true,
        "ocr_iterative_v4",
    ).await;
    
    Ok(Json(OcrProcessResponse {
        success: true,
        session_id: session.session_id.clone(),
        attempt_count: session.attempt_count,
        max_attempts: session.max_attempts,
        status,
        detected_fields: session.detected_fields.clone(),
        missing_fields: session.missing_fields.clone(),
        consolidated_image,
        message,
        cost: OcrCostInfo { 
            lumis_used: 0, // Free during testing
            tokens_used: 1200, // Estimate
        },
    }))
}

/// POST /api/v4/invoices/save-ocr
/// Guarda los datos de factura procesados con OCR
pub async fn save_ocr_invoice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(save_request): Json<SaveOcrRequest>,
) -> Result<Json<SaveOcrResponse>, StatusCode> {
    // Get current user from headers
    let current_user = extract_user_from_headers(&headers)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    info!("💾 Guardando factura OCR para usuario {}", current_user.user_id);
    
    let _start_time = std::time::Instant::now();
    
    // Get and validate session
    let session = OcrSessionService::get_session(&state, &save_request.session_id).await.map_err(|e| {
        error!("Error obteniendo sesión: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?.ok_or_else(|| {
        warn!("Sesión no encontrada: {}", save_request.session_id);
        StatusCode::NOT_FOUND
    })?;
    
    // Verify session belongs to user
    if session.user_id != current_user.user_id {
        warn!("Usuario {} intentando acceder sesión de usuario {}", current_user.user_id, session.user_id);
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Validate invoice data
    if let Err(e) = OcrProcessingService::validate_required_fields(&save_request.invoice_data) {
        warn!("Datos de factura inválidos: {}", e);
        return Ok(Json(SaveOcrResponse {
            success: false,
            invoice_id: None,
            cufe: None,
            status: "validation_error".to_string(),
            message: format!("Datos inválidos: {}", e),
            rewards: None,
            next_steps: vec!["Revisa los datos y intenta nuevamente.".to_string()],
        }));
    }
    
    // Check for duplicates
    if let Some(existing_cufe) = OcrProcessingService::check_duplicate_invoice(&state, &save_request.invoice_data).await.map_err(|e| {
        error!("Error verificando duplicados: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
        return Ok(Json(SaveOcrResponse {
            success: false,
            invoice_id: None,
            cufe: Some(existing_cufe.clone()),
            status: "duplicate".to_string(),
            message: "Esta factura ya fue registrada previamente.".to_string(),
            rewards: None,
            next_steps: vec![format!("CUFE existente: {}", existing_cufe)],
        }));
    }
    
    // Generate CUFE
    let cufe = OcrProcessingService::generate_cufe(&save_request.invoice_data);
    
    // Decode consolidated image
    let _image_data = general_purpose::STANDARD.decode(&save_request.consolidated_image).map_err(|e| {
        error!("Error decodificando imagen: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    
    // Save to database
    let invoice_id = OcrProcessingService::save_invoice_to_database(
        &state,
        &save_request.invoice_data,
        &cufe,
        current_user.user_id,
    ).await.map_err(|e| {
        error!("Error guardando en base de datos: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Log successful save
    let _ = OcrProcessingService::log_ocr_processing(
        &state,
        current_user.user_id,
        0,
        0.05,
        true,
        "save_ocr_invoice",
    ).await;
    
    // Clean up session
    let _ = OcrSessionService::delete_session(&state, &save_request.session_id).await;
    
    // Determine status and next steps based on validation_status
    let (status, next_steps) = match save_request.validation_status {
        ValidationStatus::Complete => {
            ("pending_validation".to_string(), vec![
                "La factura será validada por nuestro equipo en 24-48 horas.".to_string(),
                "Recibirás una notificación cuando esté certificada.".to_string(),
                "Los Lümis se acreditarán automáticamente.".to_string(),
            ])
        }
        ValidationStatus::ManualReview => {
            ("manual_review".to_string(), vec![
                "Esta factura requiere revisión manual detallada.".to_string(),
                "Nuestro equipo se pondrá en contacto si necesita más información.".to_string(),
                "El proceso puede tomar 2-5 días hábiles.".to_string(),
            ])
        }
    };
    
    Ok(Json(SaveOcrResponse {
        success: true,
        invoice_id: Some(invoice_id as i64),
        cufe: Some(cufe.clone()),
        status,
        message: "Factura guardada exitosamente.".to_string(),
        rewards: Some(RewardsInfo {
            lumis_earned: 0,
            xp_earned: 0,
            note: "Los Lümis se otorgarán después de la validación.".to_string(),
        }),
        next_steps,
    }))
}

// Helper functions
fn get_field_display_name(field: &str) -> &'static str {
    match field {
        "issuer_name" => "nombre del comercio",
        "invoice_number" => "número de factura",
        "date" => "fecha",
        "total" => "total",
        "products" => "productos",
        _ => "campo desconocido",
    }
}

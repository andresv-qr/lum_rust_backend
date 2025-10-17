use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use std::sync::Arc;
use tracing::{info, error, warn};
use uuid::Uuid;

use crate::api::common::{ApiError, ApiResponse};
use crate::api::webscraping::scrape_invoice;
use crate::api::database_persistence::persist_scraped_data;
use crate::api::templates::url_processing_templates::ProcessUrlResponse;
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;
use crate::models::invoice::MefPending;
use crate::shared::database as db_service;

// CORRECTED: Added optional fields from user to match database schema
#[derive(serde::Deserialize)]
pub struct UrlRequest {
    pub url: String,
    
    // Optional fields from user (will be added to invoice_header)
    #[serde(rename = "type")]
    pub type_field: Option<String>, // "QR" or "CUFE"
    pub origin: Option<String>, // "app", "whatsapp", "telegram"
    pub user_email: Option<String>,
    pub user_phone_number: Option<String>,
    pub user_telegram_id: Option<String>,
    pub user_ws: Option<String>,
}

#[axum::debug_handler]
pub async fn process_url_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<UrlRequest>,
) -> Result<Json<ApiResponse<ProcessUrlResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let user_id = current_user.user_id;  // Extract user_id from JWT for security
    
    info!("Processing URL request for user {}: {}", user_id, request.url);
    
    if request.url.trim().is_empty() {
        return Err(ApiError::validation_error("URL is required"));
    }

    // Scrape the invoice
    match scrape_invoice(&state.http_client, &request.url, user_id).await {
        Ok(mut scraping_result) => {
            // Populate user fields in the header from request
            if let Some(ref mut header) = scraping_result.header {
                header.user_id = user_id;
                header.type_field = request.type_field.clone().unwrap_or_default();
                header.origin = request.origin.clone().unwrap_or_default();
                header.user_email = request.user_email.clone();
                header.user_phone_number = request.user_phone_number.clone();
                header.user_telegram_id = request.user_telegram_id.clone();
                header.user_ws = request.user_ws.clone();
            }
            
            // Save to database
            let db_result = persist_scraped_data(&state.db_pool, scraping_result.clone(), &request.url).await;
            
            let execution_time = start_time.elapsed().as_millis() as u64;
            
            match db_result {
                Ok(mut process_response) => {
                    // 🆕 GAMIFICACIÓN: Acreditar Lumis por procesar factura
                    // Este flujo replica la implementación de Python/WhatsApp
                    if let Some(ref cufe) = process_response.cufe {
                        match crate::api::gamification_service::credit_lumis_for_invoice(
                            &state.db_pool,
                            user_id,
                            cufe
                        ).await {
                            Ok(lumis_result) => {
                                // Actualizar respuesta con información de Lumis
                                process_response.lumis_earned = Some(lumis_result.lumis_earned);
                                process_response.lumis_balance = Some(lumis_result.lumis_balance);
                                
                                // Agregar mensaje de Lumis a la respuesta
                                let lumis_message = format!(
                                    "\n\n¡Has ganado {} Lümis! 🌟 Tu nuevo balance es {} Lümis.",
                                    lumis_result.lumis_earned,
                                    lumis_result.lumis_balance
                                );
                                
                                process_response.message.push_str(&lumis_message);
                                
                                info!(
                                    "✅ Successfully credited {} Lumis to user {}. New balance: {}",
                                    lumis_result.lumis_earned,
                                    user_id,
                                    lumis_result.lumis_balance
                                );
                            },
                            Err(e) => {
                                warn!("⚠️ Failed to credit Lumis for user {}: {}", user_id, e);
                                // No fallar el request, la factura ya se guardó exitosamente
                            }
                        }
                    }
                    
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
                    // Check if this is a duplicate invoice error - if so, don't save to mef_pending
                    if error_response.message.contains("duplicada") || error_response.message.contains("duplicate") {
                        warn!("⚠️ Factura duplicada detectada - no se guarda en mef_pending");
                        let response = ApiResponse {
                            success: false,
                            data: Some(error_response),
                            error: None,
                            request_id,
                            timestamp: chrono::Utc::now(),
                            execution_time_ms: Some(execution_time),
                            cached: false,
                        };
                        return Ok(Json(response));
                    }
                    
                    // FALLBACK: Save to mef_pending when database persistence fails (not duplicate)
                    warn!("❌ Error al guardar factura: '{}'. Guardando en mef_pending para revisión manual.", error_response.message);
                    
                    let mut tx = match state.db_pool.begin().await {
                        Ok(tx) => tx,
                        Err(e) => {
                            error!("Failed to start transaction for mef_pending: {}", e);
                            // Return original error if we can't even start transaction
                            let response = ApiResponse {
                                success: false,
                                data: Some(error_response),
                                error: None,
                                request_id,
                                timestamp: chrono::Utc::now(),
                                execution_time_ms: Some(execution_time),
                                cached: false,
                            };
                            return Ok(Json(response));
                        }
                    };
                    
                    let pending_entry = MefPending {
                        id: 0,
                        url: Some(request.url.clone()),
                        chat_id: request.user_ws.clone(),
                        reception_date: Some(chrono::Utc::now()),
                        message_id: None,
                        type_document: Some(request.type_field.clone().unwrap_or_else(|| "URL".to_string())),
                        user_email: request.user_email.clone(),
                        user_id: Some(user_id),
                        error_message: Some(error_response.message.clone()),
                        origin: Some(request.origin.clone().unwrap_or_else(|| "API".to_string())),
                        ws_id: request.user_ws.clone(),
                    };
                    
                    match db_service::save_to_mef_pending(&mut tx, &pending_entry).await {
                        Ok(_) => {
                            if let Err(e) = tx.commit().await {
                                error!("Failed to commit mef_pending transaction: {}", e);
                            } else {
                                info!("✅ Factura guardada en mef_pending para revisión manual (user_id: {})", user_id);
                            }
                        }
                        Err(e) => {
                            error!("Failed to save to mef_pending: {}", e);
                        }
                    }
                    
                    // Return error response to client
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
            // FALLBACK: Save to mef_pending when scraping fails
            error!("❌ Error de scraping: {}. Guardando en mef_pending.", e);
            
            let execution_time = start_time.elapsed().as_millis() as u64;
            
            let mut tx = match state.db_pool.begin().await {
                Ok(tx) => tx,
                Err(tx_error) => {
                    error!("Failed to start transaction for mef_pending: {}", tx_error);
                    return Err(ApiError::new("SCRAPING_ERROR", &format!("Error al extraer datos de la factura: {}", e)));
                }
            };
            
            let pending_entry = MefPending {
                id: 0,
                url: Some(request.url.clone()),
                chat_id: request.user_ws.clone(),
                reception_date: Some(chrono::Utc::now()),
                message_id: None,
                type_document: Some(request.type_field.clone().unwrap_or_else(|| "URL".to_string())),
                user_email: request.user_email.clone(),
                user_id: Some(user_id),
                error_message: Some(format!("Scraping error: {}", e)),
                origin: Some(request.origin.clone().unwrap_or_else(|| "API".to_string())),
                ws_id: request.user_ws.clone(),
            };
            
            match db_service::save_to_mef_pending(&mut tx, &pending_entry).await {
                Ok(_) => {
                    if let Err(commit_error) = tx.commit().await {
                        error!("Failed to commit mef_pending transaction: {}", commit_error);
                    } else {
                        info!("✅ Error de scraping guardado en mef_pending para revisión manual (user_id: {})", user_id);
                    }
                }
                Err(save_error) => {
                    error!("Failed to save to mef_pending: {}", save_error);
                }
            }
            
            // Return user-friendly error
            let error_response = ProcessUrlResponse::error("No pudimos procesar la factura automáticamente. Nuestro equipo la revisará manualmente y te notificaremos cuando esté lista.");
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

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/process-from-url", post(process_url_handler))
}

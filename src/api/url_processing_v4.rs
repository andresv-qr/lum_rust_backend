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

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Categorizes scraping errors and returns appropriate user-facing message
fn categorize_scraping_error(error: &str) -> &'static str {
    let error_lower = error.to_lowercase();
    
    // Check for captcha/session issues (admin needs to refresh)
    if error_lower.contains("captcha") || 
       error_lower.contains("recaptcha") ||
       error_lower.contains("robot") {
        "El servicio de verificación de facturas está temporalmente no disponible. Por favor intenta más tarde."
    }
    // Check for session expired
    else if error_lower.contains("session") && 
            (error_lower.contains("expired") || error_lower.contains("invalid")) {
        "El servicio de verificación de facturas está temporalmente no disponible. Por favor intenta más tarde."
    }
    // Check for "factura no disponible" scenarios
    else if error_lower.contains("404") || 
       error_lower.contains("not found") ||
       error_lower.contains("no encontrado") ||
       error_lower.contains("no disponible") ||
       error_lower.contains("dgi_no_data") {
        "Tu factura ha sido recibida. Aún no está disponible para ser procesada. Te notificaremos cuando esté lista."
    }
    // Check for network/timeout issues
    else if error_lower.contains("timeout") || 
            error_lower.contains("connection") ||
            error_lower.contains("timed out") ||
            error_lower.contains("network") {
        "Hubo un problema temporal de conexión. Tu factura se procesará automáticamente en segundo plano."
    }
    // Check for parsing/extraction issues
    else if error_lower.contains("parse") || 
            error_lower.contains("extract") ||
            error_lower.contains("invalid html") {
        "No pudimos extraer los datos de la factura. Nuestro equipo la revisará manualmente y te notificaremos."
    }
    // Generic fallback
    else {
        "Tu factura ha sido recibida. Aún no está disponible para ser procesada. Te notificaremos cuando esté lista."
    }
}

// ============================================================================
// CUFE-SPECIFIC EXTRACTOR (different HTML structure from QR)
// ============================================================================

use std::collections::HashMap;
use regex::Regex;

/// Extracts invoice data from DGI CUFE API HTML response using scraper.
/// Uses the document parsed by scraper::Html for reliable dt/dd extraction.
fn extract_cufe_invoice_data_from_document(document: &scraper::Html) -> HashMap<String, String> {
    let mut data = HashMap::new();
    
    // Find all dt/dd pairs
    let dt_selector = scraper::Selector::parse("dt").unwrap();
    let dd_selector = scraper::Selector::parse("dd").unwrap();
    
    let dts: Vec<_> = document.select(&dt_selector).collect();
    let dds: Vec<_> = document.select(&dd_selector).collect();
    
    // Track which prefix we're on (first set is EMISOR, second is RECEPTOR)
    let mut emisor_done = false;
    
    for (dt, dd) in dts.iter().zip(dds.iter()) {
        let key = dt.text().collect::<String>().trim().to_lowercase();
        let value = dd.text().collect::<String>().trim().to_string();
        
        match key.as_str() {
            "ruc" => {
                if !emisor_done && data.get("emisor_ruc").is_none() {
                    data.insert("emisor_ruc".to_string(), value);
                } else {
                    data.insert("receptor_ruc".to_string(), value);
                    emisor_done = true;
                }
            }
            "dv" => {
                if !emisor_done && data.get("emisor_dv").is_none() {
                    if !value.is_empty() {
                        data.insert("emisor_dv".to_string(), value);
                    }
                } else if !value.is_empty() {
                    data.insert("receptor_dv".to_string(), value);
                }
            }
            "nombre" => {
                if !emisor_done && data.get("emisor_name").is_none() {
                    if !value.is_empty() {
                        data.insert("emisor_name".to_string(), value);
                    }
                } else if !value.is_empty() {
                    data.insert("receptor_name".to_string(), value);
                    emisor_done = true;
                }
            }
            s if s.contains("dirección") || s.contains("direccion") => {
                if !emisor_done && data.get("emisor_address").is_none() {
                    if !value.is_empty() {
                        data.insert("emisor_address".to_string(), value);
                    }
                } else if !value.is_empty() {
                    data.insert("receptor_address".to_string(), value);
                }
            }
            s if s.contains("teléfono") || s.contains("telefono") => {
                if !emisor_done && data.get("emisor_phone").is_none() {
                    if !value.is_empty() {
                        data.insert("emisor_phone".to_string(), value);
                    }
                } else if !value.is_empty() {
                    data.insert("receptor_phone".to_string(), value);
                }
            }
            _ => {}
        }
    }
    
    // Extract totals from tfoot
    let tfoot_selector = scraper::Selector::parse("tfoot tr").unwrap();
    for row in document.select(&tfoot_selector) {
        let text = row.text().collect::<String>();
        let text_lower = text.to_lowercase();
        
        if text_lower.contains("valor total") {
            if let Some(amount) = extract_amount_from_row(&row) {
                data.insert("tot_amount".to_string(), amount);
            }
        }
        if text_lower.contains("itbms total") {
            if let Some(amount) = extract_amount_from_row(&row) {
                data.insert("tot_itbms".to_string(), amount);
            }
        }
        if text_lower.contains("total pagado") {
            if let Some(amount) = extract_amount_from_row(&row) {
                data.insert("total_pagado".to_string(), amount);
            }
        }
    }
    
    // Extract invoice number and date from panel-heading h5 elements
    let h5_selector = scraper::Selector::parse("div.panel-heading h5").unwrap();
    for h5 in document.select(&h5_selector) {
        let text = h5.text().collect::<String>().trim().to_string();
        
        // Invoice number: "No. 0000587962"
        if text.contains("No.") {
            if let Ok(re) = Regex::new(r"No\.\s*(\d+)") {
                if let Some(caps) = re.captures(&text) {
                    if let Some(no) = caps.get(1) {
                        data.insert("no".to_string(), no.as_str().to_string());
                    }
                }
            }
        }
        
        // Date: "30/11/2025 12:26:56"
        if text.contains("/") && text.contains(":") {
            if let Ok(re) = Regex::new(r"(\d{2}/\d{2}/\d{4}\s+\d{2}:\d{2}:\d{2})") {
                if let Some(caps) = re.captures(&text) {
                    if let Some(date) = caps.get(1) {
                        data.insert("date".to_string(), date.as_str().to_string());
                    }
                }
            }
        }
    }
    
    // Extract authorization date from events table or dd
    let td_selector = scraper::Selector::parse("td").unwrap();
    for td in document.select(&td_selector) {
        let text = td.text().collect::<String>();
        if text.contains("/") && text.contains(":") && text.contains("Autorización") {
            if let Ok(re) = Regex::new(r"(\d{2}/\d{2}/\d{4}\s+\d{2}:\d{2}:\d{2})") {
                if let Some(caps) = re.captures(&text) {
                    if let Some(date) = caps.get(1) {
                        if data.get("auth_date").is_none() {
                            data.insert("auth_date".to_string(), date.as_str().to_string());
                        }
                    }
                }
            }
        }
    }
    
    // Also try to get auth date from FECHA AUTORIZACIÓN dd
    for (dt, dd) in dts.iter().zip(dds.iter()) {
        let key = dt.text().collect::<String>().trim().to_lowercase();
        if key.contains("fecha autorización") || key.contains("fecha autorizacion") {
            let value = dd.text().collect::<String>().trim().to_string();
            if !value.is_empty() && data.get("auth_date").is_none() {
                data.insert("auth_date".to_string(), value);
            }
        }
        // Also get protocol
        if key.contains("protocolo") {
            let value = dd.text().collect::<String>().trim().to_string();
            if !value.is_empty() {
                data.insert("auth_protocol".to_string(), value);
            }
        }
    }
    
    data
}

/// Helper to extract amount from tfoot row (looks for div with number inside)
fn extract_amount_from_row(row: &scraper::ElementRef) -> Option<String> {
    let div_selector = scraper::Selector::parse("div").unwrap();
    for div in row.select(&div_selector) {
        let text = div.text().collect::<String>().trim().to_string();
        // Check if it looks like a number
        if text.chars().any(|c| c.is_ascii_digit()) && 
           text.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',') {
            return Some(text);
        }
    }
    None
}

/// Legacy function for backward compatibility - calls the scraper version
fn extract_cufe_invoice_data(html: &str) -> HashMap<String, String> {
    let document = scraper::Html::parse_document(html);
    extract_cufe_invoice_data_from_document(&document)
}

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

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
    
    // ✨ OPTIMIZATION: Extract fields once to avoid multiple clones
    let url = request.url;
    let type_field = request.type_field.unwrap_or_default();
    let origin = request.origin.unwrap_or_default();
    let user_email = request.user_email;
    let user_phone_number = request.user_phone_number;
    let user_telegram_id = request.user_telegram_id;
    let user_ws = request.user_ws;
    
    info!("Processing URL request for user {}: {}", user_id, url);
    
    if url.trim().is_empty() {
        return Err(ApiError::validation_error("URL is required"));
    }

    // 1. Get final URL after following redirections
    info!("🔍 Resolving final URL for: {}", url);
    let final_url = match crate::processing::web_scraping::http_client::get_final_url(
        &state.http_client, 
        &url
    ).await {
        Ok(final_url_result) => {
            if final_url_result != url {
                info!("🔄 URL redirection detected: {} → {}", url, final_url_result);
            }
            final_url_result
        },
        Err(e) => {
            warn!("❌ Failed to resolve final URL: {}", e);
            // If we can't get final URL, use original (network issues, etc.)
            url.clone()
        }
    };

    // 2. Validate that final URL is from MEF Panama
    if !final_url.contains("dgi-fep.mef.gob.pa") && 
       !final_url.contains("fep.mef.gob.pa") &&
       !final_url.contains("mef.gob.pa") {
        error!("❌ Invalid final URL - not from MEF Panama: {}", final_url);
        return Err(ApiError::validation_error(
            "La URL no corresponde a una factura válida del MEF de Panamá"
        ));
    }

    info!("✅ Final URL validated as MEF invoice: {}", final_url);

    // 3. Scrape the invoice (using original URL, scraper will follow redirects again)
    match scrape_invoice(&state.http_client, &url, user_id).await {
        Ok(mut scraping_result) => {
            // Populate user fields in the header from request
            if let Some(ref mut header) = scraping_result.header {
                header.user_id = user_id;
                header.type_field = type_field.clone();
                header.origin = origin.clone();
                header.user_email = user_email.clone();
                header.user_phone_number = user_phone_number.clone();
                header.user_telegram_id = user_telegram_id.clone();
                header.user_ws = user_ws.clone();
            }
            
            // Save to database
            let db_result = persist_scraped_data(&state.db_pool, scraping_result.clone(), &url).await;
            
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
                        url: Some(url.clone()),
                        chat_id: user_ws.clone(),
                        reception_date: Some(chrono::Utc::now()),
                        message_id: None,
                        type_document: Some(if type_field.is_empty() { "URL".to_string() } else { type_field.clone() }),
                        user_email: user_email.clone(),
                        user_id: Some(user_id),
                        error_message: Some(error_response.message.clone()),
                        origin: Some(if origin.is_empty() { "API".to_string() } else { origin.clone() }),
                        ws_id: user_ws.clone(),
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
                    
                    // Return error response to client with user-friendly message
                    let user_friendly_message = categorize_scraping_error(&error_response.message);
                    let mut friendly_response = ProcessUrlResponse::error(user_friendly_message);
                    
                    // SPECIAL CASE: MEF Pending is considered a "successful queueing"
                    if user_friendly_message.contains("Tu factura ha sido recibida") {
                        friendly_response.success = true;
                    }

                    let response = ApiResponse {
                        success: false,
                        data: Some(friendly_response),
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
                url: Some(url.clone()),
                chat_id: user_ws.clone(),
                reception_date: Some(chrono::Utc::now()),
                message_id: None,
                type_document: Some(if type_field.is_empty() { "URL".to_string() } else { type_field.clone() }),
                user_email: user_email.clone(),
                user_id: Some(user_id),
                error_message: Some(format!("Scraping error: {}", e)),
                origin: Some(if origin.is_empty() { "API".to_string() } else { origin.clone() }),
                ws_id: user_ws.clone(),
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
            
            // Return user-friendly error with categorized message
            let user_message = categorize_scraping_error(&e);
            let mut error_response = ProcessUrlResponse::error(user_message);
            
            // SPECIAL CASE: MEF Pending is considered a "successful queueing"
            if user_message.contains("Tu factura ha sido recibida") {
                error_response.success = true;
            }

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
    Router::new()
        .route("/process-from-url", post(process_url_handler))
        .route("/process-from-cufe", post(process_cufe_handler))
}

// ============================================================================
// CUFE PROCESSING ENDPOINT - Direct API call to DGI with captcha
// ============================================================================

/// Request model for CUFE-based invoice processing
#[derive(serde::Deserialize)]
pub struct CufeRequest {
    /// The CUFE code (66 characters, starts with "FE")
    pub cufe: String,
    
    // Optional fields from user
    pub origin: Option<String>,
    pub user_email: Option<String>,
    pub user_phone_number: Option<String>,
    pub user_telegram_id: Option<String>,
    pub user_ws: Option<String>,
}

/// Response from DGI ConsultarFacturasPorCUFE endpoint
#[derive(serde::Deserialize, Debug)]
pub struct DgiCufeResponse {
    #[serde(rename = "FacturaHTML")]
    pub factura_html: Option<String>,
    #[serde(rename = "Error")]
    pub error: Option<String>,
    #[serde(rename = "Mensaje")]
    pub mensaje: Option<String>,
}

/// Validates CUFE format
/// Valid CUFE: starts with "FE", 60-75 characters, alphanumeric with hyphens
fn validate_cufe(cufe: &str) -> Result<String, &'static str> {
    let cufe = cufe.trim().to_uppercase();
    
    // Check prefix
    if !cufe.starts_with("FE") {
        return Err("CUFE debe comenzar con 'FE'");
    }
    
    // Check length (CUFE typically 66-70 chars, but allow some flexibility)
    if cufe.len() < 60 || cufe.len() > 75 {
        return Err("CUFE debe tener entre 60 y 75 caracteres");
    }
    
    // Check valid characters (alphanumeric and hyphens only)
    if !cufe.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("CUFE contiene caracteres inválidos");
    }
    
    Ok(cufe)
}

/// Calls DGI MEF API directly with CUFE and captcha token
/// Returns the HTML content of the invoice
async fn call_dgi_cufe_api(
    client: &reqwest::Client,
    cufe: &str,
    captcha_token: &str,
    session_id: &str,
) -> Result<String, String> {
    let url = "https://dgi-fep.mef.gob.pa/Consultas/ConsultarFacturasPorCUFE?Length=9";
    
    // Build form data exactly like Python
    let form_data = [
        ("CUFE", cufe),
        ("g-recaptcha-response", captcha_token),
        ("X-Requested-With", "XMLHttpRequest"),
    ];
    
    // Build cookies
    let cookie_value = if session_id.is_empty() {
        String::new()
    } else {
        format!("ASP.NET_SessionId={}", session_id)
    };
    
    info!("🔗 Calling DGI API for CUFE: {} (captcha: {} chars)", cufe, captcha_token.len());
    
    let mut request_builder = client
        .post(url)
        .header("accept", "*/*")
        .header("accept-language", "es-419,es;q=0.9,es-ES;q=0.8,en;q=0.7")
        .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
        .header("origin", "https://dgi-fep.mef.gob.pa")
        .header("referer", "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorCUFE")
        .header("sec-ch-ua", "\"Chromium\";v=\"140\", \"Not=A?Brand\";v=\"24\", \"Microsoft Edge\";v=\"140\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0")
        .header("x-requested-with", "XMLHttpRequest")
        .form(&form_data);
    
    // Add cookies if session_id is provided
    if !cookie_value.is_empty() {
        request_builder = request_builder.header("cookie", cookie_value);
    }
    
    let response = request_builder
        .send()
        .await
        .map_err(|e| format!("Error de conexión con DGI: {}", e))?;
    
    let status = response.status();
    info!("📥 DGI API response status: {}", status);
    
    if !status.is_success() {
        return Err(format!("DGI API error: HTTP {}", status));
    }
    
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Error leyendo respuesta DGI: {}", e))?;
    
    // Log response for debugging (first 500 chars)
    info!("📄 DGI response (first 500 chars): {}", &response_text[..response_text.len().min(500)]);
    
    // Try to parse as JSON
    let dgi_response: DgiCufeResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Error parseando respuesta DGI JSON: {} - Response: {}", e, &response_text[..response_text.len().min(500)]))?;
    
    // Check for errors in response FIRST (check error field and mensaje field, NOT the HTML)
    if let Some(ref error) = dgi_response.error {
        if !error.is_empty() {
            let error_lower = error.to_lowercase();
            if error_lower.contains("captcha") || error_lower.contains("verificación") || error_lower.contains("recaptcha") {
                return Err(format!("CAPTCHA_EXPIRED: {}", error));
            }
            if error_lower.contains("session") || error_lower.contains("sesión") {
                return Err(format!("SESSION_EXPIRED: {}", error));
            }
            return Err(format!("DGI_ERROR: {}", error));
        }
    }
    
    if let Some(ref mensaje) = dgi_response.mensaje {
        let mensaje_lower = mensaje.to_lowercase();
        if mensaje_lower.contains("error") || mensaje_lower.contains("no encontr") || mensaje_lower.contains("no válido") {
            if mensaje_lower.contains("captcha") || mensaje_lower.contains("verificación") {
                return Err(format!("CAPTCHA_EXPIRED: {}", mensaje));
            }
            return Err(format!("DGI_MESSAGE: {}", mensaje));
        }
    }
    
    // Check if FacturaHTML is empty or null
    match &dgi_response.factura_html {
        Some(html) if !html.is_empty() => {
            info!("✅ DGI returned FacturaHTML ({} chars)", html.len());
            Ok(html.clone())
        },
        _ => Err("DGI_NO_DATA: DGI no retornó HTML de factura".to_string())
    }
}

/// POST /api/v4/invoices/process-from-cufe
/// 
/// Processes an invoice using the CUFE code directly via DGI API.
/// This endpoint uses a captcha token that can be updated at runtime.
#[axum::debug_handler]
pub async fn process_cufe_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<CufeRequest>,
) -> Result<Json<ApiResponse<ProcessUrlResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&Uuid::new_v4().to_string())
        .to_string();

    let start_time = std::time::Instant::now();
    let user_id = current_user.user_id;
    
    info!("🔍 Processing CUFE request for user {}: {}", user_id, request.cufe);
    
    // 1. Validate CUFE format
    let cufe = match validate_cufe(&request.cufe) {
        Ok(valid_cufe) => valid_cufe,
        Err(error_msg) => {
            warn!("❌ Invalid CUFE format from user {}: {}", user_id, error_msg);
            return Err(ApiError::validation_error(error_msg));
        }
    };
    
    info!("✅ CUFE validated: {}", cufe);
    
    // 2. Get dynamic captcha token and session from AppState
    let captcha_token = state.dgi_captcha_token.read().await.clone();
    let session_id = state.dgi_session_id.read().await.clone();
    
    if captcha_token.is_empty() {
        error!("❌ DGI captcha token not configured");
        return Err(ApiError::new("CONFIG_ERROR", "Servicio DGI no configurado. Contacte al administrador."));
    }
    
    info!("🔑 Using captcha token ({} chars) and session ({} chars)", 
          captcha_token.len(), session_id.len());
    
    // 3. Call DGI API to get invoice HTML
    let html_content = match call_dgi_cufe_api(
        &state.http_client,
        &cufe,
        &captcha_token,
        &session_id,
    ).await {
        Ok(html) => {
            info!("✅ DGI returned HTML ({} bytes)", html.len());
            html
        },
        Err(e) => {
            error!("❌ DGI API error: {}", e);
            
            // Save to mef_pending for manual review
            let execution_time = start_time.elapsed().as_millis() as u64;
            
            if let Ok(mut tx) = state.db_pool.begin().await {
                let pending_entry = MefPending {
                    id: 0,
                    url: Some(format!("CUFE:{}", cufe)),
                    chat_id: request.user_ws.clone(),
                    reception_date: Some(chrono::Utc::now()),
                    message_id: None,
                    type_document: Some("CUFE".to_string()),
                    user_email: request.user_email.clone(),
                    user_id: Some(user_id),
                    error_message: Some(format!("DGI API error: {}", e)),
                    origin: Some(request.origin.clone().unwrap_or_else(|| "API".to_string())),
                    ws_id: request.user_ws.clone(),
                };
                
                if let Ok(_) = db_service::save_to_mef_pending(&mut tx, &pending_entry).await {
                    let _ = tx.commit().await;
                    info!("✅ Error guardado en mef_pending para revisión manual");
                }
            }
            
            // Return user-friendly error
            let user_message = categorize_scraping_error(&e);
            let error_response = ProcessUrlResponse::error(user_message);
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
    
    // Save HTML for debugging
    if let Err(e) = std::fs::write("/tmp/cufe_invoice.html", &html_content) {
        warn!("Failed to save HTML debug file: {}", e);
    }
    
    // 4. Build ScrapingResult - extract all data BEFORE any await
    // (scraper::Html is not Send, so we must finish using it before any await)
    let scraping_result = {
        let document = scraper::Html::parse_document(&html_content);
        let now_utc = chrono::Utc::now();
        
        // Use CUFE-specific extractor (different HTML structure than QR)
        let header_data = extract_cufe_invoice_data(&html_content);
        info!("✅ extract_cufe_invoice_data SUCCESS, keys: {:?}", header_data.keys().collect::<Vec<_>>());
        
        // Parse amounts
        let tot_amount = header_data.get("tot_amount")
            .and_then(|s| s.replace("B/.", "").replace("$", "").replace(",", "").trim().parse::<f64>().ok());
        let tot_itbms = header_data.get("tot_itbms")
            .and_then(|s| s.replace("B/.", "").replace("$", "").replace(",", "").trim().parse::<f64>().ok());
        
        let invoice_header = crate::api::webscraping::InvoiceHeader {
            cufe: cufe.clone(),
            no: header_data.get("no").cloned(),
            date: header_data.get("date").cloned(),
            auth_date: header_data.get("auth_protocol").cloned(),
            tot_amount,
            tot_itbms,
            issuer_name: header_data.get("emisor_name").cloned(),
            issuer_ruc: header_data.get("emisor_ruc").cloned(),
            issuer_dv: header_data.get("emisor_dv").cloned(),
            issuer_address: header_data.get("emisor_address").cloned(),
            issuer_phone: header_data.get("emisor_phone").cloned(),
            receptor_name: header_data.get("receptor_name").cloned(),
            receptor_id: header_data.get("receptor_ruc").cloned(),
            receptor_dv: header_data.get("receptor_dv").cloned(),
            receptor_address: header_data.get("receptor_address").cloned(),
            receptor_phone: header_data.get("receptor_phone").cloned(),
            user_id,
            user_email: request.user_email.clone(),
            user_phone_number: request.user_phone_number.clone(),
            user_telegram_id: request.user_telegram_id.clone(),
            user_ws: request.user_ws.clone(),
            origin: request.origin.clone().unwrap_or_else(|| "app".to_string()),
            type_field: "CUFE".to_string(),
            url: format!("CUFE:{}", cufe),
            process_date: now_utc,
            reception_date: now_utc,
            time: None,
        };
        
        // Extract details from table (using similar logic to webscraping)
        let details = extract_invoice_details_from_html(&document, &cufe);
        let payments = extract_invoice_payments_from_html(&document, &cufe);
        
        crate::api::webscraping::ScrapingResult {
            success: true,
            header: Some(invoice_header),
            details,
            payments,
            error_message: None,
        }
    }; // document is dropped here, before any await
    
    // 5. Persist using existing database_persistence
    let url_for_persistence = format!("CUFE:{}", cufe);
    let db_result = persist_scraped_data(&state.db_pool, scraping_result.clone(), &url_for_persistence).await;
    
    let execution_time = start_time.elapsed().as_millis() as u64;
    
    match db_result {
        Ok(mut process_response) => {
            // 7. Credit Lumis for gamification
            if let Some(ref cufe) = process_response.cufe {
                match crate::api::gamification_service::credit_lumis_for_invoice(
                    &state.db_pool,
                    user_id,
                    cufe
                ).await {
                    Ok(lumis_result) => {
                        process_response.lumis_earned = Some(lumis_result.lumis_earned);
                        process_response.lumis_balance = Some(lumis_result.lumis_balance);
                        
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
            // Check for duplicate
            if error_response.message.contains("duplicada") || error_response.message.contains("duplicate") {
                warn!("⚠️ Factura duplicada detectada");
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
            
            // Save to mef_pending
            warn!("❌ Error al guardar factura: '{}'. Guardando en mef_pending.", error_response.message);
            
            if let Ok(mut tx) = state.db_pool.begin().await {
                let pending_entry = MefPending {
                    id: 0,
                    url: Some(format!("CUFE:{}", cufe)),
                    chat_id: request.user_ws.clone(),
                    reception_date: Some(chrono::Utc::now()),
                    message_id: None,
                    type_document: Some("CUFE".to_string()),
                    user_email: request.user_email.clone(),
                    user_id: Some(user_id),
                    error_message: Some(error_response.message.clone()),
                    origin: Some(request.origin.clone().unwrap_or_else(|| "API".to_string())),
                    ws_id: request.user_ws.clone(),
                };
                
                if let Ok(_) = db_service::save_to_mef_pending(&mut tx, &pending_entry).await {
                    let _ = tx.commit().await;
                }
            }
            
            let user_friendly_message = categorize_scraping_error(&error_response.message);
            let friendly_response = ProcessUrlResponse::error(user_friendly_message);
            let response = ApiResponse {
                success: false,
                data: Some(friendly_response),
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

// Helper functions to extract details and payments from HTML (reusing webscraping logic)
fn extract_invoice_details_from_html(document: &scraper::Html, cufe: &str) -> Vec<crate::api::webscraping::InvoiceDetail> {
    use scraper::Selector;
    
    let mut details = Vec::new();
    
    let tbody_selector = match Selector::parse("tbody") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let tr_selector = match Selector::parse("tr") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let td_selector = match Selector::parse("td") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    for tbody in document.select(&tbody_selector) {
        for tr in tbody.select(&tr_selector) {
            let cells: Vec<_> = tr.select(&td_selector).collect();
            
            if cells.len() >= 8 {
                let line = cells[0].text().collect::<String>().trim().to_string();
                let code = cells[1].text().collect::<String>().trim().to_string();
                let description = cells[2].text().collect::<String>().trim().to_string();
                let information_of_interest = cells[3].text().collect::<String>().trim().to_string();
                let quantity = cells[4].text().collect::<String>().trim().to_string();
                let unit_price = cells[5].text().collect::<String>().trim().to_string();
                let unit_discount = cells[6].text().collect::<String>().trim().to_string();
                let amount = cells[7].text().collect::<String>().trim().to_string();
                let itbms = if cells.len() > 8 {
                    cells[8].text().collect::<String>().trim().to_string()
                } else {
                    "0.00".to_string()
                };
                let total = if cells.len() > 12 {
                    cells[12].text().collect::<String>().trim().to_string()
                } else {
                    amount.clone()
                };
                
                if code.is_empty() && description.is_empty() {
                    continue;
                }
                
                details.push(crate::api::webscraping::InvoiceDetail {
                    cufe: cufe.to_string(),
                    partkey: Some(format!("{}|{}", cufe, line)),
                    date: Some(chrono::Utc::now().format("%d/%m/%Y").to_string()),
                    quantity: Some(quantity),
                    code: Some(code),
                    description: Some(description),
                    unit_discount: Some(unit_discount),
                    unit_price: Some(unit_price),
                    itbms: Some(itbms),
                    amount: Some(amount),
                    total: Some(total),
                    information_of_interest: if information_of_interest.is_empty() { None } else { Some(information_of_interest) },
                });
            }
        }
    }

    details
}

fn extract_invoice_payments_from_html(document: &scraper::Html, cufe: &str) -> Vec<crate::api::webscraping::InvoicePayment> {
    use scraper::Selector;
    
    let tfoot_selector = match Selector::parse("tfoot") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let tr_selector = match Selector::parse("tr") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let td_selector = match Selector::parse("td") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let div_selector = match Selector::parse("div") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut forma_de_pago: Option<String> = None;
    let mut valor_pago: Option<String> = None;
    let mut efectivo: Option<String> = None;
    let mut tarjeta_credito: Option<String> = None;
    let mut tarjeta_debito: Option<String> = None;
    let mut tarjeta_clave_banistmo: Option<String> = None;
    #[allow(unused_mut)]
    let mut forma_de_pago_otro: Option<String> = None;
    let mut vuelto: Option<String> = None;
    let mut total_pagado: Option<String> = None;

    for tfoot in document.select(&tfoot_selector) {
        for tr in tfoot.select(&tr_selector) {
            if let Some(td) = tr.select(&td_selector).next() {
                let td_text = td.text().collect::<String>();
                let td_upper = td_text.to_uppercase();
                
                let value = if let Some(div) = td.select(&div_selector).next() {
                    div.text().collect::<String>().trim().to_string()
                } else {
                    String::new()
                };
                
                if value.is_empty() {
                    continue;
                }
                
                if td_upper.contains("EFECTIVO:") {
                    efectivo = Some(value);
                    if forma_de_pago.is_none() { forma_de_pago = Some("Efectivo".to_string()); }
                } else if td_upper.contains("TARJETA") && td_upper.contains("CRÉDITO") {
                    tarjeta_credito = Some(value);
                    if forma_de_pago.is_none() { forma_de_pago = Some("Tarjeta Crédito".to_string()); }
                } else if td_upper.contains("TARJETA") && td_upper.contains("DÉBITO") {
                    tarjeta_debito = Some(value);
                    if forma_de_pago.is_none() { forma_de_pago = Some("Tarjeta Débito".to_string()); }
                } else if td_upper.contains("TARJETA CLAVE") && td_upper.contains("BANISTMO") {
                    tarjeta_clave_banistmo = Some(value);
                    if forma_de_pago.is_none() { forma_de_pago = Some("Tarjeta Clave Banistmo".to_string()); }
                } else if td_upper.contains("TOTAL PAGADO:") {
                    total_pagado = Some(value.clone());
                    valor_pago = Some(value);
                } else if td_upper.contains("VUELTO:") {
                    vuelto = Some(value);
                }
            }
        }
    }

    if forma_de_pago.is_some() || total_pagado.is_some() {
        vec![crate::api::webscraping::InvoicePayment {
            cufe: cufe.to_string(),
            forma_de_pago,
            forma_de_pago_otro,
            valor_pago,
            efectivo,
            tarjeta_debito,
            tarjeta_credito,
            tarjeta_clave_banistmo,
            vuelto,
            total_pagado,
            descuentos: None,
            merged: None,
        }]
    } else {
        Vec::new()
    }
}

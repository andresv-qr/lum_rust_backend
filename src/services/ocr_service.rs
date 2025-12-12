use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};

use base64::{Engine as _, engine::general_purpose};
use serde_json::{json, Value};
use reqwest::Client;
use chrono::{DateTime, Utc};
use sqlx::types::Decimal;
use std::str::FromStr;

use crate::{
    services::{user_service, redis_service},
    state::AppState,
    models::user::User,
};

/// OCR response structure matching the Python implementation
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct OcrResponse {
    pub issuer_name: Option<String>,
    pub invoice_number: Option<String>,
    pub date: Option<String>,
    pub total: Option<f64>,
    pub ruc: Option<String>,
    pub dv: Option<String>,
    pub address: Option<String>,
    pub products: Vec<OcrProduct>,
    // Add other fields as needed
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct OcrProduct {
    pub name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partkey: Option<String>,
}

/// Request for OCR processing
#[derive(Debug)]
pub struct OcrProcessRequest {
    pub user_id: i64,
    pub user_identifier: String, // WhatsApp ID or email
    pub image_bytes: Vec<u8>,
    pub source: OcrSource,
    pub mode: OcrMode,
}

#[derive(Debug)]
pub enum OcrSource {
    WhatsApp,
    Api,
}

#[derive(Debug)]
pub enum OcrMode {
    Normal,      // Mode 1 - Standard processing
    Combined,    // Mode 2 - Combined image with duplicate removal
}

/// Response from OCR processing
#[derive(Debug, serde::Serialize)]
pub struct OcrProcessResponse {
    pub success: bool,
    pub cufe: Option<String>,
    pub invoice_number: Option<String>,
    pub issuer_name: Option<String>,
    pub issuer_ruc: Option<String>,
    pub issuer_dv: Option<String>,
    pub issuer_address: Option<String>,
    pub date: Option<String>,
    pub total: Option<f64>,
    pub tot_itbms: Option<f64>,
    pub products: Option<Vec<OcrProductResponse>>,
    pub cost_lumis: i32,
    pub message: String,
    /// Lista de campos obligatorios faltantes (solo presente cuando success=false por validación)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_fields: Option<Vec<RequiredField>>,
    /// Datos extraídos exitosamente (para usar en retry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_data: Option<ExtractedOcrData>,
}

/// Product details in OCR response
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct OcrProductResponse {
    pub name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partkey: Option<String>,
}

/// Campos obligatorios que deben estar presentes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequiredField {
    pub field_name: String,
    pub field_key: String,
    pub description: String,
}

/// Datos extraídos del OCR (para enviar al retry)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedOcrData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(default)]
    pub products: Vec<OcrProductResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tot_itbms: Option<f64>,
}

/// Resultado de validación de campos obligatorios
#[derive(Debug, serde::Serialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub missing_fields: Vec<RequiredField>,
    pub partial_data: PartialOcrData,
}

/// Datos parciales extraídos (para cuando faltan campos)
#[derive(Debug, serde::Serialize, Clone)]
pub struct PartialOcrData {
    pub ruc: Option<String>,
    pub dv: Option<String>,
    pub invoice_number: Option<String>,
    pub total: Option<f64>,
    pub products: Vec<OcrProductResponse>,
    pub issuer_name: Option<String>,
    pub issuer_address: Option<String>,
    pub date: Option<String>,
    pub tot_itbms: Option<f64>,
}

/// Log structure for OCR API calls
#[derive(Debug)]
struct OcrApiLog {
    user_id: i32,  // Cast from i64 to i32 for database INTEGER field
    image_size_bytes: i64,
    model_name: String,
    provider: String,
    endpoint_type: String,  // "upload" or "retry"
    success: bool,
    response_time_ms: i64,
    error_message: Option<String>,
    tokens_prompt: Option<i32>,
    tokens_completion: Option<i32>,
    tokens_total: Option<i32>,
    cost_prompt_usd: Option<Decimal>,
    cost_completion_usd: Option<Decimal>,
    cost_total_usd: Option<Decimal>,
    generation_id: Option<String>,
    model_used: Option<String>,
    finish_reason: Option<String>,
    extracted_fields: Option<Value>,
    raw_response: Option<Value>,
}


/// Request para retry de OCR con campos específicos y datos previos
#[derive(Debug, serde::Deserialize)]
pub struct OcrRetryRequest {
    pub missing_fields: Vec<String>,  // Lista de field_keys a buscar
    /// Datos extraídos previamente (del primer OCR)
    #[serde(default)]
    pub previous_data: Option<ExtractedOcrData>,
}

/// Main OCR processing service
pub struct OcrService;

impl OcrService {
    /// Log OCR API call to database
    async fn log_ocr_api_call(state: &AppState, log: &OcrApiLog) {
        let result = sqlx::query!(
            r#"
            INSERT INTO public.ocr_test_logs (
                user_id, image_path, image_size_bytes, model_name, provider, endpoint_type,
                success, response_time_ms, error_message,
                tokens_prompt, tokens_completion, tokens_total,
                cost_prompt_usd, cost_completion_usd, cost_total_usd,
                generation_id, model_used, finish_reason,
                extracted_fields, raw_response, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            "#,
            log.user_id,
            "api_call", // image_path not available in production, use placeholder
            log.image_size_bytes,
            log.model_name,
            log.provider,
            log.endpoint_type,
            log.success,
            log.response_time_ms,
            log.error_message,
            log.tokens_prompt,
            log.tokens_completion,
            log.tokens_total,
            log.cost_prompt_usd,
            log.cost_completion_usd,
            log.cost_total_usd,
            log.generation_id,
            log.model_used,
            log.finish_reason,
            log.extracted_fields,
            log.raw_response,
            Utc::now()
        )
        .execute(&state.db_pool)
        .await;

        match result {
            Ok(_) => info!("✅ OCR API call logged to database ({})", log.endpoint_type),
            Err(e) => warn!("⚠️ Failed to log OCR API call: {}", e),
        }
    }

    /// Process OCR for any source (WhatsApp or API)
    pub async fn process_ocr_invoice(
        state: Arc<AppState>,
        request: OcrProcessRequest,
    ) -> Result<OcrProcessResponse> {
        info!("🔍 Iniciando procesamiento OCR para usuario {}", request.user_identifier);

        // 1. Get user from database
        let user = match Self::get_user_by_source(&state, &request).await? {
            Some(u) => u,
            None => {
                return Ok(OcrProcessResponse {
                    success: false,
                    cufe: None,
                    invoice_number: None,
                    issuer_name: None,
                    issuer_ruc: None,
                    issuer_dv: None,
                    issuer_address: None,
                    date: None,
                    total: None,
                    tot_itbms: None,
                    products: None,
                    cost_lumis: 0,
                    message: "Usuario no encontrado. Por favor, regístrate primero.".to_string(),
                    missing_fields: None,
                    extracted_data: None,
                });
            }
        };

        // 2. Pre-validate the image
        if let Err(e) = Self::pre_validate_invoice_image(&request.image_bytes).await {
            warn!("Pre-validación de imagen falló para {}: {}", request.user_identifier, e);
            Self::log_ocr_attempt(&state, &request.user_identifier, "pre_validation_failed", &e.to_string()).await?;
            return Ok(OcrProcessResponse {
                success: false,
                cufe: None,
                invoice_number: None,
                issuer_name: None,
                issuer_ruc: None,
                issuer_dv: None,
                issuer_address: None,
                date: None,
                total: None,
                tot_itbms: None,
                products: None,
                cost_lumis: 0,
                message: "La imagen no parece ser una factura válida. Por favor, envía una imagen clara de tu factura.".to_string(),
                missing_fields: None,
                extracted_data: None,
            });
        }

        // 3. Check rate limits (different logic for WhatsApp vs API)
        if matches!(request.source, OcrSource::WhatsApp) {
            let (rate_allowed, rate_message) = redis_service::check_advanced_ocr_rate_limit(&state, &request.user_identifier).await?;
            if !rate_allowed {
                return Ok(OcrProcessResponse {
                    success: false,
                    cufe: None,
                    invoice_number: None,
                    issuer_name: None,
                    issuer_ruc: None,
                    issuer_dv: None,
                    issuer_address: None,
                    date: None,
                    total: None,
                    tot_itbms: None,
                    products: None,
                    cost_lumis: 0,
                    message: format!("{}. Usa más facturas con código QR para mejorar tu límite de OCR.", rate_message),
                    missing_fields: None,
                    extracted_data: None,
                });
            }
        }
        
        // 4. OCR sin costo en Lümis (por ahora)
        let ocr_cost = 0;

        // 6. Process with OCR (OpenRouter cascade with full logging)
        let ocr_response = match Self::process_image_with_ocr(&state, user.id, &request.image_bytes, &request.mode).await {
            Ok(response) => response,
            Err(e) => {
                error!("Error en procesamiento OCR para {}: {}", request.user_identifier, e);
                
                Self::log_ocr_attempt(&state, &request.user_identifier, "ocr_processing_error", &e.to_string()).await?;
                return Ok(OcrProcessResponse {
                    success: false,
                    cufe: None,
                    invoice_number: None,
                    issuer_name: None,
                    issuer_ruc: None,
                    issuer_dv: None,
                    issuer_address: None,
                    date: None,
                    total: None,
                    tot_itbms: None,
                    products: None,
                    cost_lumis: ocr_cost,
                    message: "Error procesando la imagen. Intenta con una imagen más clara.".to_string(),
                    missing_fields: None,
                    extracted_data: None,
                });
            }
        };

        // 7. Validate required fields (new v2 validation with detailed missing fields)
        let validation_result = Self::validate_required_fields_v2(&ocr_response);
        
        if !validation_result.is_valid {
            warn!("Validación de campos obligatorios falló para {}: {} campos faltantes", 
                  request.user_identifier, validation_result.missing_fields.len());
            
            let missing_field_names: Vec<String> = validation_result.missing_fields
                .iter()
                .map(|f| f.field_name.clone())
                .collect();
            
            Self::log_ocr_attempt(&state, &request.user_identifier, "missing_required_fields", 
                &format!("Campos faltantes: {}", missing_field_names.join(", "))).await?;
            
            // Devolver datos parciales + información de campos faltantes
            let partial_products: Vec<OcrProductResponse> = ocr_response.products.iter().map(|p| {
                OcrProductResponse {
                    name: p.name.clone(),
                    quantity: p.quantity,
                    unit_price: p.unit_price,
                    total_price: p.total_price,
                    partkey: None,
                }
            }).collect();
            
            // Construir extracted_data con los campos que SÍ se extrajeron
            let extracted_data = ExtractedOcrData {
                ruc: ocr_response.ruc.clone(),
                dv: ocr_response.dv.clone(),
                invoice_number: ocr_response.invoice_number.clone(),
                total: ocr_response.total,
                products: partial_products.clone(),
                issuer_name: ocr_response.issuer_name.clone(),
                issuer_address: ocr_response.address.clone(),
                date: ocr_response.date.clone(),
                tot_itbms: None,
            };
            
            // Construir mensaje descriptivo
            let message = format!(
                "No se pudieron detectar todos los campos obligatorios. Campos faltantes: {}. Por favor, sube una nueva imagen donde estos campos sean claramente visibles, o usa el endpoint /api/v4/invoices/upload-ocr-retry para reintentar con una imagen adicional.",
                missing_field_names.join(", ")
            );
            
            return Ok(OcrProcessResponse {
                success: false,
                cufe: None,
                invoice_number: ocr_response.invoice_number.clone(),
                issuer_name: ocr_response.issuer_name.clone(),
                issuer_ruc: ocr_response.ruc.clone(),
                issuer_dv: ocr_response.dv.clone(),
                issuer_address: ocr_response.address.clone(),
                date: ocr_response.date.clone(),
                total: ocr_response.total,
                tot_itbms: Some(0.0),
                products: if partial_products.is_empty() { None } else { Some(partial_products) },
                cost_lumis: ocr_cost,
                message,
                missing_fields: Some(validation_result.missing_fields),
                extracted_data: Some(extracted_data),
            });
        }

        // 8. Generate temporary CUFE (needed for duplicate check)
        let temp_cufe = Self::generate_ocr_cufe(&ocr_response, user.id).await?;
        
        // 9. Check for duplicate invoice (using CUFE)
        info!("🔍 VERIFICANDO DUPLICADOS:");
        info!("  🔍 CUFE generado: {}", temp_cufe);
        info!("  🔍 Issuer Name: {:?}", ocr_response.issuer_name);
        info!("  🔍 Invoice Number: {:?}", ocr_response.invoice_number);
        info!("  🔍 Date: {:?}", ocr_response.date);
        info!("  🔍 User ID: {}", user.id);
        
        if let Some(existing_cufe) = Self::check_duplicate_invoice(&state, &temp_cufe, &ocr_response, user.id).await? {
            info!("📋 ✅ FACTURA DUPLICADA DETECTADA para {}: CUFE existente {}", request.user_identifier, existing_cufe);
            
            Self::log_ocr_attempt(&state, &request.user_identifier, "duplicate_invoice", &format!("Existing CUFE: {}", existing_cufe)).await?;
            return Ok(OcrProcessResponse {
                success: false,
                cufe: Some(existing_cufe),
                invoice_number: ocr_response.invoice_number.clone(),
                issuer_name: ocr_response.issuer_name.clone(),
                issuer_ruc: ocr_response.ruc.clone(),
                issuer_dv: ocr_response.dv.clone(),
                issuer_address: ocr_response.address.clone(),
                date: ocr_response.date.clone(),
                total: ocr_response.total,
                tot_itbms: Some(0.0),
                products: Some(ocr_response.products.iter().map(|p| OcrProductResponse {
                    name: p.name.clone(),
                    quantity: p.quantity,
                    unit_price: p.unit_price,
                    total_price: p.total_price,
                    partkey: p.partkey.clone(),
                }).collect()),
                cost_lumis: ocr_cost,
                message: "Esta factura ya fue registrada anteriormente.".to_string(),
                missing_fields: None,
                extracted_data: None,
            });
        }

        info!("📋 ✅ NO SE ENCONTRÓ DUPLICADO - Procediendo a guardar factura");
        
        // 9.5. Assign partkeys to products
        let mut ocr_response_with_partkeys = ocr_response.clone();
        Self::assign_partkeys_to_products(&mut ocr_response_with_partkeys, &temp_cufe);
        
        // 10. Transform data and save to database
        if let Err(e) = Self::save_invoice_to_database(&state, &ocr_response_with_partkeys, &temp_cufe, &user, &request.user_identifier, &request.image_bytes).await {
            error!("Error guardando datos para {}: {}", request.user_identifier, e);
            
            // In case of DB error, we DON'T refund because OCR was successful
            // But we notify the problem
            Self::log_ocr_attempt(&state, &request.user_identifier, "database_error", &e.to_string()).await?;
            return Ok(OcrProcessResponse {
                success: false,
                cufe: Some(temp_cufe),
                invoice_number: ocr_response.invoice_number.clone(),
                issuer_name: ocr_response.issuer_name.clone(),
                issuer_ruc: ocr_response.ruc.clone(),
                issuer_dv: ocr_response.dv.clone(),
                issuer_address: ocr_response.address.clone(),
                date: ocr_response.date.clone(),
                total: ocr_response.total,
                tot_itbms: Some(0.0),
                products: Some(ocr_response_with_partkeys.products.iter().map(|p| OcrProductResponse {
                    name: p.name.clone(),
                    quantity: p.quantity,
                    unit_price: p.unit_price,
                    total_price: p.total_price,
                    partkey: p.partkey.clone(),
                }).collect()),
                cost_lumis: ocr_cost,
                message: "Tu factura fue procesada correctamente, pero hubo un problema guardando los datos. Nuestro equipo lo revisará.".to_string(),
                missing_fields: None,
                extracted_data: None,
            });
        }

        // 10. Log success
        Self::log_ocr_attempt(&state, &request.user_identifier, "success", &format!("CUFE: {}", temp_cufe)).await?;

        // 10.5. Log final products with partkeys
        info!("📋 PRODUCTOS CON PARTKEYS ASIGNADOS:");
        for (i, product) in ocr_response_with_partkeys.products.iter().enumerate() {
            let partkey = product.partkey.as_deref().unwrap_or("N/A");
            info!("  🔑 Producto {}: '{}' | Partkey: {}", 
                  i + 1, 
                  product.name.chars().take(40).collect::<String>(), 
                  partkey);
        }

        // 11. Success response
        info!("✅ Procesamiento OCR completado exitosamente para {}", request.user_identifier);
        
        // Convertir productos a la estructura de respuesta
        let products_response: Vec<OcrProductResponse> = ocr_response_with_partkeys.products.iter().map(|p| {
            OcrProductResponse {
                name: p.name.clone(),
                quantity: p.quantity,
                unit_price: p.unit_price,
                total_price: p.total_price,
                partkey: p.partkey.clone(),
            }
        }).collect();
        
        Ok(OcrProcessResponse {
            success: true,
            cufe: Some(temp_cufe),
            invoice_number: ocr_response_with_partkeys.invoice_number.clone(),
            issuer_name: ocr_response_with_partkeys.issuer_name.clone(),
            issuer_ruc: ocr_response_with_partkeys.ruc.clone(),
            issuer_dv: ocr_response_with_partkeys.dv.clone(),
            issuer_address: ocr_response_with_partkeys.address.clone(),
            date: ocr_response_with_partkeys.date.clone(),
            total: ocr_response_with_partkeys.total,
            tot_itbms: Some(0.0), // TODO: Calcular ITBMS de los productos si es necesario
            products: Some(products_response),
            cost_lumis: ocr_cost,
            message: "Factura procesada exitosamente. Pendiente de validación por nuestro equipo.".to_string(),
            missing_fields: None,
            extracted_data: None,
        })
    }

    /// Get user by source (WhatsApp ID or email for API)
    async fn get_user_by_source(
        state: &Arc<AppState>,
        request: &OcrProcessRequest,
    ) -> Result<Option<User>> {
        match request.source {
            OcrSource::WhatsApp => {
                user_service::get_user(state, &request.user_identifier).await
            }
            OcrSource::Api => {
                // For API, user_identifier should be email or user_id
                // For now, assume it's email - you might need to adapt this
                user_service::get_user_by_email(&state.db_pool, &request.user_identifier).await
                    .map_err(|e| anyhow!("Error getting user by email: {}", e))
            }
        }
    }

    /// Pre-validate invoice image
    async fn pre_validate_invoice_image(_image_bytes: &[u8]) -> Result<()> {
        // TODO: Implement actual image pre-validation
        // - Check image size and format
        // - Basic image quality checks
        // - Detect if it looks like a document/invoice
        Ok(())
    }

    // Deprecated: Replaced by process_with_openrouter_logged with cascade system
    #[allow(dead_code)]
    async fn process_image_with_gemini(image_bytes: &[u8], mode: &OcrMode) -> Result<OcrResponse> {
        info!("🤖 Iniciando procesamiento con Gemini OCR (mode: {:?})", mode);
        
        // Get Gemini API key from environment
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("GEMINI_API_KEY no configurado"))?;

        // Encode image to base64
        let image_base64 = general_purpose::STANDARD.encode(image_bytes);

        // Create the client
        let client = Client::new();

        // Base prompt for OCR
        let base_prompt = "Analiza esta imagen de una factura de Panamá y extrae TODA la información visible en formato JSON exacto:\n\n{\n  \"issuer_name\": \"nombre completo del comercio/empresa emisora (busca nombres grandes arriba de la factura)\",\n  \"ruc\": \"número RUC completo (busca 'RUC:', 'RUC', números cerca del nombre del comercio, puede tener formato 1234567-1-123456 o similar)\",\n  \"dv\": \"dígito verificador que viene después del RUC (ej: si dice 'RUC: 123456-1-654321 DV: 89', extrae '89')\",\n  \"address\": \"dirección completa del establecimiento\",\n  \"invoice_number\": \"número de factura completo (busca 'Factura', 'Fact', números con guiones como 001-002-123456)\",\n  \"date\": \"fecha de emisión en formato YYYY-MM-DD (busca 'Fecha:', fechas en formato DD/MM/YYYY o similar)\",\n  \"total\": valor_total_numerico (busca 'Total', 'Total a Pagar', el número más grande al final),\n  \"products\": [\n    {\n      \"name\": \"descripción completa del producto/ítem\",\n      \"quantity\": cantidad_numerica (si no está, usa 1),\n      \"unit_price\": precio_unitario_numerico,\n      \"total_price\": precio_total_del_item_numerico\n    }\n  ]\n}\n\nINSTRUCCIONES IMPORTANTES:\n1. Extrae TODOS los productos visibles en la factura, no omitas ninguno\n2. Para el RUC, busca números largos cerca del nombre del comercio o en la parte superior\n3. La fecha puede estar en varios formatos (DD/MM/YYYY, DD-MM-YYYY, etc), conviértela a YYYY-MM-DD\n4. Si no encuentras algún campo opcional (DV, dirección), usa null\n5. Los campos CRÍTICOS son: issuer_name, ruc, date, total, products (al menos 1)\n6. Solo responde con el JSON, sin texto adicional ni explicaciones";
        
        // Add mode-specific instruction
        let prompt = match mode {
            OcrMode::Normal => base_prompt.to_string(),
            OcrMode::Combined => format!("{}\n\nTen en cuenta que esta imagen es una combinación de varias imágenes, por lo que puede contener datos duplicados. Por favor, elimina los duplicados y construye una única factura consolidada, sin información repetida.", base_prompt)
        };

        // Gemini request payload
        let payload = json!({
            "contents": [{
                "parts": [
                    {
                        "text": prompt
                    },
                    {
                        "inline_data": {
                            "mime_type": "image/jpeg",
                            "data": image_base64
                        }
                    }
                ]
            }],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 2048
            }
        });

        // Make the request
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}", api_key);
        
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Error en request a Gemini: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Error en Gemini API: {} - {}", status, error_text));
        }

        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Error parseando respuesta de Gemini: {}", e))?;

        // 🔍 LOG: Respuesta completa de Gemini
        info!("🔍 GEMINI RESPONSE COMPLETA: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        // Extract the text from the response
        let text = response_json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("No se pudo extraer texto de la respuesta de Gemini"))?;

        // 🔍 LOG: Texto extraído de Gemini
        info!("🔍 TEXTO ORIGINAL DE GEMINI: '{}'", text);

        // Parse the JSON response - handle markdown code blocks
        let cleaned_text = Self::extract_json_from_markdown(text);
        
        // 🔍 LOG: Texto limpiado después del markdown
        info!("🔍 TEXTO LIMPIADO (sin markdown): '{}'", cleaned_text);

        let ocr_response: OcrResponse = serde_json::from_str(&cleaned_text)
            .map_err(|e| anyhow!("Error parseando JSON de OCR: {} - Texto limpio: {} - Texto original: {}", e, cleaned_text, text))?;

        // 🔍 LOG: Datos parseados del OCR
        info!("🔍 OCR DATOS PARSEADOS:");
        info!("  📄 Issuer Name: {:?}", ocr_response.issuer_name);
        info!("  🏢 RUC: {:?}", ocr_response.ruc);
        info!("  🔢 DV: {:?}", ocr_response.dv);
        info!("  🏠 Address: {:?}", ocr_response.address);
        info!("  📄 Invoice Number: {:?}", ocr_response.invoice_number);
        info!("  📄 Date: {:?}", ocr_response.date);
        info!("  📄 Total: {:?}", ocr_response.total);
        info!("  📄 Products Count: {}", ocr_response.products.len());
        for (i, product) in ocr_response.products.iter().enumerate() {
            info!("    🛒 Product {}: '{}' - Qty: {} - Price: {} - Total: {}", 
                i + 1, product.name, product.quantity, product.unit_price, product.total_price);
        }

        info!("✅ OCR Gemini exitoso: {:?}", ocr_response.issuer_name);
        Ok(ocr_response)
    }

    /// Get OCR prompt based on mode
    fn get_ocr_prompt(mode: &OcrMode) -> String {
        let base_prompt = "Analiza esta imagen de una factura de Panamá y extrae TODA la información visible en formato JSON exacto:\n\n{\n  \"issuer_name\": \"nombre completo del comercio/empresa emisora (busca nombres grandes arriba de la factura)\",\n  \"ruc\": \"número RUC completo (busca 'RUC:', 'RUC', números cerca del nombre del comercio, puede tener formato 1234567-1-123456 o similar)\",\n  \"dv\": \"dígito verificador que viene después del RUC (ej: si dice 'RUC: 123456-1-654321 DV: 89', extrae '89')\",\n  \"address\": \"dirección completa del establecimiento\",\n  \"invoice_number\": \"número de factura completo (busca 'Factura', 'Fact', números con guiones como 001-002-123456)\",\n  \"date\": \"fecha de emisión en formato YYYY-MM-DD (busca 'Fecha:', fechas en formato DD/MM/YYYY o similar)\",\n  \"total\": valor_total_numerico (busca 'Total', 'Total a Pagar', el número más grande al final),\n  \"products\": [\n    {\n      \"name\": \"descripción completa del producto/ítem\",\n      \"quantity\": cantidad_numerica (si no está, usa 1),\n      \"unit_price\": precio_unitario_numerico,\n      \"total_price\": precio_total_del_item_numerico\n    }\n  ]\n}\n\nINSTRUCCIONES IMPORTANTES:\n1. Extrae TODOS los productos visibles en la factura, no omitas ninguno\n2. Para el RUC, busca números largos cerca del nombre del comercio o en la parte superior\n3. La fecha puede estar en varios formatos (DD/MM/YYYY, DD-MM-YYYY, etc), conviértela a YYYY-MM-DD\n4. Si no encuentras algún campo opcional (DV, dirección), usa null\n5. Los campos CRÍTICOS son: issuer_name, ruc, date, total, products (al menos 1)\n6. Solo responde con el JSON, sin texto adicional ni explicaciones";
        
        match mode {
            OcrMode::Normal => base_prompt.to_string(),
            OcrMode::Combined => format!("{}\n\nTen en cuenta que esta imagen es una combinación de varias imágenes, por lo que puede contener datos duplicados. Por favor, elimina los duplicados y construye una única factura consolidada, sin información repetida.", base_prompt)
        }
    }

    /// Process image with OpenRouter with full logging
    async fn process_with_openrouter_logged(
        state: &AppState,
        user_id: i32,  // Already cast from i64
        image_bytes: &[u8],
        model: &str,
        mode: &OcrMode
    ) -> Result<OcrResponse> {
        let start_time = std::time::Instant::now();
        
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set in environment variables");

        let image_base64 = general_purpose::STANDARD.encode(image_bytes);
        let prompt = Self::get_ocr_prompt(mode);

        let payload = json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", image_base64)
                        }
                    }
                ]
            }],
            "temperature": 0.1,
            "max_tokens": 8192
        });

        let client = Client::new();
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        let response_time_ms = start_time.elapsed().as_millis() as i64;

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("Request error: {}", e);
                
                // Log failed attempt
                let log = OcrApiLog {
                    user_id,
                    image_size_bytes: image_bytes.len() as i64,
                    model_name: model.to_string(),
                    provider: "openrouter".to_string(),
                    endpoint_type: "upload".to_string(),
                    success: false,
                    response_time_ms,
                    error_message: Some(error_msg.clone()),
                    tokens_prompt: None,
                    tokens_completion: None,
                    tokens_total: None,
                    cost_prompt_usd: None,
                    cost_completion_usd: None,
                    cost_total_usd: None,
                    generation_id: None,
                    model_used: None,
                    finish_reason: None,
                    extracted_fields: None,
                    raw_response: None,
                };
                Self::log_ocr_api_call(state, &log).await;
                
                return Err(anyhow!(error_msg));
            }
        };

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let error_msg = format!("OpenRouter API error: {}", error_text);
            
            // Log failed attempt
            let log = OcrApiLog {
                user_id,
                image_size_bytes: image_bytes.len() as i64,
                model_name: model.to_string(),
                provider: "openrouter".to_string(),
                endpoint_type: "upload".to_string(),
                success: false,
                response_time_ms,
                error_message: Some(error_msg.clone()),
                tokens_prompt: None,
                tokens_completion: None,
                tokens_total: None,
                cost_prompt_usd: None,
                cost_completion_usd: None,
                cost_total_usd: None,
                generation_id: None,
                model_used: None,
                finish_reason: None,
                extracted_fields: None,
                raw_response: None,
            };
            Self::log_ocr_api_call(state, &log).await;
            
            return Err(anyhow!(error_msg));
        }

        let response_text = response.text().await?;
        let response_json: Value = serde_json::from_str(&response_text)?;

        // Extract metadata and usage
        let tokens_prompt = response_json["usage"]["prompt_tokens"].as_i64().map(|t| t as i32);
        let tokens_completion = response_json["usage"]["completion_tokens"].as_i64().map(|t| t as i32);
        let tokens_total = response_json["usage"]["total_tokens"].as_i64().map(|t| t as i32);
        
        let cost_total_usd = response_json["usage"]["cost"].as_f64()
            .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
        let cost_prompt_usd = response_json["usage"]["cost_details"]["upstream_inference_prompt_cost"].as_f64()
            .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
        let cost_completion_usd = response_json["usage"]["cost_details"]["upstream_inference_completions_cost"].as_f64()
            .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
        
        let generation_id = response_json["id"].as_str().map(|s| s.to_string());
        let model_used = response_json["model"].as_str().map(|s| s.to_string());
        let finish_reason = response_json["choices"][0]["finish_reason"].as_str().map(|s| s.to_string());

        info!("🎫 Tokens: {} (prompt: {}, completion: {})", 
              tokens_total.unwrap_or(0),
              tokens_prompt.unwrap_or(0),
              tokens_completion.unwrap_or(0));
        if let Some(cost) = &cost_total_usd {
            info!("💰 Cost: ${}", cost);
        }

        let text = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("No content in OpenRouter response"))?;

        let cleaned_text = Self::extract_json_from_markdown(text);
        
        let ocr_response: OcrResponse = match serde_json::from_str(&cleaned_text) {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("Error parsing OCR JSON: {} - Text: {}", e, cleaned_text);
                
                // Log failed attempt
                let log = OcrApiLog {
                    user_id,
                    image_size_bytes: image_bytes.len() as i64,
                    model_name: model.to_string(),
                    provider: "openrouter".to_string(),
                    endpoint_type: "upload".to_string(),
                    success: false,
                    response_time_ms,
                    error_message: Some(error_msg.clone()),
                    tokens_prompt,
                    tokens_completion,
                    tokens_total,
                    cost_prompt_usd,
                    cost_completion_usd,
                    cost_total_usd,
                    generation_id: generation_id.clone(),
                    model_used: model_used.clone(),
                    finish_reason: finish_reason.clone(),
                    extracted_fields: None,
                    raw_response: Some(response_json),
                };
                Self::log_ocr_api_call(state, &log).await;
                
                return Err(anyhow!(error_msg));
            }
        };

        // Log successful attempt
        let extracted_json = serde_json::to_value(&ocr_response).ok();
        let log = OcrApiLog {
            user_id,
            image_size_bytes: image_bytes.len() as i64,
            model_name: model.to_string(),
            provider: "openrouter".to_string(),
            endpoint_type: "upload".to_string(),
            success: true,
            response_time_ms,
            error_message: None,
            tokens_prompt,
            tokens_completion,
            tokens_total,
            cost_prompt_usd,
            cost_completion_usd,
            cost_total_usd,
            generation_id,
            model_used,
            finish_reason,
            extracted_fields: extracted_json,
            raw_response: Some(response_json),
        };
        Self::log_ocr_api_call(state, &log).await;

        info!("✅ OCR {} exitoso: {:?}", model, ocr_response.issuer_name);
        Ok(ocr_response)
    }

    /// Extract JSON from markdown code blocks
    fn extract_json_from_markdown(text: &str) -> String {
        let text = text.trim();
        
        // Check if text is wrapped in markdown code blocks
        if text.starts_with("```json") && text.ends_with("```") {
            // Remove ```json from start and ``` from end
            let without_start = text.strip_prefix("```json").unwrap_or(text);
            let without_end = without_start.strip_suffix("```").unwrap_or(without_start);
            without_end.trim().to_string()
        } else if text.starts_with("```") && text.ends_with("```") {
            // Remove ``` from start and end (generic code block)
            let without_start = text.strip_prefix("```").unwrap_or(text);
            let without_end = without_start.strip_suffix("```").unwrap_or(without_start);
            without_end.trim().to_string()
        } else {
            // Return as is if no code block markers
            text.to_string()
        }
    }

    // Deprecated: Replaced by process_with_openrouter_logged with full logging
    #[allow(dead_code)]
    async fn process_image_with_openrouter(image_bytes: &[u8], mode: &OcrMode) -> Result<OcrResponse> {
        info!("🔄 FALLBACK: Iniciando procesamiento con OpenRouter (Qwen3-VL-30B)");
        
        // OpenRouter API key
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set in environment variables");

        // Encode image to base64
        let image_base64 = general_purpose::STANDARD.encode(image_bytes);

        // Create the client
        let client = Client::new();

        // Base prompt for OCR (same as Gemini)
        let base_prompt = "Analiza esta imagen de una factura de Panamá y extrae TODA la información visible en formato JSON exacto:\n\n{\n  \"issuer_name\": \"nombre completo del comercio/empresa emisora (busca nombres grandes arriba de la factura)\",\n  \"ruc\": \"número RUC completo (busca 'RUC:', 'RUC', números cerca del nombre del comercio, puede tener formato 1234567-1-123456 o similar)\",\n  \"dv\": \"dígito verificador que viene después del RUC (ej: si dice 'RUC: 123456-1-654321 DV: 89', extrae '89')\",\n  \"address\": \"dirección completa del establecimiento\",\n  \"invoice_number\": \"número de factura completo (busca 'Factura', 'Fact', números con guiones como 001-002-123456)\",\n  \"date\": \"fecha de emisión en formato YYYY-MM-DD (busca 'Fecha:', fechas en formato DD/MM/YYYY o similar)\",\n  \"total\": valor_total_numerico (busca 'Total', 'Total a Pagar', el número más grande al final),\n  \"products\": [\n    {\n      \"name\": \"descripción completa del producto/ítem\",\n      \"quantity\": cantidad_numerica (si no está, usa 1),\n      \"unit_price\": precio_unitario_numerico,\n      \"total_price\": precio_total_del_item_numerico\n    }\n  ]\n}\n\nINSTRUCCIONES IMPORTANTES:\n1. Extrae TODOS los productos visibles en la factura, no omitas ninguno\n2. Para el RUC, busca números largos cerca del nombre del comercio o en la parte superior\n3. La fecha puede estar en varios formatos (DD/MM/YYYY, DD-MM-YYYY, etc), conviértela a YYYY-MM-DD\n4. Si no encuentras algún campo opcional (DV, dirección), usa null\n5. Los campos CRÍTICOS son: issuer_name, ruc, date, total, products (al menos 1)\n6. Solo responde con el JSON, sin texto adicional ni explicaciones";
        
        // Add mode-specific instruction
        let prompt = match mode {
            OcrMode::Normal => base_prompt.to_string(),
            OcrMode::Combined => format!("{}\n\nTen en cuenta que esta imagen es una combinación de varias imágenes, por lo que puede contener datos duplicados. Por favor, elimina los duplicados y construye una única factura consolidada, sin información repetida.", base_prompt)
        };

        // OpenRouter request payload (OpenAI-compatible format with vision)
        let payload = json!({
            "model": "qwen/qwen3-vl-30b-a3b-instruct",
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", image_base64)
                        }
                    }
                ]
            }],
            "temperature": 0.1,
            "max_tokens": 2048
        });

        // Make the request to OpenRouter
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://lumis.app")
            .header("X-Title", "Lumis OCR Fallback")
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Error en request a OpenRouter: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Error en OpenRouter API: {} - {}", status, error_text));
        }

        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Error parseando respuesta de OpenRouter: {}", e))?;

        // 🔍 LOG: Respuesta completa de OpenRouter
        info!("🔍 OPENROUTER RESPONSE COMPLETA: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        // Extract the text from the response (OpenAI format)
        let text = response_json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("No se pudo extraer texto de la respuesta de OpenRouter"))?;

        // 🔍 LOG: Texto extraído de OpenRouter
        info!("🔍 TEXTO ORIGINAL DE OPENROUTER: '{}'", text);

        // Parse the JSON response - handle markdown code blocks
        let cleaned_text = Self::extract_json_from_markdown(text);
        
        // 🔍 LOG: Texto limpiado después del markdown
        info!("🔍 TEXTO LIMPIADO (sin markdown): '{}'", cleaned_text);

        let ocr_response: OcrResponse = serde_json::from_str(&cleaned_text)
            .map_err(|e| anyhow!("Error parseando JSON de OCR (OpenRouter): {} - Texto limpio: {} - Texto original: {}", e, cleaned_text, text))?;

        // 🔍 LOG: Datos parseados del OCR
        info!("🔍 OPENROUTER OCR DATOS PARSEADOS:");
        info!("  📄 Issuer Name: {:?}", ocr_response.issuer_name);
        info!("  🏢 RUC: {:?}", ocr_response.ruc);
        info!("  🔢 DV: {:?}", ocr_response.dv);
        info!("  🏠 Address: {:?}", ocr_response.address);
        info!("  📄 Invoice Number: {:?}", ocr_response.invoice_number);
        info!("  📄 Date: {:?}", ocr_response.date);
        info!("  📄 Total: {:?}", ocr_response.total);
        info!("  📄 Products Count: {}", ocr_response.products.len());

        info!("✅ OCR OpenRouter (Qwen3-VL) exitoso: {:?}", ocr_response.issuer_name);
        Ok(ocr_response)
    }

    /// Process image with OCR - uses OpenRouter models in cascade
    /// Models: qwen3-vl-8b -> qwen3-vl-30b -> qwen2.5-vl-72b
    async fn process_image_with_ocr(
        state: &AppState,
        user_id: i64,
        image_bytes: &[u8],
        mode: &OcrMode
    ) -> Result<OcrResponse> {
        let models = vec![
            "qwen/qwen3-vl-8b-instruct",
            "qwen/qwen3-vl-30b-a3b-instruct",
            "qwen/qwen2.5-vl-72b-instruct",
        ];

        for (i, model) in models.iter().enumerate() {
            info!("🔄 Intentando OCR con modelo {} ({}/{})...", model, i + 1, models.len());
            
            // Cast user_id from i64 to i32 for database
            match Self::process_with_openrouter_logged(state, user_id as i32, image_bytes, model, mode).await {
                Ok(response) => {
                    info!("✅ OCR procesado exitosamente con {}", model);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("⚠️ {} OCR falló: {}", model, e);
                    if i < models.len() - 1 {
                        info!("➡️ Intentando con siguiente modelo en cascada...");
                    } else {
                        error!("❌ Todos los modelos OCR fallaron");
                        return Err(anyhow!("OCR falló en todos los modelos. Último error: {}", e));
                    }
                }
            }
        }

        Err(anyhow!("OCR falló en todos los modelos"))
    }

    /// Validate required fields and collect all missing fields
    /// Returns ValidationResult with partial data and missing fields info
    fn validate_required_fields_v2(ocr_response: &OcrResponse) -> ValidationResult {
        let mut missing_fields: Vec<RequiredField> = Vec::new();
        
        // 1. Validar RUC (OBLIGATORIO)
        let ruc_valid = ocr_response.ruc.as_ref()
            .map(|r| !r.trim().is_empty())
            .unwrap_or(false);
        if !ruc_valid {
            missing_fields.push(RequiredField {
                field_name: "RUC del comercio".to_string(),
                field_key: "ruc".to_string(),
                description: "Número de RUC del comercio emisor (ej: 155751938-2-2024)".to_string(),
            });
        }
        
        // 2. Validar DV (OBLIGATORIO)
        let dv_valid = ocr_response.dv.as_ref()
            .map(|d| !d.trim().is_empty())
            .unwrap_or(false);
        if !dv_valid {
            missing_fields.push(RequiredField {
                field_name: "Dígito Verificador (DV)".to_string(),
                field_key: "dv".to_string(),
                description: "Dígito verificador que acompaña al RUC (ej: 66, 89)".to_string(),
            });
        }
        
        // 3. Validar Número de Factura (OBLIGATORIO)
        let invoice_valid = ocr_response.invoice_number.as_ref()
            .map(|n| !n.trim().is_empty())
            .unwrap_or(false);
        if !invoice_valid {
            missing_fields.push(RequiredField {
                field_name: "Número de Factura".to_string(),
                field_key: "invoice_number".to_string(),
                description: "Número o código de la factura (ej: 001-002-123456, 10374)".to_string(),
            });
        }
        
        // 4. Validar Monto Total (OBLIGATORIO)
        let total_valid = ocr_response.total
            .map(|t| t > 0.0)
            .unwrap_or(false);
        if !total_valid {
            missing_fields.push(RequiredField {
                field_name: "Monto Total".to_string(),
                field_key: "total".to_string(),
                description: "Valor total de la factura (ej: 25.99, 1.30)".to_string(),
            });
        }
        
        // 5. Validar Productos (OBLIGATORIO - al menos 1 con descripción y monto)
        let products_valid = if ocr_response.products.is_empty() {
            false
        } else {
            // Al menos un producto debe tener descripción y precio
            ocr_response.products.iter().any(|p| {
                !p.name.trim().is_empty() && p.total_price > 0.0
            })
        };
        if !products_valid {
            missing_fields.push(RequiredField {
                field_name: "Detalle de Productos".to_string(),
                field_key: "products".to_string(),
                description: "Al menos un producto con descripción y monto (ej: 'Coca Cola 500ml - $1.50')".to_string(),
            });
        }
        
        // Construir datos parciales
        let partial_data = PartialOcrData {
            ruc: ocr_response.ruc.clone(),
            dv: ocr_response.dv.clone(),
            invoice_number: ocr_response.invoice_number.clone(),
            total: ocr_response.total,
            products: ocr_response.products.iter().map(|p| OcrProductResponse {
                name: p.name.clone(),
                quantity: p.quantity,
                unit_price: p.unit_price,
                total_price: p.total_price,
                partkey: None,
            }).collect(),
            issuer_name: ocr_response.issuer_name.clone(),
            issuer_address: ocr_response.address.clone(),
            date: ocr_response.date.clone(),
            tot_itbms: None,
        };
        
        ValidationResult {
            is_valid: missing_fields.is_empty(),
            missing_fields,
            partial_data,
        }
    }

    /// Assign partkeys to products based on CUFE
    fn assign_partkeys_to_products(ocr_response: &mut OcrResponse, cufe: &str) {
        for (index, product) in ocr_response.products.iter_mut().enumerate() {
            // partkey format: {cufe}|{i} where i starts from 1
            let partkey = format!("{}|{}", cufe, index + 1);
            product.partkey = Some(partkey.clone());
            
            info!("🔑 Partkey asignado al producto '{}': {}", 
                  product.name.chars().take(30).collect::<String>(), partkey);
        }
    }

    /// Generate OCR CUFE
    async fn generate_ocr_cufe(ocr_response: &OcrResponse, _user_id: i64) -> Result<String> {
        // Usar RUC+DV en lugar del nombre del comercio
        let ruc = ocr_response.ruc.as_deref().unwrap_or("UNKNOWN");
        let dv = ocr_response.dv.as_deref().unwrap_or("");
        
        // Normalizar RUC (eliminar guiones y espacios)
        let normalized_ruc = ruc
            .replace('-', "")
            .replace(' ', "")
            .trim()
            .to_string();
        
        // Combinar RUC+DV
        let ruc_dv = if !dv.is_empty() {
            format!("{}{}", normalized_ruc, dv)
        } else {
            normalized_ruc
        };
        
        // Procesar fecha (eliminar guiones para formato YYYYMMDD)
        let processed_date = ocr_response.date.as_deref().unwrap_or("19700101").replace('-', "");
        
        // Normalizar número de factura
        let invoice_number = ocr_response.invoice_number.as_deref().unwrap_or("UNKNOWN");
        let normalized_invoice = invoice_number
            .trim()
            .replace('-', "_");
        
        // Generar CUFE con el patrón: OCR-[RUC+DV]-[FECHA]-[NUMERO]
        let cufe = format!(
            "OCR-{}-{}-{}",
            ruc_dv,
            processed_date,
            normalized_invoice
        );
        
        info!("🏷️ CUFE generado: {} (RUC+DV: {}, Fecha: {}, Número: {})", 
              cufe, ruc_dv, processed_date, normalized_invoice);
        
        Ok(cufe)
    }

    /// Save invoice data to database - extracted from WhatsApp implementation
    async fn save_invoice_to_database(
        state: &Arc<AppState>,
        ocr_response: &OcrResponse,
        temp_cufe: &str,
        user: &User,
        user_identifier: &str,
        image_bytes: &[u8],
    ) -> Result<()> {
        info!("💾 Guardando datos de factura en base de datos: {}", temp_cufe);
        
        let mut tx = state.db_pool.begin().await
            .map_err(|e| anyhow!("Error iniciando transacción: {}", e))?;
        
        // 1. Transform and insert invoice header
        let user_email = user.email.as_deref().unwrap_or(user_identifier);
        let header_data = Self::transform_ocr_to_header(ocr_response, temp_cufe, user.id, user_identifier, user_email, None, image_bytes)?; // user_ws es None por defecto, puede ser parámetro de API
        
        sqlx::query!(
            r#"
            INSERT INTO public.invoice_header (
                cufe, issuer_name, no, date, tot_amount, issuer_ruc, issuer_dv, 
                issuer_address, type, origin, user_id, user_ws, user_email, 
                url, tot_itbms, time, process_date, reception_date
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
            header_data.cufe,
            header_data.issuer_name,
            header_data.no,
            header_data.date,
            header_data.tot_amount,
            header_data.issuer_ruc,
            header_data.issuer_dv,
            header_data.issuer_address,
            header_data.type_field,
            header_data.origin,
            header_data.user_id as i32,
            header_data.user_ws,
            header_data.user_email,
            header_data.url,
            header_data.tot_itbms,
            header_data.time,
            header_data.process_date,
            header_data.reception_date
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando invoice_header: {}", e))?;
        
        // 2. Transform and insert invoice details
        let detail_data = Self::transform_ocr_to_detail(ocr_response, temp_cufe)?;
        
        for detail in detail_data {
            sqlx::query!(
                r#"
                INSERT INTO public.invoice_detail (
                    cufe, partkey, code, description, information_of_interest, 
                    quantity, unit_price, unit_discount, amount, itbms, total, date
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#,
                detail.cufe,
                detail.partkey,
                detail.code,
                detail.description,
                detail.information_of_interest,
                detail.quantity,
                detail.unit_price,
                detail.unit_discount,
                detail.amount,
                detail.itbms,
                detail.total,
                detail.date
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Error insertando invoice_detail: {}", e))?;
        }
        
        // 3. Transform and insert payment data
        let payment_data = Self::transform_ocr_to_payment(ocr_response, temp_cufe)?;
        
        sqlx::query!(
            r#"
            INSERT INTO public.invoice_payment (
                cufe, total_pagado, forma_de_pago, efectivo, valor_pago
            ) VALUES ($1, $2, $3, $4, $5)
            "#,
            payment_data.cufe,
            payment_data.total_pagado,
            payment_data.forma_de_pago,
            payment_data.efectivo,
            payment_data.valor_pago
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando invoice_payment: {}", e))?;
        
        // Commit the transaction
        tx.commit().await
            .map_err(|e| anyhow!("Error confirmando transacción: {}", e))?;
        
        info!("✅ Datos de factura guardados exitosamente: {}", temp_cufe);
        Ok(())
    }

    /// Check if invoice is duplicate based on issuer_name, invoice_number, date and user
    async fn check_duplicate_invoice(
        state: &Arc<AppState>,
        cufe: &str,
        ocr_response: &OcrResponse,
        user_id: i64,
    ) -> Result<Option<String>> {
        // Verificar primero por CUFE (constraint único en BD)
        let cufe_check = sqlx::query!(
            "SELECT cufe FROM public.invoice_header WHERE cufe = $1 LIMIT 1",
            cufe
        )
        .fetch_optional(&state.db_pool)
        .await?;

        if let Some(row) = cufe_check {
            info!("🔍 Duplicado encontrado por CUFE: {}", cufe);
            return Ok(row.cufe);
        }

        // Verificación adicional por issuer_name, número y fecha (por si acaso)
        let date_obj = if let Some(date_str) = &ocr_response.date {
            if !date_str.is_empty() {
                chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_else(|| Utc::now().naive_utc()))
                .unwrap_or_else(|_| Utc::now().naive_utc())
            } else {
                Utc::now().naive_utc()
            }
        } else {
            Utc::now().naive_utc()
        };

        let query_result = sqlx::query!(
            "SELECT cufe FROM public.invoice_header WHERE issuer_name = $1 AND no = $2 AND date::date = $3::date AND user_id = $4 LIMIT 1",
            ocr_response.issuer_name,
            ocr_response.invoice_number,
            date_obj.date(),
            user_id as i32
        )
        .fetch_optional(&state.db_pool)
        .await?;

        match query_result {
            Some(row) => {
                info!("🔍 Duplicado encontrado por datos: issuer={:?}, no={:?}, date={:?}", 
                      ocr_response.issuer_name, ocr_response.invoice_number, date_obj.date());
                Ok(row.cufe)
            },
            None => Ok(None),
        }
    }

    /// Log OCR attempt for analytics and rate limiting
    async fn log_ocr_attempt(
        _state: &Arc<AppState>,
        user_identifier: &str,
        status: &str,
        details: &str,
    ) -> Result<()> {
        // TODO: Implement actual logging to database
        info!("📊 OCR Attempt - User: {}, Status: {}, Details: {}", user_identifier, status, details);
        Ok(())
    }

    // Data transformation functions (extracted from WhatsApp implementation)
    
    fn transform_ocr_to_header(
        ocr_data: &OcrResponse,
        cufe: &str,
        user_id: i64,
        _user_identifier: &str,
        user_email: &str,
        user_ws: Option<String>,
        image_bytes: &[u8],
    ) -> Result<InvoiceHeaderData> {
        // Convert image to base64 data URL
        let image_base64 = general_purpose::STANDARD.encode(image_bytes);
        let data_url = format!("data:image/jpeg;base64,{}", image_base64);
        
        // Generate current timestamp GMT-5 (Colombia time) for date field
        let current_time = Utc::now();
        let colombia_offset = chrono::FixedOffset::west_opt(5 * 3600).unwrap(); // GMT-5
        let colombia_time = current_time.with_timezone(&colombia_offset);

        
        // Generate current time in HHMMSS format
        let time_hhmmss = current_time.format("%H%M%S").to_string();
        
        Ok(InvoiceHeaderData {
            cufe: cufe.to_string(),
            issuer_name: ocr_data.issuer_name.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
            no: ocr_data.invoice_number.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
            date: colombia_time.naive_local(), // Timestamp GMT-5 como NaiveDateTime
            tot_amount: ocr_data.total.unwrap_or(0.0),
            issuer_ruc: ocr_data.ruc.clone(),
            issuer_dv: ocr_data.dv.clone(),
            issuer_address: ocr_data.address.clone(),

            type_field: "ocr_pending".to_string(),
            origin: "api".to_string(),
            user_id,
            user_ws: user_ws, // Parámetro opcional
            user_email: user_email.to_string(),
            url: data_url,
            tot_itbms: 0.0, // Calculate from products if needed
            time: time_hhmmss,
            process_date: Utc::now(),
            reception_date: Utc::now(),
        })
    }
    
    fn transform_ocr_to_detail(
        ocr_data: &OcrResponse,
        cufe: &str,
    ) -> Result<Vec<InvoiceDetailData>> {
        let mut details = Vec::new();
        
        for (index, product) in ocr_data.products.iter().enumerate() {
            details.push(InvoiceDetailData {
                cufe: cufe.to_string(),
                partkey: (index + 1).to_string(),
                code: format!("OCR-{}", index + 1),
                description: product.name.clone(),
                information_of_interest: "Extraído por OCR".to_string(),
                quantity: product.quantity.to_string(),
                unit_price: product.unit_price.to_string(),
                unit_discount: "0".to_string(),
                amount: product.total_price.to_string(),
                itbms: "0".to_string(), // Calculate if needed
                total: product.total_price.to_string(),
                date: ocr_data.date.clone().unwrap_or_else(|| "1970-01-01".to_string()),
            });
        }
        
        Ok(details)
    }
    
    fn transform_ocr_to_payment(
        ocr_data: &OcrResponse,
        cufe: &str,
    ) -> Result<InvoicePaymentData> {
        Ok(InvoicePaymentData {
            cufe: cufe.to_string(),
            total_pagado: ocr_data.total.unwrap_or(0.0).to_string(),
            forma_de_pago: "Efectivo".to_string(), // Default
            efectivo: ocr_data.total.unwrap_or(0.0).to_string(),
            valor_pago: ocr_data.total.unwrap_or(0.0).to_string(),
        })
    }

    /// Process OCR retry - focuses only on specific missing fields
    /// 
    /// Este método es para cuando el usuario toma una nueva foto
    /// enfocada en los campos que no se pudieron detectar la primera vez.
    /// Combina los datos previos con los nuevos extraídos.
    pub async fn process_ocr_retry(
        state: Arc<AppState>,
        user_id: i64,
        user_email: String,
        image_bytes: Vec<u8>,
        retry_request: OcrRetryRequest,
    ) -> Result<OcrProcessResponse> {
        info!("🔄 Iniciando OCR RETRY para usuario {} - campos: {:?}", user_email, retry_request.missing_fields);
        info!("📦 Datos previos recibidos: {:?}", retry_request.previous_data.is_some());

        // 1. Verify user exists
        let user_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE user_id = $1)"
        )
        .bind(user_id)
        .fetch_one(&state.db_pool)
        .await?;

        if !user_exists {
            return Ok(OcrProcessResponse {
                success: false,
                cufe: None,
                invoice_number: None,
                issuer_name: None,
                issuer_ruc: None,
                issuer_dv: None,
                issuer_address: None,
                date: None,
                total: None,
                tot_itbms: None,
                products: None,
                cost_lumis: 0,
                message: "Usuario no encontrado.".to_string(),
                missing_fields: None,
                extracted_data: None,
            });
        }

        // 2. Build specialized prompt for missing fields with OpenRouter cascade
        let ocr_result = Self::process_image_for_specific_fields_logged(
            &state,
            user_id as i32,
            &image_bytes, 
            &retry_request.missing_fields,
            retry_request.previous_data.as_ref(),
        ).await;

        match ocr_result {
            Ok(new_ocr_response) => {
                // 3. Merge previous data with new extracted data
                let merged_data = Self::merge_ocr_data(
                    retry_request.previous_data.as_ref(),
                    &new_ocr_response,
                    &retry_request.missing_fields,
                );
                
                info!("🔗 Datos combinados:");
                info!("  RUC: {:?}", merged_data.ruc);
                info!("  DV: {:?}", merged_data.dv);
                info!("  Invoice: {:?}", merged_data.invoice_number);
                info!("  Total: {:?}", merged_data.total);
                info!("  Products: {}", merged_data.products.len());
                
                // 4. Validate completeness with merged data
                let validation = Self::validate_merged_data(&merged_data);
                
                if validation.is_valid {
                    // All fields complete!
                    info!("✅ OCR RETRY exitoso - factura completa con datos combinados");
                    
                    Ok(OcrProcessResponse {
                        success: true,
                        cufe: None,  // Retry no genera CUFE aún
                        invoice_number: merged_data.invoice_number.clone(),
                        issuer_name: merged_data.issuer_name.clone(),
                        issuer_ruc: merged_data.ruc.clone(),
                        issuer_dv: merged_data.dv.clone(),
                        issuer_address: merged_data.issuer_address.clone(),
                        date: merged_data.date.clone(),
                        total: merged_data.total,
                        tot_itbms: merged_data.tot_itbms,
                        products: if merged_data.products.is_empty() { None } else { Some(merged_data.products.clone()) },
                        cost_lumis: 5, // Costo reducido por ser retry
                        message: "¡Factura completa! Todos los campos obligatorios fueron extraídos.".to_string(),
                        missing_fields: None,
                        extracted_data: Some(merged_data),
                    })
                } else {
                    // Still missing some fields
                    warn!("⚠️ OCR RETRY parcial - aún faltan campos: {:?}", validation.missing_fields);
                    
                    Ok(OcrProcessResponse {
                        success: false,
                        cufe: None,
                        invoice_number: merged_data.invoice_number.clone(),
                        issuer_name: merged_data.issuer_name.clone(),
                        issuer_ruc: merged_data.ruc.clone(),
                        issuer_dv: merged_data.dv.clone(),
                        issuer_address: merged_data.issuer_address.clone(),
                        date: merged_data.date.clone(),
                        total: merged_data.total,
                        tot_itbms: merged_data.tot_itbms,
                        products: if merged_data.products.is_empty() { None } else { Some(merged_data.products.clone()) },
                        cost_lumis: 5,
                        message: format!("Aún no se pudieron detectar todos los campos requeridos. Faltan: {}", 
                            validation.missing_fields.iter()
                                .map(|f| f.field_name.clone())
                                .collect::<Vec<_>>()
                                .join(", ")),
                        missing_fields: Some(validation.missing_fields),
                        extracted_data: Some(merged_data),
                    })
                }
            }
            Err(e) => {
                error!("💥 Error en OCR RETRY: {}", e);
                // En caso de error, devolver los datos previos si existen
                let prev = retry_request.previous_data.clone();
                Ok(OcrProcessResponse {
                    success: false,
                    cufe: None,
                    invoice_number: prev.as_ref().and_then(|p| p.invoice_number.clone()),
                    issuer_name: prev.as_ref().and_then(|p| p.issuer_name.clone()),
                    issuer_ruc: prev.as_ref().and_then(|p| p.ruc.clone()),
                    issuer_dv: prev.as_ref().and_then(|p| p.dv.clone()),
                    issuer_address: prev.as_ref().and_then(|p| p.issuer_address.clone()),
                    date: prev.as_ref().and_then(|p| p.date.clone()),
                    total: prev.as_ref().and_then(|p| p.total),
                    tot_itbms: prev.as_ref().and_then(|p| p.tot_itbms),
                    products: prev.as_ref().map(|p| p.products.clone()).filter(|p| !p.is_empty()),
                    cost_lumis: 0,
                    message: format!("Error procesando la imagen: {}. Los datos previos se mantienen.", e),
                    missing_fields: None,
                    extracted_data: prev,
                })
            }
        }
    }

    /// Merge previous OCR data with new extracted data
    /// Prioriza los nuevos datos para los campos que se estaban buscando
    fn merge_ocr_data(
        previous: Option<&ExtractedOcrData>,
        new_response: &OcrResponse,
        searched_fields: &[String],
    ) -> ExtractedOcrData {
        let prev = previous.cloned().unwrap_or_else(|| ExtractedOcrData {
            ruc: None,
            dv: None,
            invoice_number: None,
            total: None,
            products: vec![],
            issuer_name: None,
            issuer_address: None,
            date: None,
            tot_itbms: None,
        });
        
        // Para cada campo: usar el nuevo si se buscaba Y se encontró, sino mantener el anterior
        let use_new_ruc = searched_fields.contains(&"ruc".to_string()) 
            && new_response.ruc.as_ref().map(|r| !r.trim().is_empty()).unwrap_or(false);
        let use_new_dv = searched_fields.contains(&"dv".to_string()) 
            && new_response.dv.as_ref().map(|d| !d.trim().is_empty()).unwrap_or(false);
        let use_new_invoice = searched_fields.contains(&"invoice_number".to_string()) 
            && new_response.invoice_number.as_ref().map(|n| !n.trim().is_empty()).unwrap_or(false);
        let use_new_total = searched_fields.contains(&"total".to_string()) 
            && new_response.total.map(|t| t > 0.0).unwrap_or(false);
        let use_new_products = searched_fields.contains(&"products".to_string()) 
            && !new_response.products.is_empty() 
            && new_response.products.iter().any(|p| !p.name.trim().is_empty() && p.total_price > 0.0);
        
        ExtractedOcrData {
            ruc: if use_new_ruc { new_response.ruc.clone() } else { prev.ruc },
            dv: if use_new_dv { new_response.dv.clone() } else { prev.dv },
            invoice_number: if use_new_invoice { new_response.invoice_number.clone() } else { prev.invoice_number },
            total: if use_new_total { new_response.total } else { prev.total },
            products: if use_new_products {
                new_response.products.iter().map(|p| OcrProductResponse {
                    name: p.name.clone(),
                    quantity: p.quantity,
                    unit_price: p.unit_price,
                    total_price: p.total_price,
                    partkey: None,
                }).collect()
            } else {
                prev.products
            },
            // Campos secundarios: usar nuevo si existe, sino mantener anterior
            issuer_name: new_response.issuer_name.clone().or(prev.issuer_name),
            issuer_address: new_response.address.clone().or(prev.issuer_address),
            date: new_response.date.clone().or(prev.date),
            tot_itbms: prev.tot_itbms, // Mantener el anterior si existe
        }
    }

    /// Validate merged data for completeness
    fn validate_merged_data(data: &ExtractedOcrData) -> ValidationResult {
        let mut missing_fields: Vec<RequiredField> = Vec::new();
        
        // 1. Validar RUC
        if !data.ruc.as_ref().map(|r| !r.trim().is_empty()).unwrap_or(false) {
            missing_fields.push(RequiredField {
                field_name: "RUC del comercio".to_string(),
                field_key: "ruc".to_string(),
                description: "Número de RUC del comercio emisor".to_string(),
            });
        }
        
        // 2. Validar DV
        if !data.dv.as_ref().map(|d| !d.trim().is_empty()).unwrap_or(false) {
            missing_fields.push(RequiredField {
                field_name: "Dígito Verificador (DV)".to_string(),
                field_key: "dv".to_string(),
                description: "Dígito verificador que acompaña al RUC".to_string(),
            });
        }
        
        // 3. Validar Número de Factura
        if !data.invoice_number.as_ref().map(|n| !n.trim().is_empty()).unwrap_or(false) {
            missing_fields.push(RequiredField {
                field_name: "Número de Factura".to_string(),
                field_key: "invoice_number".to_string(),
                description: "Número o código de la factura".to_string(),
            });
        }
        
        // 4. Validar Total
        if !data.total.map(|t| t > 0.0).unwrap_or(false) {
            missing_fields.push(RequiredField {
                field_name: "Monto Total".to_string(),
                field_key: "total".to_string(),
                description: "Valor total de la factura".to_string(),
            });
        }
        
        // 5. Validar Productos
        let has_valid_products = !data.products.is_empty() 
            && data.products.iter().any(|p| !p.name.trim().is_empty() && p.total_price > 0.0);
        if !has_valid_products {
            missing_fields.push(RequiredField {
                field_name: "Detalle de Productos".to_string(),
                field_key: "products".to_string(),
                description: "Al menos un producto con descripción y monto".to_string(),
            });
        }
        
        ValidationResult {
            is_valid: missing_fields.is_empty(),
            missing_fields,
            partial_data: PartialOcrData {
                ruc: data.ruc.clone(),
                dv: data.dv.clone(),
                invoice_number: data.invoice_number.clone(),
                total: data.total,
                products: data.products.clone(),
                issuer_name: data.issuer_name.clone(),
                issuer_address: data.issuer_address.clone(),
                date: data.date.clone(),
                tot_itbms: data.tot_itbms,
            },
        }
    }

    /// Process image focusing only on specific fields with OpenRouter cascade and logging
    async fn process_image_for_specific_fields_logged(
        state: &AppState,
        user_id: i32,
        image_bytes: &[u8],
        missing_fields: &[String],
        previous_data: Option<&ExtractedOcrData>,
    ) -> Result<OcrResponse> {
        info!("🎯 Procesando imagen RETRY para campos específicos: {:?}", missing_fields);
        
        // Use OpenRouter cascade (same as main OCR flow)
        let models = vec![
            "qwen/qwen3-vl-8b-instruct",
            "qwen/qwen3-vl-30b-a3b-instruct",
            "qwen/qwen2.5-vl-72b-instruct",
        ];

        for (i, model) in models.iter().enumerate() {
            info!("🔄 RETRY: Intentando con modelo {} ({}/{})...", model, i + 1, models.len());
            
            match Self::process_retry_with_openrouter_logged(
                state, user_id, image_bytes, model, missing_fields, previous_data
            ).await {
                Ok(response) => {
                    info!("✅ RETRY OCR procesado exitosamente con {}", model);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("⚠️ RETRY {} falló: {}", model, e);
                    if i < models.len() - 1 {
                        info!("➡️ Intentando con siguiente modelo en cascada...");
                    } else {
                        error!("❌ Todos los modelos RETRY fallaron");
                        return Err(e);
                    }
                }
            }
        }
        
        Err(anyhow!("Todos los modelos de OCR RETRY fallaron"))
    }

    /// Process retry with OpenRouter and full logging
    async fn process_retry_with_openrouter_logged(
        state: &AppState,
        user_id: i32,
        image_bytes: &[u8],
        model: &str,
        missing_fields: &[String],
        previous_data: Option<&ExtractedOcrData>,
    ) -> Result<OcrResponse> {
        let start_time = std::time::Instant::now();
        
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set in environment variables");

        let image_base64 = general_purpose::STANDARD.encode(image_bytes);

        // Build focused prompt with previous data context
        let prompt = Self::build_retry_prompt(missing_fields, previous_data);

        let payload = json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/jpeg;base64,{}", image_base64)
                        }
                    }
                ]
            }],
            "temperature": 0.1,
            "max_tokens": 4096
        });

        let client = Client::new();
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        let response_time_ms = start_time.elapsed().as_millis() as i64;

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("RETRY Request error: {}", e);
                
                // Log failed attempt
                let log = OcrApiLog {
                    user_id,
                    image_size_bytes: image_bytes.len() as i64,
                    model_name: model.to_string(),
                    provider: "openrouter".to_string(),
                    endpoint_type: "retry".to_string(),
                    success: false,
                    response_time_ms,
                    error_message: Some(error_msg.clone()),
                    tokens_prompt: None,
                    tokens_completion: None,
                    tokens_total: None,
                    cost_prompt_usd: None,
                    cost_completion_usd: None,
                    cost_total_usd: None,
                    generation_id: None,
                    model_used: None,
                    finish_reason: None,
                    extracted_fields: None,
                    raw_response: None,
                };
                Self::log_ocr_api_call(state, &log).await;
                
                return Err(anyhow!(error_msg));
            }
        };

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let error_msg = format!("RETRY OpenRouter API error: {}", error_text);
            
            // Log failed attempt
            let log = OcrApiLog {
                user_id,
                image_size_bytes: image_bytes.len() as i64,
                model_name: model.to_string(),
                provider: "openrouter".to_string(),
                endpoint_type: "retry".to_string(),
                success: false,
                response_time_ms,
                error_message: Some(error_msg.clone()),
                tokens_prompt: None,
                tokens_completion: None,
                tokens_total: None,
                cost_prompt_usd: None,
                cost_completion_usd: None,
                cost_total_usd: None,
                generation_id: None,
                model_used: None,
                finish_reason: None,
                extracted_fields: None,
                raw_response: None,
            };
            Self::log_ocr_api_call(state, &log).await;
            
            return Err(anyhow!(error_msg));
        }

        let response_text = response.text().await?;
        let response_json: Value = serde_json::from_str(&response_text)?;

        // Extract metadata and usage
        let tokens_prompt = response_json["usage"]["prompt_tokens"].as_i64().map(|t| t as i32);
        let tokens_completion = response_json["usage"]["completion_tokens"].as_i64().map(|t| t as i32);
        let tokens_total = response_json["usage"]["total_tokens"].as_i64().map(|t| t as i32);
        
        let cost_total_usd = response_json["usage"]["cost"].as_f64()
            .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
        let cost_prompt_usd = response_json["usage"]["cost_details"]["upstream_inference_prompt_cost"].as_f64()
            .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
        let cost_completion_usd = response_json["usage"]["cost_details"]["upstream_inference_completions_cost"].as_f64()
            .and_then(|v| Decimal::from_str(&format!("{:.8}", v)).ok());
        
        let generation_id = response_json["id"].as_str().map(|s| s.to_string());
        let model_used = response_json["model"].as_str().map(|s| s.to_string());
        let finish_reason = response_json["choices"][0]["finish_reason"].as_str().map(|s| s.to_string());

        info!("🎫 RETRY Tokens: {} (prompt: {}, completion: {})", 
              tokens_total.unwrap_or(0),
              tokens_prompt.unwrap_or(0),
              tokens_completion.unwrap_or(0));
        if let Some(cost) = &cost_total_usd {
            info!("💰 RETRY Cost: ${}", cost);
        }

        let text = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("No content in RETRY OpenRouter response"))?;

        let cleaned_text = Self::extract_json_from_markdown(text);
        
        let ocr_response: OcrResponse = match serde_json::from_str(&cleaned_text) {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("Error parsing RETRY OCR JSON: {} - Text: {}", e, cleaned_text);
                
                // Log failed attempt
                let log = OcrApiLog {
                    user_id,
                    image_size_bytes: image_bytes.len() as i64,
                    model_name: model.to_string(),
                    provider: "openrouter".to_string(),
                    endpoint_type: "retry".to_string(),
                    success: false,
                    response_time_ms,
                    error_message: Some(error_msg.clone()),
                    tokens_prompt,
                    tokens_completion,
                    tokens_total,
                    cost_prompt_usd,
                    cost_completion_usd,
                    cost_total_usd,
                    generation_id: generation_id.clone(),
                    model_used: model_used.clone(),
                    finish_reason: finish_reason.clone(),
                    extracted_fields: None,
                    raw_response: Some(response_json),
                };
                Self::log_ocr_api_call(state, &log).await;
                
                return Err(anyhow!(error_msg));
            }
        };

        // Log successful attempt
        let extracted_json = serde_json::to_value(&ocr_response).ok();
        let log = OcrApiLog {
            user_id,
            image_size_bytes: image_bytes.len() as i64,
            model_name: model.to_string(),
            provider: "openrouter".to_string(),
            endpoint_type: "retry".to_string(),
            success: true,
            response_time_ms,
            error_message: None,
            tokens_prompt,
            tokens_completion,
            tokens_total,
            cost_prompt_usd,
            cost_completion_usd,
            cost_total_usd,
            generation_id,
            model_used,
            finish_reason,
            extracted_fields: extracted_json,
            raw_response: Some(response_json),
        };
        Self::log_ocr_api_call(state, &log).await;

        info!("✅ RETRY OCR {} exitoso: RUC={:?}, DV={:?}, Invoice={:?}", 
              model, ocr_response.ruc, ocr_response.dv, ocr_response.invoice_number);
        Ok(ocr_response)
    }

    /// Build specialized prompt for retry with previous data context
    fn build_retry_prompt(missing_fields: &[String], previous_data: Option<&ExtractedOcrData>) -> String {
        // Build field instructions
        let field_instructions = missing_fields.iter().map(|f| {
            match f.as_str() {
                "ruc" => "- RUC: Número de RUC del comercio (formato: números con guiones como 1234567-1-654321, busca cerca del nombre del negocio, encabezado, o pie de factura)".to_string(),
                "dv" => "- DV: Dígito Verificador que acompaña al RUC (usualmente 2 dígitos después de 'DV:' o al final del RUC)".to_string(),
                "invoice_number" => "- invoice_number: Número de factura (busca 'Factura', 'Fact', 'No.', 'Nro', números con formato como 001-002-123456)".to_string(),
                "total" => "- total: Monto total de la factura (busca 'Total', 'Total a Pagar', generalmente el número más grande al final)".to_string(),
                "products" => "- products: Lista de productos/servicios con nombre, cantidad y precio (escanea todas las líneas de ítems)".to_string(),
                _ => format!("- {}: valor correspondiente", f)
            }
        }).collect::<Vec<_>>().join("\n");

        // Build previous data context if available
        let previous_context = if let Some(data) = previous_data {
            let mut context_parts = vec![];
            
            if let Some(ref name) = data.issuer_name {
                context_parts.push(format!("- Comercio: {}", name));
            }
            if let Some(ref addr) = data.issuer_address {
                context_parts.push(format!("- Dirección: {}", addr));
            }
            if let Some(ref date) = data.date {
                context_parts.push(format!("- Fecha: {}", date));
            }
            if let Some(total) = data.total {
                context_parts.push(format!("- Total: ${:.2}", total));
            }
            if let Some(ref ruc) = data.ruc {
                context_parts.push(format!("- RUC: {}", ruc));
            }
            if let Some(ref dv) = data.dv {
                context_parts.push(format!("- DV: {}", dv));
            }
            if let Some(ref inv) = data.invoice_number {
                context_parts.push(format!("- Número de Factura: {}", inv));
            }
            if !data.products.is_empty() {
                context_parts.push(format!("- Productos: {} items ya detectados", data.products.len()));
            }
            
            if context_parts.is_empty() {
                String::new()
            } else {
                format!("\n\n📋 DATOS YA CAPTURADOS (no necesitas buscarlos):\n{}\n", context_parts.join("\n"))
            }
        } else {
            String::new()
        };

        format!(r#"Esta imagen es de una factura de Panamá. Esta es una imagen ADICIONAL para completar datos faltantes.
{previous_context}
🔍 SOLO NECESITO QUE BUSQUES ESTOS CAMPOS ESPECÍFICOS QUE FALTAN:

{field_instructions}

Responde ÚNICAMENTE con un JSON con esta estructura exacta:

{{
  "issuer_name": "nombre del comercio o null",
  "ruc": "número RUC completo o null",
  "dv": "dígito verificador o null",
  "address": "dirección o null",
  "invoice_number": "número de factura o null",
  "date": "fecha en formato YYYY-MM-DD o null",
  "total": valor_numerico_o_null,
  "products": [
    {{
      "name": "descripción del producto",
      "quantity": cantidad_numerica,
      "unit_price": precio_unitario,
      "total_price": precio_total
    }}
  ]
}}

INSTRUCCIONES IMPORTANTES:
1. ENFÓCATE ESPECÍFICAMENTE en los campos listados como FALTANTES
2. Para el RUC, busca en TODA la imagen: encabezado, pie de página, cerca del nombre del comercio
3. El DV suele estar justo después del RUC o etiquetado como "DV:"
4. Si el campo NO está visible en la imagen, usa null (NO INVENTES DATOS)
5. Para productos, extrae TODOS los que sean visibles
6. Solo responde con el JSON, sin texto adicional ni explicaciones"#, 
        previous_context = previous_context, 
        field_instructions = field_instructions)
    }

    // Deprecated: Old Gemini-based retry function - keeping for reference
    #[allow(dead_code)]
    async fn process_image_for_specific_fields(image_bytes: &[u8], missing_fields: &[String]) -> Result<OcrResponse> {
        info!("🎯 Procesando imagen para campos específicos: {:?}", missing_fields);
        
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("GEMINI_API_KEY no configurado"))?;

        let image_base64 = general_purpose::STANDARD.encode(image_bytes);
        let client = Client::new();

        // Build focused prompt based on which fields we need
        let field_instructions = missing_fields.iter().map(|f| {
            match f.as_str() {
                "ruc" => "- RUC: Número de RUC del comercio (formato: números con guiones como 1234567-1-654321, busca cerca del nombre del negocio o encabezado)".to_string(),
                "dv" => "- DV: Dígito Verificador que acompaña al RUC (usualmente 2 dígitos después de 'DV:' o al final del RUC)".to_string(),
                "invoice_number" => "- invoice_number: Número de factura (busca 'Factura', 'Fact', 'No.', números con formato como 001-002-123456)".to_string(),
                "total" => "- total: Monto total de la factura (busca 'Total', 'Total a Pagar', generalmente el número más grande al final)".to_string(),
                "products" => "- products: Lista de productos/servicios con nombre, cantidad y precio (escanea todas las líneas de ítems)".to_string(),
                _ => format!("- {}: valor correspondiente", f)
            }
        }).collect::<Vec<_>>().join("\n");

        let prompt = format!(r#"Esta imagen es de una factura de Panamá. SOLO necesito que extraigas los siguientes campos ESPECÍFICOS:

{field_instructions}

Responde ÚNICAMENTE con un JSON con esta estructura exacta (incluye todos los campos aunque estén vacíos):

{{
  "issuer_name": "nombre del comercio o null si no es visible",
  "ruc": "número RUC completo o null",
  "dv": "dígito verificador o null",
  "address": null,
  "invoice_number": "número de factura o null",
  "date": null,
  "total": valor_numerico_o_null,
  "products": [
    {{
      "name": "descripción del producto",
      "quantity": cantidad_numerica,
      "unit_price": precio_unitario,
      "total_price": precio_total
    }}
  ]
}}

IMPORTANTE:
1. ENFÓCATE ESPECÍFICAMENTE en los campos listados arriba
2. Si el campo NO está visible en la imagen, usa null (no inventes datos)
3. Para productos, extrae TODOS los que sean visibles
4. Solo responde con el JSON, sin texto adicional"#, field_instructions = field_instructions);

        let payload = json!({
            "contents": [{
                "parts": [
                    {"text": prompt},
                    {
                        "inline_data": {
                            "mime_type": "image/jpeg",
                            "data": image_base64
                        }
                    }
                ]
            }],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 2048
            }
        });

        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}", api_key);
        
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Error en request a Gemini RETRY: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Error en Gemini API RETRY: {} - {}", status, error_text));
        }

        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Error parseando respuesta de Gemini RETRY: {}", e))?;

        info!("🔍 GEMINI RETRY RESPONSE: {}", serde_json::to_string_pretty(&response_json).unwrap_or_default());

        let text = response_json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("No se pudo extraer texto de respuesta Gemini RETRY"))?;

        let cleaned_text = Self::extract_json_from_markdown(text);
        
        let ocr_response: OcrResponse = serde_json::from_str(&cleaned_text)
            .map_err(|e| anyhow!("Error parseando JSON de OCR RETRY: {} - Texto: {}", e, cleaned_text))?;

        info!("✅ OCR RETRY procesado - RUC: {:?}, DV: {:?}, Invoice: {:?}, Total: {:?}, Products: {}", 
            ocr_response.ruc, ocr_response.dv, ocr_response.invoice_number, 
            ocr_response.total, ocr_response.products.len());

        Ok(ocr_response)
    }
}

// Data structures for database transformations

#[derive(Debug)]
struct InvoiceHeaderData {
    cufe: String,
    issuer_name: String,
    no: String,
    date: chrono::NaiveDateTime, // Timestamp GMT-5
    tot_amount: f64,
    issuer_ruc: Option<String>,
    issuer_dv: Option<String>,
    issuer_address: Option<String>,
    type_field: String,
    origin: String,
    user_id: i64,
    user_ws: Option<String>, // Parámetro opcional
    user_email: String,
    url: String,
    tot_itbms: f64,
    time: String,
    process_date: DateTime<Utc>,
    reception_date: DateTime<Utc>,
}

#[derive(Debug)]
struct InvoiceDetailData {
    cufe: String,
    partkey: String,
    code: String,
    description: String,
    information_of_interest: String,
    quantity: String,
    unit_price: String,
    unit_discount: String,
    amount: String,
    itbms: String,
    total: String,
    date: String,
}

#[derive(Debug)]
struct InvoicePaymentData {
    cufe: String,
    total_pagado: String,
    forma_de_pago: String,
    efectivo: String,
    valor_pago: String,
}
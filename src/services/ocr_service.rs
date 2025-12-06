use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};

use base64::{Engine as _, engine::general_purpose};
use serde_json::{json, Value};
use reqwest::Client;
use chrono::{DateTime, Utc};

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
}

/// Product details in OCR response
#[derive(Debug, serde::Serialize, Clone)]
pub struct OcrProductResponse {
    pub name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total_price: f64,
    pub partkey: Option<String>,
}

/// Main OCR processing service
pub struct OcrService;

impl OcrService {
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
                });
            }
        }
        
        // 4. OCR sin costo en Lümis (por ahora)
        let ocr_cost = 0;

        // 6. Process with OCR (Gemini primary, OpenRouter fallback)
        let ocr_response = match Self::process_image_with_ocr(&request.image_bytes, &request.mode).await {
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
                });
            }
        };

        // 7. Validate required fields
        if let Err(e) = Self::validate_required_fields(&ocr_response) {
            warn!("Validación de campos falló para {}: {}", request.user_identifier, e);
            
            Self::log_ocr_attempt(&state, &request.user_identifier, "field_validation_failed", &e.to_string()).await?;
            
            // Devolver datos parciales que SÍ fueron extraídos
            let partial_products: Vec<OcrProductResponse> = ocr_response.products.iter().map(|p| {
                OcrProductResponse {
                    name: p.name.clone(),
                    quantity: p.quantity,
                    unit_price: p.unit_price,
                    total_price: p.total_price,
                    partkey: None,
                }
            }).collect();
            
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
                message: e.to_string(),
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

    /// Process image with Gemini OCR
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

    /// Process image with OpenRouter as fallback (Qwen3-VL-30B model)
    async fn process_image_with_openrouter(image_bytes: &[u8], mode: &OcrMode) -> Result<OcrResponse> {
        info!("🔄 FALLBACK: Iniciando procesamiento con OpenRouter (Qwen3-VL-30B)");
        
        // OpenRouter API key (fallback)
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .unwrap_or_else(|_| "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".to_string());

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

    /// Process image with OCR - tries Gemini first, falls back to OpenRouter
    async fn process_image_with_ocr(image_bytes: &[u8], mode: &OcrMode) -> Result<OcrResponse> {
        // Try Gemini first
        match Self::process_image_with_gemini(image_bytes, mode).await {
            Ok(response) => {
                info!("✅ OCR procesado exitosamente con Gemini");
                Ok(response)
            }
            Err(gemini_error) => {
                warn!("⚠️ Gemini OCR falló: {}. Intentando fallback con OpenRouter...", gemini_error);
                
                // Try OpenRouter as fallback
                match Self::process_image_with_openrouter(image_bytes, mode).await {
                    Ok(response) => {
                        info!("✅ OCR procesado exitosamente con OpenRouter (fallback)");
                        Ok(response)
                    }
                    Err(openrouter_error) => {
                        error!("❌ Ambos proveedores OCR fallaron. Gemini: {} | OpenRouter: {}", gemini_error, openrouter_error);
                        Err(anyhow!("OCR falló en todos los proveedores. Gemini: {} | OpenRouter: {}", gemini_error, openrouter_error))
                    }
                }
            }
        }
    }

    /// Validate required fields and collect all missing fields
    fn validate_required_fields(ocr_response: &OcrResponse) -> Result<()> {
        let mut missing_fields: Vec<String> = Vec::new();
        
        // Validar nombre del comercio
        if let Some(name) = &ocr_response.issuer_name {
            if name.trim().is_empty() {
                missing_fields.push("nombre del comercio".to_string());
            }
        } else {
            missing_fields.push("nombre del comercio".to_string());
        }
        
        // Validar RUC
        if let Some(ruc) = &ocr_response.ruc {
            if ruc.trim().is_empty() {
                missing_fields.push("RUC".to_string());
            }
        } else {
            missing_fields.push("RUC".to_string());
        }
        
        // Validar fecha
        if let Some(date_str) = &ocr_response.date {
            if date_str.trim().is_empty() {
                missing_fields.push("fecha".to_string());
            }
        } else {
            missing_fields.push("fecha".to_string());
        }
        
        // Validar monto total
        if let Some(total) = ocr_response.total {
            if total <= 0.0 {
                missing_fields.push("monto total válido".to_string());
            }
        } else {
            missing_fields.push("monto total".to_string());
        }
        
        // Validar productos
        if ocr_response.products.is_empty() {
            missing_fields.push("productos".to_string());
        } else {
            // Validar campos de cada producto
            let mut product_issues = Vec::new();
            for (i, product) in ocr_response.products.iter().enumerate() {
                let mut product_missing = Vec::new();
                
                if product.name.trim().is_empty() {
                    product_missing.push("descripción");
                }
                if product.quantity <= 0.0 {
                    product_missing.push("cantidad válida");
                }
                if product.unit_price < 0.0 {
                    product_missing.push("precio unitario válido");
                }
                
                if !product_missing.is_empty() {
                    product_issues.push(format!("Producto {}: {}", i + 1, product_missing.join(", ")));
                }
            }
            
            if !product_issues.is_empty() {
                missing_fields.push(format!("datos de productos ({})", product_issues.join("; ")));
            }
        }
        
        // Si hay campos faltantes, devolver error con mensaje específico
        if !missing_fields.is_empty() {
            let error_message = if missing_fields.len() == 1 {
                format!("Por favor, vuelve a subir una factura donde se pueda ver claramente el campo: {}", missing_fields[0])
            } else {
                format!("Por favor, vuelve a subir una factura donde se puedan ver claramente los siguientes campos: {}", missing_fields.join(", "))
            };
            return Err(anyhow!(error_message));
        }
        
        Ok(())
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
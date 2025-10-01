use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose};
use serde_json::{json, Value};
use reqwest::Client;

use crate::{
    models::{ocr::*, invoice::*},
    services::{user_service, rewards_service},
    state::AppState,
};

/// Servicio para procesamiento OCR con Gemini LLM
pub struct OcrProcessingService;

impl OcrProcessingService {
    /// Procesa imagen con Gemini LLM
    pub async fn process_image_with_gemini(
        image_data: &[u8],
        focus_fields: Option<Vec<String>>,
    ) -> Result<InvoiceData> {
        // Mock implementation for now
        Ok(InvoiceData {
            issuer_name: Some("Test Comercio".to_string()),
            issuer_ruc: Some("123456789".to_string()),
            invoice_number: Some("F001-123456".to_string()),
            date: Some("2025-09-05".to_string()),
            total: Some(100.0),
            tax_amount: Some(15.0),
            products: vec![],
        })
    }
    
    /// Valida campos obligatorios
    pub fn validate_required_fields(invoice_data: &InvoiceData) -> Result<Vec<String>> {
        let mut missing = Vec::new();
        
        if invoice_data.issuer_name.is_none() { missing.push("issuer_name".to_string()); }
        if invoice_data.invoice_number.is_none() { missing.push("invoice_number".to_string()); }
        if invoice_data.date.is_none() { missing.push("date".to_string()); }
        if invoice_data.total.is_none() { missing.push("total".to_string()); }
        if invoice_data.products.is_empty() { missing.push("products".to_string()); }
        
        Ok(missing)
    }
    
    /// Genera CUFE para la factura
    pub fn generate_cufe(invoice_data: &InvoiceData) -> String {
        format!("cufe_{}", Uuid::new_v4().simple())
    }
    
    /// Verifica facturas duplicadas
    pub async fn check_duplicate_invoice(
        state: &Arc<AppState>,
        invoice_number: &str,
        issuer_name: &str,
        total: f64,
        user_id: i64,
    ) -> Result<Option<String>> {
        let existing_cufe = sqlx::query_scalar!(
            r#"
            SELECT cufe FROM public.invoice_header 
            WHERE no = $1 AND issuer_name ILIKE $2 AND tot_amount = $3 AND user_id = $4
            LIMIT 1
            "#,
            invoice_number,
            issuer_name,
            total,
            user_id as i32
        )
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| anyhow!("Error verificando duplicados: {}", e))?;
        
        Ok(existing_cufe)
    }
    
    /// Guarda factura en base de datos
    pub async fn save_invoice_to_database(
        state: &Arc<AppState>,
        invoice_data: &InvoiceData,
        cufe: &str,
        user_id: i64,
        processing_method: &ProcessingMethod,
        image_data: Option<&[u8]>,
    ) -> Result<i64> {
        let mut tx = state.db_pool.begin().await
            .map_err(|e| anyhow!("Error iniciando transacción: {}", e))?;

        // 1. Transform to header data
        let header_data = Self::transform_to_header(invoice_data, cufe, user_id, &processing_method)?;
        
        // 2. Insert header
        let _cufe_result = sqlx::query_scalar!(
            r#"
            INSERT INTO public.invoice_header (
                cufe, issuer_name, no, date, tot_amount, issuer_ruc, type, origin, 
                user_id, process_date, reception_date
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING cufe
            "#,
            header_data.cufe,
            header_data.issuer_name,
            header_data.no,
            header_data.date,
            header_data.tot_amount,
            header_data.issuer_ruc,
            header_data.type_,
            header_data.origin,
            header_data.user_id,
            header_data.process_date,
            header_data.reception_date
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando invoice_header: {}", e))?;

        // Get the actual invoice ID for details
        let invoice_id = sqlx::query_scalar!(
            "SELECT id FROM public.invoice_header WHERE cufe = $1",
            cufe
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error obteniendo invoice_id: {}", e))?;

        // 3. Insert products
        for product in &invoice_data.products {
            sqlx::query!(
                r#"
                INSERT INTO public.invoice_detail (
                    invoice_header_id, product_name, quantity, unit_price, total_price
                ) VALUES ($1, $2, $3, $4, $5)
                "#,
                invoice_id,
                product.name,
                product.quantity,
                product.unit_price,
                product.total
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Error insertando invoice_detail: {}", e))?;
        }

        // 4. Insert payment data
        sqlx::query!(
            r#"
            INSERT INTO public.invoice_payment (
                cufe, total_pagado, forma_de_pago
            ) VALUES ($1, $2, $3)
            "#,
            cufe,
            header_data.tot_amount.to_string(),
            "unknown"
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando invoice_payment: {}", e))?;
        
        tx.commit().await
            .map_err(|e| anyhow!("Error confirmando transacción: {}", e))?;
        
        info!("✅ Factura guardada exitosamente: {} (ID: {:?})", cufe, invoice_id);
        Ok(invoice_id.unwrap_or(0))
    }
    
    /// Log de procesamiento OCR para analytics
    pub async fn log_ocr_processing(
        state: &Arc<AppState>,
        user_id: i64,
        session_id: &str,
        tokens_used: i32,
        cost_usd: f64,
        success: bool,
        processing_time_ms: i32,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO analytics.ocr_token_usage (
                user_id, session_id, tokens_used, lumis_cost, success, 
                processing_time_ms, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            user_id,
            session_id,
            tokens_used,
            cost_usd as f32,
            success,
            processing_time_ms,
            chrono::Utc::now().naive_utc()
        )
        .execute(&state.db_pool)
        .await
        .map_err(|e| anyhow!("Error logging OCR processing: {}", e))?;
        
        Ok(())
    }
    
    /// Transforma datos a estructura de header
    fn transform_to_header(
        invoice_data: &InvoiceData,
        cufe: &str,
        user_id: i64,
        _processing_method: &ProcessingMethod,
    ) -> Result<InvoiceHeader> {
        Ok(InvoiceHeader {
            id: None,
            cufe: cufe.to_string(),
            issuer_name: invoice_data.issuer_name.clone().unwrap_or_default(),
            no: invoice_data.invoice_number.clone().unwrap_or_default(),
            date: invoice_data.date.clone().unwrap_or_default(),
            tot_amount: invoice_data.total.unwrap_or(0.0),
            issuer_ruc: invoice_data.issuer_ruc.clone().unwrap_or_default(),
            type_: "FACTURA".to_string(),
            origin: "OCR_ITERATIVO".to_string(),
            user_id: user_id as i32,
            process_date: chrono::Utc::now().naive_utc(),
            reception_date: chrono::Utc::now().naive_utc(),
        })
    }
}
    state::AppState,
};

/// Servicio abstracto para procesamiento OCR que reutiliza lógica de factura_sin_qr
pub struct OcrProcessingService;

impl OcrProcessingService {
    /// Procesar imagen con Gemini OCR usando prompt específico
    pub async fn process_image_with_gemini(
        image_bytes: &[u8],
        prompt: Option<String>,
    ) -> Result<InvoiceData> {
        info!("🤖 Iniciando procesamiento con Gemini OCR");
        
        // Get Gemini API key from environment
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("GEMINI_API_KEY no configurada en variables de entorno"))?;
        
        // Encode image to base64
        let image_base64 = general_purpose::STANDARD.encode(image_bytes);
        
        // Use custom prompt or default
        let ocr_prompt = prompt.unwrap_or_else(|| Self::get_default_prompt());
        
        // Prepare the request payload
        let payload = json!({
            "contents": [{
                "parts": [
                    {
                        "inline_data": {
                            "mime_type": "image/jpeg",
                            "data": image_base64
                        }
                    },
                    {
                        "text": ocr_prompt
                    }
                ]
            }]
        });
        
        // Make the API call
        let client = Client::new();
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}", api_key);
        
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Error en llamada a Gemini API: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Gemini API error {}: {}", status, error_text));
        }
        
        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Error parsing Gemini response: {}", e))?;
        
        // Extract the text from Gemini's response structure
        let response_text = response_json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("Formato de respuesta de Gemini inesperado"))?;
        
        info!("📄 Respuesta de Gemini: {}", response_text);
        
        // Clean the response (remove markdown if present)
        let cleaned_text = if response_text.starts_with("```json") {
            response_text.replace("```json", "").replace("```", "").trim().to_string()
        } else if response_text.starts_with("```") {
            response_text.replace("```", "").trim().to_string()
        } else {
            response_text.trim().to_string()
        };
        
        // Parse the JSON response
        let gemini_response: Value = serde_json::from_str(&cleaned_text)
            .map_err(|e| anyhow!("Error parsing JSON from Gemini: {}. Raw response: {}", e, cleaned_text))?;
        
        // Convert Gemini response to our InvoiceData structure
        let invoice_data = Self::gemini_response_to_invoice_data(gemini_response)?;
        
        info!("✅ OCR Gemini exitoso: {:?}", invoice_data.issuer_name);
        Ok(invoice_data)
    }
    
    /// Validar que los campos requeridos estén presentes
    pub fn validate_required_fields(invoice_data: &InvoiceData) -> Result<()> {
        let missing = invoice_data.get_missing_fields();
        if !missing.is_empty() {
            return Err(anyhow!("Campos faltantes: {}", missing.join(", ")));
        }
        Ok(())
    }
    
    /// Generar CUFE temporal para la factura
    pub async fn generate_cufe(invoice_data: &InvoiceData, user_id: i64, processing_method: ProcessingMethod) -> Result<String> {
        let uuid = Uuid::new_v4();
        let method_prefix = match processing_method {
            ProcessingMethod::OcrIterative => "OCR-IT",
            ProcessingMethod::OcrSingle => "OCR-SG",
            _ => "OCR",
        };
        
        let invoice_num = invoice_data.invoice_number
            .as_ref()
            .map(|n| n.replace([' ', '-'], ""))
            .unwrap_or_else(|| "UNKNOWN".to_string());
        
        let cufe = format!(
            "{}-{}-{}-{}",
            method_prefix,
            user_id,
            invoice_num,
            &uuid.to_string()[..8]
        );
        Ok(cufe)
    }
    
    /// Guardar factura en base de datos (abstrae lógica de factura_sin_qr)
    pub async fn save_invoice_to_database(
        state: &Arc<AppState>,
        invoice_data: &InvoiceData,
        cufe: &str,
        user_id: i64,
        processing_method: ProcessingMethod,
        image_data: Option<&[u8]>,
    ) -> Result<i64> {
        info!("💾 Guardando factura en base de datos: {}", cufe);
        
        let mut tx = state.db_pool.begin().await
            .map_err(|e| anyhow!("Error iniciando transacción: {}", e))?;
        
        // 1. Transform and insert invoice header
        let header_data = Self::transform_to_header(invoice_data, cufe, user_id, &processing_method)?;
        
        let cufe_result = sqlx::query_scalar!(
            r#"
            INSERT INTO public.invoice_header (
                cufe, issuer_name, no, date, tot_amount, issuer_ruc, type, origin, 
                user_id, process_date, reception_date
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING cufe
            "#,
            header_data.cufe,
            header_data.issuer_name,
            header_data.no,
            header_data.date,
            header_data.tot_amount,
            header_data.issuer_ruc,
            "invoice",
            processing_method.to_string(),
            user_id as i32,
            chrono::Utc::now(),
            chrono::Utc::now()
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando invoice_header: {}", e))?;
        
        // 2. Insert invoice details
        for (index, product) in invoice_data.products.iter().enumerate() {
            sqlx::query!(
                r#"
                INSERT INTO public.invoice_detail (
                    cufe, partkey, description, 
                    quantity, unit_price, total, date
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                cufe,
                format!("PROD-{}", index + 1),
                product.name,
                product.quantity.to_string(),
                product.unit_price.to_string(),
                product.total_price.to_string(),
                header_data.date.format("%Y-%m-%d").to_string()
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Error insertando invoice_detail: {}", e))?;
        }
        
        // 3. Insert payment data
        sqlx::query!(
            r#"
            INSERT INTO public.invoice_payment (
                cufe, total_pagado, forma_de_pago
            ) VALUES ($1, $2, $3)
            "#,
            cufe,
            header_data.tot_amount.to_string(),
            "unknown"
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando invoice_payment: {}", e))?;
        
        // 4. Save image if provided (skip for now as table doesn't exist)
        // if let Some(img_bytes) = image_data {
        //     let image_base64 = general_purpose::STANDARD.encode(img_bytes);
        //     // TODO: Create invoice_images table or save elsewhere
        // }
        
        // Commit the transaction
        tx.commit().await
            .map_err(|e| anyhow!("Error confirmando transacción: {}", e))?;
        
        info!("✅ Factura guardada exitosamente: {} (ID: {:?})", cufe, invoice_id);
        Ok(invoice_id.unwrap_or(0))
    }
    
    /// Log de procesamiento OCR para analytics
    pub async fn log_ocr_processing(
        state: &Arc<AppState>,
        user_id: i64,
        session_id: Option<&str>,
        tokens_used: i32,
        success: bool,
        processing_time: std::time::Duration,
        details: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO analytics.ocr_token_usage (
                user_id, session_id, tokens_used, lumis_cost, success, 
                processing_time_ms, details, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            user_id,
            session_id,
            tokens_used,
            0, // Lumis cost actualmente 0
            success,
            processing_time.as_millis() as i32,
            details,
            chrono::Utc::now().naive_utc()
        )
        .execute(&state.db_pool)
        .await
        .map_err(|e| anyhow!("Error logging OCR processing: {}", e))?;
        
        Ok(())
    }
    
    /// Verificar si la factura es duplicada
    pub async fn check_duplicate_invoice(
        state: &Arc<AppState>,
        invoice_data: &InvoiceData,
        user_id: i64,
    ) -> Result<Option<String>> {
        if let (Some(invoice_number), Some(issuer_name), Some(total)) = 
            (&invoice_data.invoice_number, &invoice_data.issuer_name, invoice_data.total) {
            
            let existing_cufe = sqlx::query_scalar!(
                r#"
                SELECT cufe FROM public.invoice_header 
                WHERE no = $1 AND issuer_name ILIKE $2 AND tot_amount = $3 AND user_id = $4
                LIMIT 1
                "#,
                invoice_number,
                issuer_name,
                total,
                user_id as i32
            )
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| anyhow!("Error verificando duplicados: {}", e))?;
            
            Ok(existing_cufe)
        } else {
            Ok(None)
        }
    }
    
    // Private helper methods
    
    fn get_default_prompt() -> String {
        r#"Analiza esta imagen de factura y extrae la siguiente información en formato JSON:
{
  "issuer_name": "Nombre del comercio/empresa",
  "invoice_number": "Número de factura",
  "date": "Fecha en formato YYYY-MM-DD",
  "total": 0.0,
  "products": [
    {
      "name": "Nombre del producto",
      "quantity": 1.0,
      "unit_price": 0.0,
      "total_price": 0.0
    }
  ],
  "rif": "RIF/NIT si está visible",
  "address": "Dirección si está visible",
  "subtotal": 0.0,
  "tax": 0.0
}

Extrae TODOS los datos visibles. Si un campo no está claro, usa null.
Responde ÚNICAMENTE el JSON válido sin explicaciones adicionales."#.to_string()
    }
    
    fn gemini_response_to_invoice_data(response: Value) -> Result<InvoiceData> {
        let mut invoice_data = InvoiceData::empty();
        
        invoice_data.issuer_name = response.get("issuer_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
            
        invoice_data.invoice_number = response.get("invoice_number")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
            
        invoice_data.date = response.get("date")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
            
        invoice_data.total = response.get("total")
            .and_then(|v| v.as_f64())
            .filter(|&t| t > 0.0);
            
        invoice_data.rif = response.get("rif")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
            
        invoice_data.address = response.get("address")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
            
        invoice_data.subtotal = response.get("subtotal")
            .and_then(|v| v.as_f64());
            
        invoice_data.tax = response.get("tax")
            .and_then(|v| v.as_f64());
        
        // Parse products
        if let Some(products_array) = response.get("products").and_then(|v| v.as_array()) {
            invoice_data.products = products_array.iter().filter_map(|p| {
                Some(ProductData {
                    name: p.get("name")?.as_str()?.to_string(),
                    quantity: p.get("quantity")?.as_f64().unwrap_or(1.0),
                    unit_price: p.get("unit_price")?.as_f64().unwrap_or(0.0),
                    total_price: p.get("total_price")?.as_f64().unwrap_or(0.0),
                    line_number: None,
                })
            }).collect();
        }
        
        Ok(invoice_data)
    }
    
    fn transform_to_header(
        invoice_data: &InvoiceData,
        cufe: &str,
        user_id: i64,
        processing_method: &ProcessingMethod,
    ) -> Result<InvoiceHeaderData> {
        let date_str = invoice_data.date.as_ref()
            .ok_or_else(|| anyhow!("Fecha requerida"))?;
        
        let parsed_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| anyhow!("Formato de fecha inválido: {}", date_str))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow!("Error creando datetime"))?;
        
        Ok(InvoiceHeaderData {
            cufe: cufe.to_string(),
            issuer_name: invoice_data.issuer_name.clone().unwrap_or_default(),
            no: invoice_data.invoice_number.clone().unwrap_or_default(),
            date: parsed_date,
            tot_amount: invoice_data.total.unwrap_or(0.0),
            issuer_ruc: invoice_data.rif.clone(),
        })
    }
}

// Helper structures
#[derive(Debug)]
struct InvoiceHeaderData {
    cufe: String,
    issuer_name: String,
    no: String,
    date: chrono::NaiveDateTime,
    tot_amount: f64,
    issuer_ruc: Option<String>,
}

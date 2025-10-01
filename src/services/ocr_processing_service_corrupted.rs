use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;
use base64::{Engine as _, prelude::BASE64_STANDARD}    /// Generar CUFE para la factura
    pub fn generate_cufe(data: &InvoiceData) -> String {
        let source = format!("{}-{}-{}-{}", 
            data.issuer_name.as_deref().unwrap_or(""),
            data.invoice_number.as_deref().unwrap_or(""), 
            data.date.as_deref().unwrap_or(""), 
            data.total.unwrap_or(0.0)
        );
        
        // Usar UUID v4 ya que new_v5 no está disponible en esta versión
        format!("{}", Uuid::new_v4())
    }serde_json::{json, Value};
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
        info!("Iniciando procesamiento OCR con Gemini");
        
        let prompt = Self::generate_prompt(&focus_fields);
        let base64_image = BASE64_STANDARD.encode(image_data);
        
        let client = Client::new();
        let payload = json!({
            "contents": [{
                "parts": [
                    {"text": prompt},
                    {
                        "inline_data": {
                            "mime_type": "image/jpeg",
                            "data": base64_image
                        }
                    }
                ]
            }],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 2048
            }
        });

        let response = client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent")
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", std::env::var("GEMINI_API_KEY")?)
            .json(&payload)
            .send()
            .await?;

        let response_text = response.text().await?;
        info!("Respuesta de Gemini recibida");

        // Parse response JSON
        let parsed: Value = serde_json::from_str(&response_text)?;
        let content = parsed["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("No se pudo extraer texto de la respuesta"))?;

        // Parse invoice data from JSON
        let invoice_data: InvoiceData = serde_json::from_str(content)?;
        
        Ok(invoice_data)
    }

    /// Genera prompt para OCR enfocado en campos específicos
    fn generate_prompt(focus_fields: &Option<Vec<String>>) -> String {
        let base_prompt = r#"
Extrae los datos de esta factura y devuelve un JSON válido con esta estructura exacta:
{
  "issuer_name": "string",
  "rif": "string",
  "no": "string",
  "date": "YYYY-MM-DD",
  "tot_amount": number,
  "tax": number,
  "products": [
    {
      "product_code": "string",
      "product_description": "string", 
      "quantity": number,
      "unit_price": number
    }
  ]
}

Reglas importantes:
- Usa SOLO la estructura JSON mostrada arriba
- Los números deben ser decimales válidos, no strings
- Las fechas en formato YYYY-MM-DD
- Si no encuentras un campo, usa null
"#;

        match focus_fields {
            Some(fields) if !fields.is_empty() => {
                format!("{}\n\nENFOCATE ESPECIALMENTE en estos campos: {}", 
                       base_prompt, fields.join(", "))
            },
            _ => base_prompt.to_string()
        }
    }

    /// Validar campos requeridos antes de guardar
    pub fn validate_required_fields(data: &InvoiceData) -> Result<()> {
        let mut errors = Vec::new();
        
        if data.issuer_name.as_ref().map_or(true, |s| s.trim().is_empty()) {
            errors.push("issuer_name");
        }
        if data.invoice_number.as_ref().map_or(true, |s| s.trim().is_empty()) {
            errors.push("invoice_number");
        }
        if data.date.as_ref().map_or(true, |s| s.trim().is_empty()) {
            errors.push("date");
        }
        if data.total.map_or(true, |t| t <= 0.0) {
            errors.push("total");
        }
        if data.products.is_empty() {
            errors.push("products");
        }
        
        if !errors.is_empty() {
            return Err(anyhow!("Missing required fields: {}", errors.join(", ")));
        }
        
        Ok(())
    }

    /// Verificar si la factura ya existe (por RIF + número + fecha)
    pub async fn check_duplicate_invoice(state: &Arc<AppState>, data: &InvoiceData) -> Result<Option<String>> {
        let query_result = sqlx::query!(
            "SELECT cufe FROM public.invoice_header WHERE issuer_ruc = $1 AND no = $2 AND date = $3 LIMIT 1",
            data.rif.as_deref(),
            data.invoice_number.as_deref(),
            data.date.as_deref()
        )
        .fetch_optional(&state.db_pool)
        .await?;

        match query_result {
            Some(row) => Ok(row.cufe),
            None => Ok(None),
        }
    }

    /// Genera CUFE único para la factura
    pub fn generate_cufe(data: &InvoiceData) -> String {
        let source = format!("{}-{}-{}-{}", 
                           data.issuer_name, data.no, data.date, data.tot_amount);
        format!("OCR-{}", Uuid::new_v5(&Uuid::NAMESPACE_DNS, source.as_bytes()))
    }

    /// Guarda factura completa en base de datos
    pub async fn save_invoice_to_database(
        state: &Arc<AppState>,
        data: &InvoiceData,
        cufe: &str,
        user_id: i64,
    ) -> Result<i32> {
        let mut tx = state.db_pool.begin().await?;

        // 1. Preparar datos
        let header_data = Self::build_header_data(data, cufe);
        let details_data = Self::build_details_data(&data.products);

        // 2. Insert header
        let _cufe_result = sqlx::query!(
            r#"
            INSERT INTO public.invoice_header (
                cufe, issuer_name, no, date, tot_amount, issuer_ruc, user_phone_number, time
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            cufe,
            data.issuer_name.as_deref().unwrap_or(""),
            data.invoice_number.as_deref().unwrap_or(""),
            data.date.as_deref().unwrap_or(""),
            data.total.unwrap_or(0.0),
            data.rif.as_deref(),
            "", // user_phone_number placeholder
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando invoice_header: {}", e))?;

        // Get cufe as ID since there's no auto-increment ID
        let invoice_cufe = cufe.to_string();

        // 3. Insert details usando las columnas reales
        for detail in &data.products {
            sqlx::query!(
                r#"
                INSERT INTO public.invoice_detail (
                    cufe, code, description, quantity, unit_price, total
                ) VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                invoice_cufe.clone(),
                format!("PROD-{}", detail.line_number.unwrap_or(1)), // code
                detail.name, // description
                detail.quantity.to_string(),
                detail.unit_price.to_string(),
                detail.total_price.to_string()
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Error insertando invoice_detail: {}", e))?;
        }

        // 4. Log analytics usando las columnas reales
        let tokens_used = 1500; // Placeholder
        sqlx::query!(
            r#"
            INSERT INTO analytics.ocr_token_usage (
                user_id, tokens_used, lumis_cost, success, processing_time_ms, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user_id,
            tokens_used,
            0.05, // placeholder cost
            true,
            500, // placeholder processing time
            chrono::Utc::now().naive_utc()
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Error insertando analytics: {}", e))?;
        tx.commit().await?;
        
        info!("Factura guardada exitosamente con CUFE: {}", cufe);
        Ok(1) // Return success indicator
    }
    
    /// Log de procesamiento OCR para analytics
    pub async fn log_ocr_processing(
        state: &Arc<AppState>,
        user_id: i64,
        tokens_used: i32,
        cost_usd: f64,
        success: bool,
        endpoint: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO analytics.ocr_token_usage (
                user_id, tokens_used, lumis_cost, success, processing_time_ms, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user_id,
            tokens_used,
            cost_usd,
            success,
            500, // placeholder processing time
            chrono::Utc::now().naive_utc()
        )
        .execute(&state.db_pool)
        .await
        .map_err(|e| anyhow!("Error logging OCR processing: {}", e))?;
        
        Ok(())
    }
}

use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;
use base64::{Engine as _, prelude::BASE64_STANDARD};
use serde_json::{json, Value};
use reqwest::Client;

use crate::{
    models::ocr::*,
    state::AppState,
};

/// Servicio para procesamiento OCR con Gemini LLM
pub struct OcrProcessingService;

impl OcrProcessingService {
    /// Procesa imagen con Gemini LLM
    pub async fn process_image_with_gemini(
        image_data: &[u8],
        _focus_fields: Option<Vec<String>>,
    ) -> Result<InvoiceData> {
        // Mock implementation for testing
        info!("Processing image with Gemini LLM");
        
        Ok(InvoiceData {
            issuer_name: Some("Test Store".to_string()),
            invoice_number: Some("F001-12345".to_string()),
            date: Some("2025-09-05".to_string()),
            total: Some(45.50),
            products: vec![
                ProductData {
                    name: "Test Product".to_string(),
                    quantity: 1.0,
                    unit_price: 45.50,
                    total_price: 45.50,
                    line_number: Some(1),
                }
            ],
            rif: Some("12345678-9".to_string()),
            address: None,
            subtotal: Some(40.0),
            tax: Some(5.50),
        })
    }

    /// Verificar duplicados por RIF + número + fecha
    pub async fn check_for_duplicates(
        state: &Arc<AppState>,
        data: &InvoiceData,
    ) -> Result<bool> {
        let existing = sqlx::query!(
            "SELECT 1 FROM public.invoice_header WHERE issuer_ruc = $1 AND no = $2 AND date = $3 LIMIT 1",
            data.rif.as_deref(),
            data.invoice_number.as_deref(),
            data.date.as_deref()
        )
        .fetch_optional(&state.db_pool)
        .await?;

        Ok(existing.is_some())
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

    /// Generar CUFE para la factura
    pub fn generate_cufe(data: &InvoiceData) -> String {
        let source = format!("{}-{}-{}-{}", 
            data.issuer_name.as_deref().unwrap_or(""),
            data.invoice_number.as_deref().unwrap_or(""), 
            data.date.as_deref().unwrap_or(""), 
            data.total.unwrap_or(0.0)
        );
        
        // Usar UUID v4 ya que new_v5 no está disponible en esta versión
        format!("{}", Uuid::new_v4())
    }

    /// Guardar factura en base de datos
    pub async fn save_invoice_to_database(
        state: &Arc<AppState>,
        data: &InvoiceData,
        cufe: &str,
        user_id: i64,
    ) -> Result<i32> {
        let mut tx = state.db_pool.begin().await?;
        
        // 1. Validate data
        Self::validate_required_fields(data)?;

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

        // 3. Insert details usando las columnas reales
        for detail in &data.products {
            sqlx::query!(
                r#"
                INSERT INTO public.invoice_detail (
                    cufe, code, description, quantity, unit_price, total
                ) VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                cufe,
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

        // 5. Commit transaction
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
        _endpoint: &str,
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

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
        let _cufe_result = sqlx::query!(
            r#"
            INSERT INTO public.invoice_header (
                cufe, issuer_name, no, date, tot_amount, issuer_ruc, type, origin, 
                user_id, process_date, reception_date
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
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
    ) -> Result<InvoiceHeaderData> {
        Ok(InvoiceHeaderData {
            cufe: cufe.to_string(),
            issuer_name: invoice_data.issuer_name.clone().unwrap_or_default(),
            no: invoice_data.invoice_number.clone().unwrap_or_default(),
            date: invoice_data.date.clone().unwrap_or_default(),
            tot_amount: invoice_data.total.unwrap_or(0.0),
            issuer_ruc: invoice_data.issuer_ruc.clone(),
            type_: "FACTURA".to_string(),
            origin: "OCR_ITERATIVO".to_string(),
            user_id: user_id as i32,
            process_date: chrono::Utc::now().naive_utc(),
            reception_date: chrono::Utc::now().naive_utc(),
        })
    }
}

/// Helper struct para operaciones de BD
#[derive(Debug)]
struct InvoiceHeaderData {
    cufe: String,
    issuer_name: String,
    no: String,
    date: String,
    tot_amount: f64,
    issuer_ruc: Option<String>,
    type_: String,
    origin: String,
    user_id: i32,
    process_date: chrono::NaiveDateTime,
    reception_date: chrono::NaiveDateTime,
}

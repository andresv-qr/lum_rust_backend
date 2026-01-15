use sqlx::PgPool;
use tracing::{info, warn, error as log_error};
use chrono::{NaiveDateTime, DateTime, Utc, TimeZone};
use chrono_tz::America::Panama;

use crate::api::webscraping::{InvoiceHeader, InvoiceDetail, InvoicePayment, ScrapingResult};
use crate::api::templates::url_processing_templates::ProcessUrlResponse;

// ============================================================================
// DATE UTILITIES
// ============================================================================

/// Convierte fecha en formato DD/MM/YYYY HH:MM:SS (hora local de Panamá)
/// a DateTime<Utc> para almacenamiento correcto en PostgreSQL TIMESTAMPTZ.
/// 
/// Las facturas de DGI/MEF de Panamá vienen en hora local de Panamá (UTC-5).
/// Esta función interpreta la fecha como hora de Panamá y la convierte a UTC.
/// 
/// Ejemplo: "25/06/2025 14:30:00" (Panamá) → "2025-06-25 19:30:00+00" (UTC)
fn convert_panama_date_to_utc(date_str: &str) -> Option<DateTime<Utc>> {
    // Intentar parsear formato: DD/MM/YYYY HH:MM:SS
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S") {
        // Interpretar como hora de Panamá y convertir a UTC
        match Panama.from_local_datetime(&naive_dt) {
            chrono::LocalResult::Single(panama_dt) => {
                return Some(panama_dt.with_timezone(&Utc));
            }
            chrono::LocalResult::Ambiguous(earliest, _) => {
                // En caso de ambigüedad (cambio de horario), usar la más temprana
                warn!("⚠️ Ambiguous datetime for Panama: {}, using earliest", date_str);
                return Some(earliest.with_timezone(&Utc));
            }
            chrono::LocalResult::None => {
                // Hora inválida (gap por cambio de horario)
                warn!("⚠️ Invalid datetime for Panama timezone: {}", date_str);
            }
        }
    }
    
    // Intentar parsear formato: DD/MM/YYYY (sin hora) - asumir medianoche
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%d/%m/%Y") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0)?;
        match Panama.from_local_datetime(&naive_dt) {
            chrono::LocalResult::Single(panama_dt) => {
                return Some(panama_dt.with_timezone(&Utc));
            }
            chrono::LocalResult::Ambiguous(earliest, _) => {
                return Some(earliest.with_timezone(&Utc));
            }
            chrono::LocalResult::None => {
                warn!("⚠️ Invalid date for Panama timezone: {}", date_str);
            }
        }
    }
    
    // Si no se puede parsear, log y retornar None
    warn!("⚠️ Could not parse date format: {}", date_str);
    None
}

// ============================================================================
// MAIN PERSISTENCE FUNCTION
// ============================================================================

pub async fn persist_scraped_data(
    db_pool: &PgPool,
    scraping_result: ScrapingResult,
    source_url: &str,
) -> Result<ProcessUrlResponse, ProcessUrlResponse> {
    if !scraping_result.success {
        let error_msg = scraping_result
            .error_message
            .unwrap_or_else(|| "Error desconocido al extraer datos".to_string());
        warn!(
            "Scraping failed for URL '{}', not persisting: {}",
            source_url, error_msg
        );
        return Err(ProcessUrlResponse::error(&error_msg));
    }

    let header = match scraping_result.header {
        Some(h) => h,
        None => {
            return Err(ProcessUrlResponse::error("Faltan datos de la factura"));
        }
    };

    // VALIDATION: Ensure we have critical data before saving as valid invoice
    // These validations match the WhatsApp flow in data_parser.rs
    // If any are missing, it means the invoice is not yet fully available in MEF
    
    // 1. Validate tot_amount exists and is greater than 0
    let has_valid_amount = header.tot_amount
        .map(|amt| amt > 0.0)
        .unwrap_or(false);
    
    // 2. Validate issuer_name exists and is not empty
    let has_valid_issuer_name = header.issuer_name
        .as_ref()
        .map(|name| !name.trim().is_empty())
        .unwrap_or(false);
    
    // 3. Validate issuer_ruc exists and is not empty
    let has_valid_issuer_ruc = header.issuer_ruc
        .as_ref()
        .map(|ruc| !ruc.trim().is_empty())
        .unwrap_or(false);
    
    // 4. Validate invoice number (no) exists and is not empty
    let has_valid_no = header.no
        .as_ref()
        .map(|no| !no.trim().is_empty())
        .unwrap_or(false);
    
    // 5. Validate date exists and is not empty
    let has_valid_date = header.date
        .as_ref()
        .map(|date| !date.trim().is_empty())
        .unwrap_or(false);
    
    if !has_valid_amount || !has_valid_issuer_name || !has_valid_issuer_ruc || !has_valid_no || !has_valid_date {
        let missing_fields: Vec<&str> = [
            (!has_valid_amount, "monto"),
            (!has_valid_issuer_name, "nombre del emisor"),
            (!has_valid_issuer_ruc, "RUC del emisor"),
            (!has_valid_no, "número de factura"),
            (!has_valid_date, "fecha"),
        ]
        .iter()
        .filter(|(missing, _)| *missing)
        .map(|(_, field)| *field)
        .collect();
        
        warn!("⚠️ Invoice data incomplete. Missing fields: {:?}. CUFE: {}", missing_fields, header.cufe);
        return Err(ProcessUrlResponse::error("Factura no disponible: Datos incompletos en MEF"));
    }

    let mut tx = match db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            log_error!("Failed to begin transaction: {}", e);
            return Err(ProcessUrlResponse::error("Error de transacción en base de datos"));
        }
    };

    // CORRECTED: Check if invoice already exists (fixed table name)
    match sqlx::query!("SELECT cufe FROM invoice_header WHERE cufe = $1", &header.cufe)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(_record)) => {
            return Err(ProcessUrlResponse::error("Factura duplicada detectada"));
        }
        Ok(None) => (),
        Err(e) => {
            log_error!("Failed to check for duplicate CUFE: {}", e);
            return Err(ProcessUrlResponse::error("Error de base de datos"));
        }
    }

    let cufe = match save_invoice_header(&mut tx, &header).await {
        Ok(id) => id,
        Err(e) => {
            log_error!("❌ Failed to save invoice header: {:?}", e);
            log_error!("❌ Error details - CUFE: {}, RUC: {:?}, Name: {:?}", 
                      header.cufe, header.issuer_ruc, header.issuer_name);
            return Err(ProcessUrlResponse::error(&format!("Error al guardar encabezado: {}", e)));
        }
    };

    // CORRECTED: No need to pass invoice_header_id (doesn't exist), relation is by CUFE
    if let Err(e) = save_invoice_details(&mut tx, &scraping_result.details).await {
        log_error!("Failed to save invoice details: {}", e);
        return Err(ProcessUrlResponse::error("Error al guardar detalles de factura"));
    }

    if let Err(e) = save_invoice_payments(&mut tx, &scraping_result.payments).await {
        log_error!("Failed to save invoice payments: {}", e);
        return Err(ProcessUrlResponse::error("Error al guardar pagos de factura"));
    }

    if let Err(e) = tx.commit().await {
        log_error!("Failed to commit transaction: {}", e);
        return Err(ProcessUrlResponse::error("Error al confirmar transacción"));
    }

    // 🔍 DEBUG: Log los valores antes de crear la respuesta
    info!("🔍 DEBUG - Creando respuesta con issuer_name: {:?}, tot_amount: {:?}", 
          header.issuer_name, header.tot_amount);

    Ok(ProcessUrlResponse::success(
        "API",
        None, // No invoice_id to return (doesn't exist)
        Some(cufe),
        0,
        header.issuer_name.clone(),
        header.tot_amount,
    ))
}

async fn save_invoice_header(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    header: &InvoiceHeader,
) -> Result<String, sqlx::Error> {
    info!("Saving invoice header with CUFE: {}", header.cufe);
    
    // Convertir fecha de Panamá (DD/MM/YYYY HH:MM:SS) a UTC
    // La fecha viene en hora local de Panamá y se convierte a UTC para almacenamiento correcto
    let date_utc: Option<DateTime<Utc>> = header.date.as_ref().and_then(|d| convert_panama_date_to_utc(d));
    
    // CORRECTED: Fixed table name (singular) and all field names to match PostgreSQL schema
    // Changed return type from i32 to String (returns CUFE instead of ID)
    // Now using DateTime<Utc> directly for date field (no CAST needed)
    sqlx::query(
        r#"
        INSERT INTO invoice_header (
            cufe, no, date, issuer_name, issuer_ruc, issuer_dv,
            issuer_address, issuer_phone, receptor_name, receptor_id,
            receptor_dv, receptor_address, receptor_phone, tot_amount,
            tot_itbms, auth_date, url, type, origin, process_date,
            reception_date, time, user_id, user_email, user_phone_number,
            user_telegram_id, user_ws
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 
                $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)
        "#
    )
    .bind(&header.cufe)
    .bind(&header.no)
    .bind(&date_utc) // Option<DateTime<Utc>> - Fecha de Panamá convertida a UTC
    .bind(&header.issuer_name)
    .bind(&header.issuer_ruc)
    .bind(&header.issuer_dv)
    .bind(&header.issuer_address)
    .bind(&header.issuer_phone)
    .bind(&header.receptor_name)
    .bind(&header.receptor_id)
    .bind(&header.receptor_dv)
    .bind(&header.receptor_address)
    .bind(&header.receptor_phone)
    .bind(&header.tot_amount) // Option<f64> - matches DOUBLE PRECISION
    .bind(&header.tot_itbms) // Option<f64> - matches DOUBLE PRECISION
    .bind(&header.auth_date)
    .bind(&header.url)
    .bind(&header.type_field)
    .bind(&header.origin)
    .bind(&header.process_date)
    .bind(&header.reception_date)
    .bind(&header.time)
    .bind(header.user_id) // i64 - matches BIGINT
    .bind(&header.user_email)
    .bind(&header.user_phone_number)
    .bind(&header.user_telegram_id)
    .bind(&header.user_ws)
    .execute(&mut **tx)
    .await?;
    
    Ok(header.cufe.clone())
}

// CORRECTED: Fixed table name, field names, and removed invoice_header_id (doesn't exist)
async fn save_invoice_details(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    details: &[InvoiceDetail],
) -> Result<(), sqlx::Error> {
    if details.is_empty() {
        return Ok(());
    }
    info!("Saving {} invoice details", details.len());
    for detail in details {
        sqlx::query!(
            r#"
            INSERT INTO invoice_detail (
                cufe, partkey, date, quantity, code, description,
                unit_discount, unit_price, itbms, amount, total,
                information_of_interest
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            detail.cufe,
            detail.partkey,
            detail.date,
            detail.quantity,
            detail.code,
            detail.description,
            detail.unit_discount,
            detail.unit_price,
            detail.itbms,
            detail.amount,
            detail.total,
            detail.information_of_interest,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

// CORRECTED: Fixed table name, field names, and removed invoice_header_id (doesn't exist)
async fn save_invoice_payments(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    payments: &[InvoicePayment],
) -> Result<(), sqlx::Error> {
    if payments.is_empty() {
        return Ok(());
    }
    info!("Saving {} invoice payments", payments.len());
    for payment in payments {
        sqlx::query!(
            r#"
            INSERT INTO invoice_payment (
                cufe, forma_de_pago, forma_de_pago_otro, valor_pago,
                efectivo, tarjeta_débito, tarjeta_crédito, tarjeta_clave__banistmo_,
                vuelto, total_pagado, descuentos, merged
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            payment.cufe,
            payment.forma_de_pago,
            payment.forma_de_pago_otro,
            payment.valor_pago,
            payment.efectivo,
            payment.tarjeta_debito,
            payment.tarjeta_credito,
            payment.tarjeta_clave_banistmo,
            payment.vuelto,
            payment.total_pagado,
            payment.descuentos,
            payment.merged,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

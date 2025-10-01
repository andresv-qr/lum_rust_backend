use sqlx::PgPool;
use tracing::{info, warn, error as log_error};
use chrono::NaiveDate;

use crate::api::webscraping::{InvoiceHeader, InvoiceDetail, InvoicePayment, ScrapingResult};
use crate::api::templates::url_processing_templates::ProcessUrlResponse;

// ============================================================================
// DATE UTILITIES
// ============================================================================

/// Parse date string (DD/MM/YYYY HH:MM:SS or DD/MM/YYYY) to NaiveDate
fn parse_date_string(date_opt: &Option<String>) -> Option<NaiveDate> {
    if let Some(date_str) = date_opt {
        // Try parsing with time first
        if let Ok(parsed_datetime) = chrono::NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S") {
            return Some(parsed_datetime.date());
        }
        // Try parsing date only
        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(date_str, "%d/%m/%Y") {
            return Some(parsed_date);
        }
        warn!("Could not parse date string: {}", date_str);
    }
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
            .unwrap_or_else(|| "Unknown scraping error".to_string());
        warn!(
            "Scraping failed for URL '{}', not persisting: {}",
            source_url, error_msg
        );
        return Err(ProcessUrlResponse::error(&error_msg));
    }

    let header = match scraping_result.header {
        Some(h) => h,
        None => {
            return Err(ProcessUrlResponse::error("Scraping result missing header"));
        }
    };

    let mut tx = match db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            log_error!("Failed to begin transaction: {}", e);
            return Err(ProcessUrlResponse::error("Database transaction error"));
        }
    };

    // Check if invoice already exists
    match sqlx::query!("SELECT id, cufe FROM invoice_headers WHERE cufe = $1", &header.cufe)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(_record)) => {
            return Err(ProcessUrlResponse::error("Duplicate invoice detected"));
        }
        Ok(None) => (),
        Err(e) => {
            log_error!("Failed to check for duplicate CUFE: {}", e);
            return Err(ProcessUrlResponse::error("Database error"));
        }
    }

    let invoice_id = match save_invoice_header(&mut tx, &header).await {
        Ok(id) => id,
        Err(e) => {
            log_error!("Failed to save invoice header: {}", e);
            return Err(ProcessUrlResponse::error("Failed to save invoice header"));
        }
    };

    if let Err(e) = save_invoice_details(&mut tx, &scraping_result.details, invoice_id).await {
        log_error!("Failed to save invoice details: {}", e);
        return Err(ProcessUrlResponse::error("Failed to save invoice details"));
    }

    if let Err(e) = save_invoice_payments(&mut tx, &scraping_result.payments, invoice_id).await {
        log_error!("Failed to save invoice payments: {}", e);
        return Err(ProcessUrlResponse::error("Failed to save invoice payments"));
    }

    if let Err(e) = tx.commit().await {
        log_error!("Failed to commit transaction: {}", e);
        return Err(ProcessUrlResponse::error("Database transaction commit error"));
    }

    Ok(ProcessUrlResponse::success(
        "API",
        Some(invoice_id),
        Some(header.cufe),
        0,
    ))
}

async fn save_invoice_header(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    header: &InvoiceHeader,
) -> Result<i32, sqlx::Error> {
    info!("Saving invoice header with CUFE: {}", header.cufe);
    let rec = sqlx::query!(
        r#"
        INSERT INTO invoice_headers (
            cufe, numero_factura, fecha_emision, proveedor_nombre, proveedor_ruc,
            cliente_nombre, cliente_ruc, subtotal, impuestos, total, moneda,
            estado, user_id, source_url
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING id
        "#,
        header.cufe,
        header.no, // numero_factura
        parse_date_string(&header.date), // fecha_emision - convert string to NaiveDate
        header.issuer_name, // proveedor_nombre
        header.issuer_ruc, // proveedor_ruc
        header.receptor_name, // cliente_nombre
        header.receptor_id, // cliente_ruc
        None::<rust_decimal::Decimal>, // subtotal (calculado)
        header.tot_itbms, // impuestos
        header.tot_amount, // total
        Some("PAB".to_string()), // moneda (Balboa panameño)
        Some("ACTIVO".to_string()), // estado
        header.user_id,
        Some(header.url.clone()), // source_url
    )
    .fetch_one(&mut **tx)
    .await?;
    
    Ok(rec.id as i32)
}

async fn save_invoice_details(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    details: &[InvoiceDetail],
    invoice_header_id: i32,
) -> Result<(), sqlx::Error> {
    if details.is_empty() {
        return Ok(());
    }
    info!("Saving {} invoice details for invoice_id: {}", details.len(), invoice_header_id);
    for detail in details {
        sqlx::query!(
            r#"
            INSERT INTO invoice_details (
                invoice_header_id, cufe, item_numero, descripcion, cantidad,
                precio_unitario, subtotal, impuesto_porcentaje, impuesto_monto, total
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            invoice_header_id,
            detail.cufe,
            detail.item_numero,
            detail.descripcion,
            detail.cantidad,
            detail.precio_unitario,
            detail.subtotal,
            detail.impuesto_porcentaje,
            detail.impuesto_monto,
            detail.total,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn save_invoice_payments(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    payments: &[InvoicePayment],
    invoice_header_id: i32,
) -> Result<(), sqlx::Error> {
    if payments.is_empty() {
        return Ok(());
    }
    info!("Saving {} invoice payments for invoice_id: {}", payments.len(), invoice_header_id);
    for payment in payments {
        sqlx::query!(
            r#"
            INSERT INTO invoice_payments (
                invoice_header_id, cufe, metodo_pago, monto, referencia
            )
            VALUES ($1, $2, $3, $4, $5)
            "#,
            invoice_header_id,
            payment.cufe,
            payment.metodo_pago,
            payment.monto,
            payment.referencia,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc, NaiveDateTime};
use crate::api::invoices::models::{
    InvoiceData, FullInvoiceData,
    LogStatus, ErrorType
};
use crate::api::invoices::error_handling::InvoiceProcessingError;
use tracing::{info, warn, error};

// ============================================================================
// DATE CONVERSION UTILITIES
// ============================================================================

/// Converts DGI date string (DD/MM/YYYY HH:MM:SS) to DateTime<Utc>
fn parse_dgi_date(date_str: &str) -> Result<DateTime<Utc>, InvoiceProcessingError> {
    // Parse format: "15/05/2025 09:50:04"
    let naive_dt = NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S")
        .map_err(|e| {
            error!("Error parsing date '{}': {}", date_str, e);
            InvoiceProcessingError::ValidationError {
                errors: vec![format!("Invalid date format '{}': {}", date_str, e)],
            }
        })?;
    
    // Convert to UTC (assuming Panama timezone, but treating as UTC for now)
    Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc))
}

// ============================================================================
// DUPLICATE CHECK
// ============================================================================

pub async fn check_duplicate_invoice(
    pool: &PgPool,
    cufe: &str,
) -> Result<Option<(String, DateTime<Utc>)>, InvoiceProcessingError> {
    let query = "SELECT user_id, process_date FROM public.invoice_header WHERE cufe = $1";
    
    match sqlx::query(query)
        .bind(cufe)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(row)) => {
            let original_user: String = row.try_get("user_id")?;
            let processed_date: DateTime<Utc> = row.try_get("process_date")?;
            Ok(Some((original_user, processed_date)))
        },
        Ok(None) => Ok(None),
        Err(e) => {
            error!("Error checking duplicate invoice: {}", e);
            Err(InvoiceProcessingError::DatabaseError {
                message: format!("Error checking duplicates: {}", e),
            })
        }
    }
}

// ============================================================================
// INVOICE PERSISTENCE (ATOMIC TRANSACTION)
// ============================================================================

pub async fn save_full_invoice(
    pool: &PgPool,
    invoice_data: &FullInvoiceData,
) -> Result<(), InvoiceProcessingError> {
    info!("Starting atomic transaction for CUFE: {}", invoice_data.header.cufe);
    
    // Convert DGI date string to DateTime
    let parsed_date = parse_dgi_date(&invoice_data.header.date)?;
    
    // Parse amount strings to f64
    let tot_amount: Option<f64> = invoice_data.header.tot_amount.parse().ok();
    let tot_itbms: Option<f64> = invoice_data.header.tot_itbms.parse().ok();
    
    let mut tx = pool.begin().await?;
    
    // 1. Insert invoice header
    let header_query = r#"
        INSERT INTO public.invoice_header (
            no, date, cufe, issuer_name, issuer_ruc, issuer_dv, 
            issuer_address, issuer_phone, tot_amount, tot_itbms,
            url, type, process_date, reception_date, user_id, origin, user_email
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
    "#;
    
    sqlx::query(header_query)
        .bind(&invoice_data.header.no)
        .bind(parsed_date)  // Use parsed DateTime instead of string
        .bind(&invoice_data.header.cufe)
        .bind(&invoice_data.header.issuer_name)
        .bind(&invoice_data.header.issuer_ruc)
        .bind(&invoice_data.header.issuer_dv)
        .bind(&invoice_data.header.issuer_address)
        .bind(&invoice_data.header.issuer_phone)
        .bind(&tot_amount)
        .bind(&tot_itbms)
        .bind(&invoice_data.header.url)
        .bind(&invoice_data.header.r#type)
        .bind(&invoice_data.header.process_date)
        .bind(&invoice_data.header.reception_date)
        .bind(&invoice_data.header.user_id)
        .bind(&invoice_data.header.origin)
        .bind(&invoice_data.header.user_email)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Error inserting invoice header: {}", e);
            InvoiceProcessingError::DatabaseError {
                message: format!("Failed to insert header: {}", e),
            }
        })?;
    
    info!("Invoice header inserted successfully");
    
    // 2. Insert invoice details (multiple items)
    if !invoice_data.details.is_empty() {
        let detail_query = r#"
            INSERT INTO public.invoice_detail (
                cufe, quantity, code, description, unit_discount, 
                unit_price, itbms, information_of_interest
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;
        
        for detail in &invoice_data.details {
            sqlx::query(detail_query)
                .bind(&detail.cufe)
                .bind(&detail.quantity)
                .bind(&detail.code)
                .bind(&detail.description)
                .bind(&detail.unit_discount)
                .bind(&detail.unit_price)
                .bind(&detail.itbms)
                .bind(&detail.information_of_interest)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error!("Error inserting invoice detail: {}", e);
                    InvoiceProcessingError::DatabaseError {
                        message: format!("Failed to insert detail: {}", e),
                    }
                })?;
        }
        
        info!("Inserted {} invoice detail items", invoice_data.details.len());
    }
    
    // 3. Insert invoice payment
    let payment_query = r#"
        INSERT INTO public.invoice_payment (cufe, vuelto, total_pagado) 
        VALUES ($1, $2, $3)
    "#;
    
    sqlx::query(payment_query)
        .bind(&invoice_data.payment.cufe)
        .bind(&invoice_data.payment.vuelto)
        .bind(&invoice_data.payment.total_pagado)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Error inserting invoice payment: {}", e);
            InvoiceProcessingError::DatabaseError {
                message: format!("Failed to insert payment: {}", e),
            }
        })?;
    
    info!("Invoice payment inserted successfully");
    
    // 4. Commit transaction
    tx.commit().await.map_err(|e| {
        error!("Error committing transaction: {}", e);
        InvoiceProcessingError::DatabaseError {
            message: format!("Failed to commit transaction: {}", e),
        }
    })?;
    
    info!("Full invoice transaction committed successfully for CUFE: {}", invoice_data.header.cufe);
    Ok(())
}

// ============================================================================
// LOGGING REPOSITORY (logs.bot_rust_scrapy)
// ============================================================================

pub async fn create_initial_log(
    pool: &PgPool,
    url: &str,
    origin: &str,
    user_id: &str,
    user_email: &str,
) -> Result<i32, InvoiceProcessingError> {
    let query = r#"
        INSERT INTO logs.bot_rust_scrapy (
            url, origin, user_id, user_email, status, 
            request_timestamp, retry_attempts, scraped_fields_count
        ) VALUES ($1, $2, $3, $4, 'PROCESSING', $5, 0, 0)
        RETURNING id
    "#;
    
    let log_id = sqlx::query(query)
        .bind(url)
        .bind(origin)
        .bind(user_id)
        .bind(user_email)
        .bind(Utc::now())
        .fetch_one(pool)
        .await?
        .try_get::<i32, _>("id")?;
    
    info!("Created initial log entry with ID: {}", log_id);
    Ok(log_id)
}

pub async fn update_log_success(
    pool: &PgPool,
    log_id: i32,
    cufe: &str,
    execution_time_ms: i32,
    fields_count: i32,
    retry_attempts: i32,
) -> Result<(), InvoiceProcessingError> {
    let query = r#"
        UPDATE logs.bot_rust_scrapy 
        SET cufe = $1, 
            execution_time_ms = $2, 
            status = $3, 
            response_timestamp = $4,
            scraped_fields_count = $5,
            retry_attempts = $6
        WHERE id = $7
    "#;
    
    sqlx::query(query)
        .bind(cufe)
        .bind(execution_time_ms)
        .bind(LogStatus::Success.as_str())
        .bind(Utc::now())
        .bind(fields_count)
        .bind(retry_attempts)
        .bind(log_id)
        .execute(pool)
        .await?;
    
    info!("Updated log {} with success status", log_id);
    Ok(())
}

pub async fn update_log_duplicate(
    pool: &PgPool,
    log_id: i32,
    cufe: &str,
    execution_time_ms: i32,
) -> Result<(), InvoiceProcessingError> {
    let query = r#"
        UPDATE logs.bot_rust_scrapy 
        SET cufe = $1, 
            execution_time_ms = $2, 
            status = $3, 
            response_timestamp = $4,
            error_message = 'Factura ya existe en base de datos'
        WHERE id = $5
    "#;
    
    sqlx::query(query)
        .bind(cufe)
        .bind(execution_time_ms)
        .bind(LogStatus::Duplicate.as_str())
        .bind(Utc::now())
        .bind(log_id)
        .execute(pool)
        .await?;
    
    info!("Updated log {} with duplicate status", log_id);
    Ok(())
}

pub async fn update_log_error(
    pool: &PgPool,
    log_id: i32,
    status: LogStatus,
    error_type: ErrorType,
    error_message: &str,
    execution_time_ms: Option<i32>,
    retry_attempts: i32,
) -> Result<(), InvoiceProcessingError> {
    let query = r#"
        UPDATE logs.bot_rust_scrapy 
        SET status = $1, 
            error_type = $2, 
            error_message = $3, 
            response_timestamp = $4,
            execution_time_ms = $5,
            retry_attempts = $6
        WHERE id = $7
    "#;
    
    sqlx::query(query)
        .bind(status.as_str())
        .bind(error_type.as_str())
        .bind(error_message)
        .bind(Utc::now())
        .bind(execution_time_ms)
        .bind(retry_attempts)
        .bind(log_id)
        .execute(pool)
        .await?;
    
    warn!("Updated log {} with error status: {}", log_id, status.as_str());
    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

pub async fn get_invoice_by_cufe(
    pool: &PgPool,
    cufe: &str,
) -> Result<Option<InvoiceData>, InvoiceProcessingError> {
    let query = r#"
        SELECT no, date, cufe, issuer_name, issuer_ruc, issuer_dv,
               issuer_address, issuer_phone, tot_amount, tot_itbms,
               url, type, process_date, reception_date, user_id, origin, user_email
        FROM public.invoice_header 
        WHERE cufe = $1
    "#;
    
    match sqlx::query(query)
        .bind(cufe)
        .fetch_optional(pool)
        .await?
    {
        Some(row) => {
            let invoice = InvoiceData {
                no: row.try_get("no")?,
                date: row.try_get("date")?,
                cufe: row.try_get("cufe")?,
                issuer_name: row.try_get("issuer_name")?,
                issuer_ruc: row.try_get("issuer_ruc")?,
                issuer_dv: row.try_get("issuer_dv")?,
                issuer_address: row.try_get("issuer_address")?,
                issuer_phone: row.try_get("issuer_phone")?,
                tot_amount: row.try_get("tot_amount")?,
                tot_itbms: row.try_get("tot_itbms")?,
                url: row.try_get("url")?,
                r#type: row.try_get("type")?,
                process_date: row.try_get("process_date")?,
                reception_date: row.try_get("reception_date")?,
                user_id: row.try_get("user_id")?,
                origin: row.try_get("origin")?,
                user_email: row.try_get("user_email")?,
            };
            Ok(Some(invoice))
        },
        None => Ok(None),
    }
}

pub async fn count_invoice_details(
    pool: &PgPool,
    cufe: &str,
) -> Result<i64, InvoiceProcessingError> {
    let query = "SELECT COUNT(*) as count FROM public.invoice_detail WHERE cufe = $1";
    
    let count = sqlx::query(query)
        .bind(cufe)
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("count")?;
    
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: These tests would require a test database setup
    // For now, they serve as documentation of expected behavior
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_check_duplicate_invoice() {
        // This test would verify duplicate checking functionality
        // Implementation depends on test database configuration
    }
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_save_full_invoice() {
        // This test would verify atomic transaction behavior
        // Implementation depends on test database configuration
    }
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_logging_operations() {
        // This test would verify logging functionality
        // Implementation depends on test database configuration
    }
}

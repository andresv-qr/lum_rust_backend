use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc, NaiveDateTime};
use crate::api::invoice_processor::models::{
    FullInvoiceData,
    LogStatus, ErrorType
};
use crate::api::invoice_processor::error_handling::InvoiceProcessingError;
use crate::models::invoice::InvoiceHeader; // Import the canonical InvoiceHeader
use tracing::{info, warn, error};

// ============================================================================
// DATE CONVERSION UTILITIES
// ============================================================================

/// Converts DGI date string (DD/MM/YYYY HH:MM:SS) to DateTime<Utc>
pub fn parse_dgi_date(date_str: &str) -> Result<DateTime<Utc>, InvoiceProcessingError> {
    // Parse format: "15/05/2025 09:50:04"
    let naive_dt = NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S")
        .map_err(|e| {
            error!("Error parsing date '{}': {}", date_str, e);
            InvoiceProcessingError::ValidationError {
                message: format!("Invalid date format '{}': {}", date_str, e),
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

pub async fn save_invoice_data(
    pool: &PgPool,
    invoice_data: &FullInvoiceData,
) -> Result<(), InvoiceProcessingError> {
    let mut tx = pool.begin().await?;

    let parsed_date = invoice_data.header.date;

    // Enhanced logging for database insertion debugging
    info!("🗃️ About to insert invoice header with values:");
    info!("   📄 no: '{:?}'", invoice_data.header.no);
    info!("   📅 date: '{:?}'", parsed_date);
    info!("   🔑 cufe: '{}'", invoice_data.header.cufe);
    info!("   🏢 issuer_name: '{}'", invoice_data.header.issuer_name);
    info!("   📞 issuer_ruc: '{:?}'", invoice_data.header.issuer_ruc);
    info!("   🔢 issuer_dv: '{:?}'", invoice_data.header.issuer_dv);
    info!("   🏠 issuer_address: '{:?}'", invoice_data.header.issuer_address);
    info!("   📱 issuer_phone: '{:?}'", invoice_data.header.issuer_phone);
    info!("   💰 tot_amount: {:?} (type: f64)", invoice_data.header.tot_amount);
    info!("   🧾 tot_itbms: {:?} (type: f64)", invoice_data.header.tot_itbms);
    info!("   🌐 url: '{}'", invoice_data.header.url);
    info!("   📂 type: '{}'", invoice_data.header.r#type);
    info!("   ⏰ process_date: '{:?}'", invoice_data.header.process_date);
    info!("   📥 reception_date: '{:?}'", invoice_data.header.reception_date);
    info!("   👤 user_id: {} (type: i64)", invoice_data.header.user_id);
    info!("   📍 origin: '{}'", invoice_data.header.origin);
    info!("   📧 user_email: '{}'", invoice_data.header.user_email);
    
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
        .bind(parsed_date)
        .bind(&invoice_data.header.cufe)
        .bind(&invoice_data.header.issuer_name)
        .bind(&invoice_data.header.issuer_ruc)
        .bind(&invoice_data.header.issuer_dv)
        .bind(&invoice_data.header.issuer_address)
        .bind(&invoice_data.header.issuer_phone)
        .bind(&invoice_data.header.tot_amount)
        .bind(&invoice_data.header.tot_itbms)
        .bind(&invoice_data.header.url)
        .bind(&invoice_data.header.r#type)
        .bind(invoice_data.header.process_date)
        .bind(invoice_data.header.reception_date)
        .bind(&invoice_data.header.user_id)
        .bind(&invoice_data.header.origin)
        .bind(&invoice_data.header.user_email)
        .execute(tx.as_mut())
        .await
        .map_err(|e| {
            error!("❌ Error inserting invoice header: {}", e);
            InvoiceProcessingError::DatabaseError {
                message: format!("Failed to insert header: {}", e),
            }
        })?;
    
    info!("✅ Invoice header inserted successfully");
    
    // 2. Insert invoice details (multiple items)
    if !invoice_data.details.is_empty() {
        info!("🗃️ About to insert {} invoice detail items:", invoice_data.details.len());
        
        for (index, item) in invoice_data.details.iter().enumerate() {
            info!("   Item {}:", index + 1);
            info!("     partkey: '{}'", item.partkey);
            info!("     cufe: '{}'", item.cufe);
            info!("     date: '{:?}'", item.date);
            info!("     quantity: '{:?}'", item.quantity);
            info!("     code: '{:?}'", item.code);
            info!("     description: '{:?}'", item.description);
            info!("     unit_price: '{:?}'", item.unit_price);
            info!("     total: '{:?}'", item.total);
            info!("     amount: '{:?}'", item.amount);
            info!("     information_of_interest: '{:?}'", item.information_of_interest);
            info!("     linea: '{:?}'", item.linea);
        }

        let detail_query = r#"
            INSERT INTO public.invoice_detail (
                partkey, cufe, date, quantity, code, description, 
                unit_price, total, amount, information_of_interest, 
                linea, unit_discount, itbms
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#;
        
        for item in &invoice_data.details {
            sqlx::query(detail_query)
                .bind(&item.partkey)
                .bind(&item.cufe)
                .bind(item.date)
                .bind(&item.quantity)
                .bind(&item.code)
                .bind(&item.description)
                .bind(&item.unit_price)
                .bind(&item.total)
                .bind(&item.amount)
                .bind(&item.information_of_interest)
                .bind(&item.linea)
                .bind(&item.unit_discount)
                .bind(&item.itbms)
                .bind(item.date)
                .bind(&item.quantity)
                .bind(&item.code)
                .bind(&item.description)
                .bind(&item.unit_price)
                .bind(&item.total)
                .bind(&item.amount)
                .bind(&item.information_of_interest)
                .bind(&item.linea)
                .execute(tx.as_mut())
                .await
                .map_err(|e| {
                    error!("❌ Error inserting invoice detail item: {}", e);
                    InvoiceProcessingError::DatabaseError {
                        message: format!("Failed to insert detail item: {}", e),
                    }
                })?;
        }
        info!("✅ All invoice detail items inserted successfully");
    }
    
    // 3. Insert invoice payment
    if !invoice_data.payment.cufe.is_empty() {
        info!("🗃️ About to insert invoice payment:");
        info!("   cufe: '{}'", invoice_data.payment.cufe);
        info!("   vuelto: '{:?}'", invoice_data.payment.vuelto);
        info!("   total_pagado: '{:?}'", invoice_data.payment.total_pagado);

        let payment_query = r#"
            INSERT INTO public.invoice_payment (cufe, vuelto, total_pagado)
            VALUES ($1, $2, $3)
        "#;
        
        sqlx::query(payment_query)
            .bind(&invoice_data.payment.cufe)
            .bind(&invoice_data.payment.vuelto)
            .bind(&invoice_data.payment.total_pagado)
            .execute(tx.as_mut())
            .await
            .map_err(|e| {
                error!("❌ Error inserting invoice payment: {}", e);
                InvoiceProcessingError::DatabaseError {
                    message: format!("Failed to insert payment: {}", e),
                }
            })?;
        
        info!("✅ Invoice payment inserted successfully");
    }
    
    tx.commit().await?;
    
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
) -> Result<Option<InvoiceHeader>, InvoiceProcessingError> {
    let query = r#"
        SELECT no, date, cufe, issuer_name, issuer_ruc, issuer_dv,
               issuer_address, issuer_phone, tot_amount, tot_itbms,
               url, type, process_date, reception_date, user_id, origin, user_email
        FROM public.invoice_header 
        WHERE cufe = $1
    "#;
    
    match sqlx::query_as::<_, InvoiceHeader>(query)
        .bind(cufe)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(invoice)) => Ok(Some(invoice)),
        Ok(None) => Ok(None),
        Err(e) => {
            error!("Error fetching invoice by CUFE: {}", e);
            Err(InvoiceProcessingError::DatabaseError {
                message: format!("Failed to fetch invoice: {}", e),
            })
        }
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

pub fn validate_and_parse_date(date_str: &str) -> Result<NaiveDateTime, InvoiceProcessingError> {
    NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S")
        .map_err(|e| InvoiceProcessingError::ValidationError{ 
            message: format!("Invalid date format '{}': {}", date_str, e) 
        })
}

/// Checks if an invoice with the given CUFE already exists in the database.
/// Returns `Ok(true)` if exists, `Ok(false)` if not, and an error if there was a problem checking.
pub async fn invoice_exists(
    pool: &PgPool,
    cufe: &str,
) -> Result<bool, InvoiceProcessingError> {
    let query = "SELECT EXISTS(SELECT 1 FROM public.invoice_header WHERE cufe = $1)";
    
    let exists = sqlx::query_scalar(query)
        .bind(cufe)
        .fetch_one(pool)
        .await?;
    
    Ok(exists)
}

// ============================================================================
// MEF_PENDING FALLBACK
// ============================================================================

/// Saves invoice data to mef_pending table when automatic processing fails
/// This allows manual review and processing later
pub async fn save_to_mef_pending(
    pool: &PgPool,
    url: &str,
    user_id: &str,
    user_email: &str,
    origin: &str,
    error_message: &str,
    cufe: Option<&str>,
) -> Result<(), InvoiceProcessingError> {
    info!("💾 Guardando factura en mef_pending para procesamiento manual");
    info!("   URL: {}", url);
    info!("   User ID: {}", user_id);
    info!("   Error: {}", error_message);
    if let Some(c) = cufe {
        info!("   CUFE: {}", c);
    }

    let query = r#"
        INSERT INTO public.mef_pending (
            url, 
            date, 
            type, 
            user_email, 
            user_id, 
            error, 
            origin
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (url) DO UPDATE SET
            date = EXCLUDED.date,
            error = EXCLUDED.error,
            user_id = EXCLUDED.user_id
    "#;

    // Parse user_id to i64
    let user_id_i64: i64 = user_id.parse().unwrap_or_else(|e| {
        warn!("Failed to parse user_id '{}' as i64: {}. Using 0.", user_id, e);
        0
    });

    sqlx::query(query)
        .bind(url)
        .bind(Utc::now())
        .bind("API_INVOICE")
        .bind(user_email)
        .bind(user_id_i64)
        .bind(error_message)
        .bind(origin)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Failed to insert into mef_pending: {}", e);
            InvoiceProcessingError::DatabaseError {
                message: format!("Error saving to mef_pending: {}", e),
            }
        })?;

    info!("✅ Factura guardada en mef_pending exitosamente");
    Ok(())
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

use crate::models::invoice::{InvoiceHeader, InvoiceDetail, InvoicePayment, MefPending};
use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{info, debug};

pub async fn save_invoice_data(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    header: &InvoiceHeader,
    details: &[InvoiceDetail],
    _payments: &[InvoicePayment], // Payments not handled yet
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO public.invoice_header (cufe, no, date, issuer_name, issuer_ruc, issuer_dv, issuer_address, issuer_phone, tot_amount, tot_itbms, url, type, process_date, reception_date, user_id, origin, user_email)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        ON CONFLICT (cufe) DO NOTHING
        "#
    )
    .bind(&header.cufe)
    .bind(&header.no)
    .bind(header.date)
    .bind(&header.issuer_name)
    .bind(&header.issuer_ruc)
    .bind(&header.issuer_dv)
    .bind(&header.issuer_address)
    .bind(&header.issuer_phone)
    .bind(&header.tot_amount)
    .bind(&header.tot_itbms)
    .bind(&header.url)
    .bind(&header.r#type)
    .bind(&header.process_date)
    .bind(&header.reception_date)
    .bind(&header.user_id)
    .bind(&header.origin)
    .bind(&header.user_email)
    .execute(&mut **tx)
    .await
    .context("Failed to insert invoice header")?;

    for detail in details {
        sqlx::query(
            r#"
            INSERT INTO invoice_detail (partkey, cufe, date, quantity, code, description, unit_price, total, amount, information_of_interest, linea, unit_discount, itbms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(&detail.partkey)
        .bind(&detail.cufe)
        .bind(detail.date)
        .bind(&detail.quantity)
        .bind(&detail.code)
        .bind(&detail.description)
        .bind(&detail.unit_price)
        .bind(&detail.total)
        .bind(&detail.amount)
        .bind(&detail.information_of_interest)
        .bind(&detail.linea)
        .bind(&detail.unit_discount)
        .bind(&detail.itbms)
        .execute(tx.as_mut())
        .await?;
    }

    // Insert payment
    /*
    for payment in _payments {
        sqlx::query(
            r#"
            INSERT INTO public.invoice_payment (cufe, payment_method, payment_date, amount, reference, user_id, user_email)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(&payment.cufe)
        .bind(&payment.payment_method)
        .bind(payment.payment_date)
        .bind(&payment.amount)
        .bind(&payment.reference)
        .bind(&payment.user_id)
        .bind(&payment.user_email)
        .execute(&mut **tx)
        .await
        .context("Failed to insert invoice payment")?;
    }
    */

    Ok(())
}

pub async fn save_to_mef_pending(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entry: &MefPending,
) -> Result<()> {
    // CORRECTED: Column names to match actual PostgreSQL table
    // Table has: type, error, date (not type_document, error_message, reception_date)
    // Using ON CONFLICT DO UPDATE to handle duplicate URLs (url is UNIQUE)
    sqlx::query(
        r#"
        INSERT INTO public.mef_pending (url, chat_id, date, message_id, type, user_email, user_id, error, origin, ws_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (url) DO UPDATE SET
            date = EXCLUDED.date,
            error = EXCLUDED.error,
            user_id = EXCLUDED.user_id
        "#
    )
    .bind(&entry.url)
    .bind(&entry.chat_id)
    .bind(entry.reception_date) // Maps to 'date' column
    .bind(&entry.message_id)
    .bind(&entry.type_document) // Maps to 'type' column
    .bind(&entry.user_email)
    .bind(entry.user_id)
    .bind(&entry.error_message) // Maps to 'error' column
    .bind(&entry.origin)
    .bind(&entry.ws_id)
    .execute(&mut **tx)
    .await
    .context("Failed to insert into mef_pending")?;

    Ok(())
}

/// Validates if a CUFE already exists in the database
/// Returns true if CUFE exists, false if it's new
pub async fn validate_cufe_exists(pool: &PgPool, cufe: &str) -> Result<bool> {
    debug!("🔍 Validando si CUFE ya existe: {}", cufe);
    
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM public.invoice_header WHERE cufe = $1",
        cufe
    )
    .fetch_one(pool)
    .await
    .context("Failed to check CUFE existence")?;

    let exists = result.count.unwrap_or(0) > 0;
    
    if exists {
        info!("✅ CUFE ya existe en la base de datos: {}", cufe);
    } else {
        info!("🆕 CUFE es nuevo, proceder con guardado: {}", cufe);
    }
    
    Ok(exists)
}

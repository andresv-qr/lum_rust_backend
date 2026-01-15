//! Macros para generar structs de query con campos comunes
//! 
//! Evita duplicación de código entre headers y details
//! manteniendo la misma estructura con diferentes campos adicionales

/// Macro para generar struct de header con campo adicional (count)
/// 
/// Uso:
/// ```rust
/// header_with_count_struct!(HeaderWithCount);
/// header_with_count_struct!(RecoveryQueryResult);
/// ```
#[macro_export]
macro_rules! header_with_count_struct {
    ($name:ident) => {
        #[derive(sqlx::FromRow)]
        struct $name {
            cufe: String,
            issuer_name: Option<String>,
            issuer_ruc: Option<String>,
            store_id: Option<String>,
            no: Option<String>,
            date: Option<chrono::DateTime<chrono::Utc>>,
            tot_amount: Option<f64>,
            tot_itbms: Option<f64>,
            url: Option<String>,
            process_date: Option<chrono::DateTime<chrono::Utc>>,
            reception_date: Option<chrono::DateTime<chrono::Utc>>,
            #[sqlx(rename = "type")]
            invoice_type: Option<String>,
            update_date: chrono::DateTime<chrono::Utc>,
            total_count: i64,
        }
    };
}

/// Macro para generar struct de header para recovery con total_missing
#[macro_export]
macro_rules! header_recovery_struct {
    ($name:ident) => {
        #[derive(sqlx::FromRow)]
        struct $name {
            cufe: String,
            issuer_name: Option<String>,
            issuer_ruc: Option<String>,
            store_id: Option<String>,
            no: Option<String>,
            date: Option<chrono::DateTime<chrono::Utc>>,
            tot_amount: Option<f64>,
            tot_itbms: Option<f64>,
            url: Option<String>,
            process_date: Option<chrono::DateTime<chrono::Utc>>,
            reception_date: Option<chrono::DateTime<chrono::Utc>>,
            #[sqlx(rename = "type")]
            invoice_type: Option<String>,
            update_date: chrono::DateTime<chrono::Utc>,
            total_missing: i64,
        }
    };
}

/// Macro para generar struct de detail con campo adicional (count)
#[macro_export]
macro_rules! detail_with_count_struct {
    ($name:ident) => {
        #[derive(sqlx::FromRow)]
        struct $name {
            cufe: String,
            code: Option<String>,
            description: Option<String>,
            quantity: Option<String>,
            unit_price: Option<String>,
            amount: Option<String>,
            itbms: Option<String>,
            total: Option<String>,
            unit_discount: Option<String>,
            information_of_interest: Option<String>,
            update_date: chrono::DateTime<chrono::Utc>,
            total_count: i64,
        }
    };
}

/// Macro para generar struct de detail para recovery
#[macro_export]
macro_rules! detail_recovery_struct {
    ($name:ident) => {
        #[derive(sqlx::FromRow)]
        struct $name {
            cufe: String,
            code: Option<String>,
            description: Option<String>,
            quantity: Option<String>,
            unit_price: Option<String>,
            amount: Option<String>,
            itbms: Option<String>,
            total: Option<String>,
            unit_discount: Option<String>,
            information_of_interest: Option<String>,
            update_date: chrono::DateTime<chrono::Utc>,
            total_missing: i64,
        }
    };
}

/// Helper para convertir header interno a response
pub fn header_to_response(
    cufe: String,
    issuer_name: Option<String>,
    issuer_ruc: Option<String>,
    store_id: Option<String>,
    no: Option<String>,
    date: Option<chrono::DateTime<chrono::Utc>>,
    tot_amount: Option<f64>,
    tot_itbms: Option<f64>,
    url: Option<String>,
    process_date: Option<chrono::DateTime<chrono::Utc>>,
    reception_date: Option<chrono::DateTime<chrono::Utc>>,
    invoice_type: Option<String>,
    update_date: chrono::DateTime<chrono::Utc>,
) -> crate::api::user_invoice_headers_v4::UserInvoiceHeadersResponse {
    crate::api::user_invoice_headers_v4::UserInvoiceHeadersResponse {
        cufe,
        issuer_name,
        issuer_ruc,
        store_id,
        no,
        date,
        tot_amount,
        tot_itbms,
        url,
        process_date,
        reception_date,
        invoice_type,
        update_date,
    }
}

/// Helper para convertir detail interno a response
pub fn detail_to_response(
    cufe: String,
    code: Option<String>,
    description: Option<String>,
    quantity: Option<String>,
    unit_price: Option<String>,
    amount: Option<String>,
    itbms: Option<String>,
    total: Option<String>,
    unit_discount: Option<String>,
    information_of_interest: Option<String>,
    update_date: chrono::DateTime<chrono::Utc>,
) -> crate::api::user_invoice_details_v4::UserInvoiceDetailsResponse {
    crate::api::user_invoice_details_v4::UserInvoiceDetailsResponse {
        cufe,
        code,
        description,
        quantity,
        unit_price,
        amount,
        itbms,
        total,
        unit_discount,
        information_of_interest,
        update_date,
    }
}

#[cfg(test)]
mod tests {
    // Test that macros compile correctly
    header_with_count_struct!(TestHeaderWithCount);
    detail_with_count_struct!(TestDetailWithCount);
    
    #[test]
    fn test_macro_generates_valid_struct() {
        // This test just verifies the macros compile
        // The structs are validated by sqlx at compile time
        assert!(true);
    }
}

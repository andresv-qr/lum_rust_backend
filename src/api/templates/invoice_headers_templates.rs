use serde::{Deserialize, Serialize};

/// Query templates for invoice_headers domain
pub struct InvoiceHeadersQueryTemplates;

impl InvoiceHeadersQueryTemplates {
    /// get_invoice_headers - list operation with caching
    pub fn get_invoice_headers_query() -> &'static str {
        "SELECT date, tot_itbms, issuer_name, issuer_ruc, no, tot_amount, url, process_date, reception_date, type, cufe FROM public.invoice_header WHERE user_id = $1 ORDER BY date DESC LIMIT 100 OFFSET $2"
    }
    
    pub fn get_invoice_headers_cache_key_prefix() -> &'static str {
        "invoice_headers_get_invoice_headers_list"
    }
    
    pub fn get_invoice_headers_cache_ttl() -> u64 {
        300
    }
}

/// Response model for get_invoice_headers
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct InvoiceHeadersResponse {
    pub date: Option<chrono::DateTime<chrono::Utc>>,
    pub tot_itbms: Option<String>,
    pub issuer_name: Option<String>,
    pub issuer_ruc: Option<String>,
    pub no: Option<String>,
    pub tot_amount: Option<String>,
    pub url: Option<String>,
    pub process_date: Option<chrono::DateTime<chrono::Utc>>,
    pub reception_date: Option<chrono::DateTime<chrono::Utc>>,
    pub r#type: Option<String>,
    pub cufe: Option<String>,
}

/// Request model for get_invoice_headers (for POST endpoints)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceHeadersRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filters: Option<serde_json::Value>,
}

/// Cache invalidation patterns for invoice_headers
pub struct InvoiceHeadersCachePatterns;

impl InvoiceHeadersCachePatterns {
    pub fn invalidate_patterns(id: i64) -> Vec<String> {
        vec![
            format!("invoice_headers_*_{}", id),
            format!("invoice_headers_list_*"),
            format!("invoice_headers_stats_*"),
        ]
    }
}

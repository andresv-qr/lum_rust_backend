use serde::{Deserialize, Serialize};

/// Query templates for invoices domain
pub struct InvoicesQueryTemplates;

impl InvoicesQueryTemplates {
    /// get_invoice_details - single operation with caching
    pub fn get_invoice_details_query() -> &'static str {
        "SELECT cufe, quantity, code, description, unit_discount, unit_price, itbms, information_of_interest FROM public.invoice_detail WHERE cufe = $1 LIMIT 1"
    }
    
    pub fn get_invoice_details_cache_key_prefix() -> &'static str {
        "invoices_get_invoice_details_$1"
    }
    
    pub fn get_invoice_details_cache_ttl() -> u64 {
        600
    }
}

/// Response model for get_invoice_details
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct InvoicesResponse {
    pub cufe: Option<String>,
    pub quantity: Option<String>,
    pub code: Option<String>,
    pub date: Option<String>,
    pub total: Option<String>,
    pub unit_price: Option<String>,
    pub amount: Option<String>,
    pub unit_discount: Option<String>,
    pub description: Option<String>,
}

/// Request model for get_invoice_details (for POST endpoints)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoicesRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filters: Option<serde_json::Value>,
}

/// Cache invalidation patterns for invoices
pub struct InvoicesCachePatterns;

impl InvoicesCachePatterns {
    pub fn invalidate_patterns(id: i64) -> Vec<String> {
        vec![
            format!("invoices_*_{}", id),
            format!("invoices_list_*"),
            format!("invoices_stats_*"),
        ]
    }
}

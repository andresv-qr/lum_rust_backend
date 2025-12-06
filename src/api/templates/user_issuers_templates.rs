use serde::{Deserialize, Serialize};
use crate::api::common::{HasUpdateDate, HasId};

/// Query templates for user_issuers domain
pub struct UserIssuersQueryTemplates;

impl UserIssuersQueryTemplates {
    /// get_user_issuers - Get issuer stores that a user has invoices with
    pub fn get_user_issuers_query() -> &'static str {
        r#"
        SELECT DISTINCT 
            a.issuer_ruc,
            a.store_id,
            a.store_name,
            a.brand_name,
            a.l1,
            a.l2,
            a.l3,
            a.l4,
            a.update_date
        FROM public.dim_issuer_stores a 
        WHERE EXISTS (
            SELECT 1 
            FROM public.invoice_header ih 
            WHERE ih.user_id = $1 
            AND a.issuer_ruc = ih.issuer_ruc 
            AND a.store_id = ih.store_id
        )
        ORDER BY a.store_name ASC
        LIMIT $2 OFFSET $3
        "#
    }
    
    /// get_user_issuers_with_date_filter - Get issuer stores with update_date filter
    pub fn get_user_issuers_with_date_filter_query() -> &'static str {
        r#"
        SELECT DISTINCT 
            a.issuer_ruc,
            a.store_id,
            a.store_name,
            a.brand_name,
            a.l1,
            a.l2,
            a.l3,
            a.l4,
            a.update_date
        FROM public.dim_issuer_stores a 
        WHERE EXISTS (
            SELECT 1 
            FROM public.invoice_header ih 
            WHERE ih.user_id = $1 
            AND a.issuer_ruc = ih.issuer_ruc 
            AND a.store_id = ih.store_id
        )
        AND a.update_date >= $4
        ORDER BY a.store_name ASC
        LIMIT $2 OFFSET $3
        "#
    }
    
    /// Count query for pagination
    pub fn get_user_issuers_count_query() -> &'static str {
        r#"
        SELECT COUNT(DISTINCT (a.issuer_ruc, a.store_id)) as total
        FROM public.dim_issuer_stores a 
        WHERE EXISTS (
            SELECT 1 
            FROM public.invoice_header ih 
            WHERE ih.user_id = $1 
            AND a.issuer_ruc = ih.issuer_ruc 
            AND a.store_id = ih.store_id
        )
        "#
    }
    
    /// Count query for pagination with date filter
    pub fn get_user_issuers_count_with_date_filter_query() -> &'static str {
        r#"
        SELECT COUNT(DISTINCT (a.issuer_ruc, a.store_id)) as total
        FROM public.dim_issuer_stores a 
        WHERE EXISTS (
            SELECT 1 
            FROM public.invoice_header ih 
            WHERE ih.user_id = $1 
            AND a.issuer_ruc = ih.issuer_ruc 
            AND a.store_id = ih.store_id
        )
        AND a.update_date >= $2
        "#
    }
    
    pub fn get_user_issuers_cache_key_prefix() -> &'static str {
        "user_issuers_list"
    }
    
    pub fn get_user_issuers_cache_ttl() -> u64 {
        600 // 10 minutes cache
    }
}

/// Response model for get_user_issuers
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserIssuersResponse {
    pub issuer_ruc: Option<String>,
    pub store_id: Option<String>,
    pub store_name: Option<String>,
    pub brand_name: Option<String>,
    pub l1: Option<String>,
    pub l2: Option<String>,
    pub l3: Option<String>,
    pub l4: Option<String>,
    pub update_date: Option<chrono::NaiveDateTime>,
}

// Implement traits for incremental sync helpers
impl HasUpdateDate for UserIssuersResponse {
    fn get_update_date(&self) -> Option<chrono::NaiveDateTime> {
        self.update_date
    }
}

impl HasId for UserIssuersResponse {
    fn get_id(&self) -> Option<String> {
        // Composite ID: issuer_ruc + store_id
        match (&self.issuer_ruc, &self.store_id) {
            (Some(ruc), Some(store)) => Some(format!("{}-{}", ruc, store)),
            _ => None,
        }
    }
}

/// Count result for pagination
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserIssuersCount {
    pub total: i64,
}

/// Request model for get_user_issuers (for pagination and filters)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserIssuersRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>, // Future: for filtering by issuer name
    pub update_date_from: Option<String>, // Filter by update_date >= this date (ISO 8601 format)
}

/// Complete response with pagination
#[derive(Debug, Serialize, Deserialize)]
pub struct UserIssuersPagedResponse {
    pub issuers: Vec<UserIssuersResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_next: bool,
    pub has_previous: bool,
    pub total_pages: i64,
    pub current_page: i64,
}

/// Cache invalidation patterns for user_issuers
pub struct UserIssuersCachePatterns;

impl UserIssuersCachePatterns {
    pub fn invalidate_patterns(user_id: i32) -> Vec<String> {
        vec![
            format!("user_issuers_list_{}*", user_id),
            format!("user_issuers_count_{}*", user_id),
        ]
    }
}

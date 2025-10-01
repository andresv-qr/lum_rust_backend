use serde::{Deserialize, Serialize};

/// Query templates for user_products domain
pub struct UserProductsQueryTemplates;

impl UserProductsQueryTemplates {
    /// get_user_products - Get products that a user has purchased
    pub fn get_user_products_query() -> &'static str {
        r#"
        SELECT 
            p.code,
            p.issuer_name,
            p.description,
            p.l1_gemini as l1,
            p.l2_gemini as l2,
            p.l3_gemini as l3,
            p.l4_gemini as l4,
            p.process_date
        FROM public.dim_product p
        JOIN (
            SELECT DISTINCT d.code, h.issuer_name, d.description
            FROM public.invoice_detail d
            JOIN public.invoice_header h ON d.cufe = h.cufe
            WHERE h.user_id = $1
        ) u ON p.code = u.code
           AND p.issuer_name = u.issuer_name
           AND p.description = u.description
        ORDER BY p.process_date DESC, p.issuer_name ASC, p.description ASC
        LIMIT $2 OFFSET $3
        "#
    }
    
    /// get_user_products_with_date_filter - Get products with process_date filter
    pub fn get_user_products_with_date_filter_query() -> &'static str {
        r#"
        SELECT 
            p.code,
            p.issuer_name,
            p.description,
            p.l1_gemini as l1,
            p.l2_gemini as l2,
            p.l3_gemini as l3,
            p.l4_gemini as l4,
            p.process_date
        FROM public.dim_product p
        JOIN (
            SELECT DISTINCT d.code, h.issuer_name, d.description
            FROM public.invoice_detail d
            JOIN public.invoice_header h ON d.cufe = h.cufe
            WHERE h.user_id = $1
        ) u ON p.code = u.code
           AND p.issuer_name = u.issuer_name
           AND p.description = u.description
        WHERE p.process_date >= $4
        ORDER BY p.process_date DESC, p.issuer_name ASC, p.description ASC
        LIMIT $2 OFFSET $3
        "#
    }
    
    /// Count query for pagination
    pub fn get_user_products_count_query() -> &'static str {
        r#"
        SELECT COUNT(*) as total
        FROM public.dim_product p
        JOIN (
            SELECT DISTINCT d.code, h.issuer_name, d.description
            FROM public.invoice_detail d
            JOIN public.invoice_header h ON d.cufe = h.cufe
            WHERE h.user_id = $1
        ) u ON p.code = u.code
           AND p.issuer_name = u.issuer_name
           AND p.description = u.description
        "#
    }
    
    /// Count query for pagination with date filter
    pub fn get_user_products_count_with_date_filter_query() -> &'static str {
        r#"
        SELECT COUNT(*) as total
        FROM public.dim_product p
        JOIN (
            SELECT DISTINCT d.code, h.issuer_name, d.description
            FROM public.invoice_detail d
            JOIN public.invoice_header h ON d.cufe = h.cufe
            WHERE h.user_id = $1
        ) u ON p.code = u.code
           AND p.issuer_name = u.issuer_name
           AND p.description = u.description
        WHERE p.process_date >= $2
        "#
    }
    
    pub fn get_user_products_cache_key_prefix() -> &'static str {
        "user_products_list"
    }
    
    pub fn get_user_products_cache_ttl() -> u64 {
        600 // 10 minutes cache
    }
}

/// Response model for get_user_products
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserProductsResponse {
    pub code: Option<String>,
    pub issuer_name: Option<String>,
    pub description: Option<String>,
    pub l1: Option<String>,
    pub l2: Option<String>,
    pub l3: Option<String>,
    pub l4: Option<String>,
    pub process_date: Option<String>,
}

/// Count result for pagination
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserProductsCount {
    pub total: i64,
}

/// Request model for get_user_products (for pagination and filters)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProductsRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>, // Future: for filtering by product description
    pub update_date_from: Option<String>, // Filter by process_date >= this date (ISO 8601 format)
}

/// Complete response with pagination
#[derive(Debug, Serialize, Deserialize)]
pub struct UserProductsPagedResponse {
    pub products: Vec<UserProductsResponse>,
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

/// Cache invalidation patterns for user_products
pub struct UserProductsCachePatterns;

impl UserProductsCachePatterns {
    pub fn invalidate_patterns(user_id: i32) -> Vec<String> {
        vec![
            format!("user_products_list_{}*", user_id),
            format!("user_products_count_{}*", user_id),
        ]
    }
}

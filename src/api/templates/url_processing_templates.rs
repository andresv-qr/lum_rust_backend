use serde::{Deserialize, Serialize};

// ============================================================================
// REQUEST MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ProcessUrlRequest {
    pub url: String,
    pub source: Option<String>, // "APP", "WHATSAPP", etc.
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ProcessUrlResponse {
    pub success: bool,
    pub message: String,
    pub process_type: Option<String>, // "CUFE", "QR"
    pub invoice_id: Option<i32>,
    pub cufe: Option<String>,
    pub processing_time_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct UrlValidationResponse {
    pub is_valid: bool,
    pub process_type: Option<String>,
    pub domain: Option<String>,
    pub error: Option<String>,
}

// ============================================================================
// DATABASE QUERY TEMPLATES
// ============================================================================

#[derive(Debug)]
pub struct UrlProcessingQueries;

impl UrlProcessingQueries {
    /// Check if URL has been processed recently (duplicate prevention)
    pub const CHECK_RECENT_PROCESSING: &'static str = r#"
        SELECT id, cufe, created_at
        FROM public.invoice_headers 
        WHERE user_id = $1 
        AND source_url = $2 
        AND created_at > NOW() - INTERVAL '1 hour'
        ORDER BY created_at DESC
        LIMIT 1
    "#;

    /// Log URL processing attempt
    pub const LOG_PROCESSING_ATTEMPT: &'static str = r#"
        INSERT INTO public.url_processing_logs 
        (user_id, url, process_type, source, success, error_message, processing_time_ms, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        RETURNING id
    "#;

    /// Get user processing stats for rate limiting
    pub const GET_USER_PROCESSING_STATS: &'static str = r#"
        SELECT COUNT(*) as count
        FROM public.url_processing_logs 
        WHERE user_id = $1 
        AND created_at > NOW() - INTERVAL '1 hour'
    "#;
}

// ============================================================================
// HELPER STRUCTS
// ============================================================================

#[derive(Debug)]
pub struct ProcessingResult {
    pub success: bool,
    pub invoice_id: Option<i32>,
    pub cufe: Option<String>,
    pub error_message: Option<String>,
    pub processing_time_ms: u64,
}

#[derive(Debug)]
pub struct UrlValidation {
    pub is_valid: bool,
    pub process_type: Option<String>,
    pub domain: Option<String>,
    pub normalized_url: Option<String>,
}

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

impl ProcessUrlRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.url.trim().is_empty() {
            return Err("URL is required".to_string());
        }
        
        // Basic URL format validation
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err("URL must start with http:// or https://".to_string());
        }
        
        // URL length validation
        if self.url.len() > 2048 {
            return Err("URL is too long (max 2048 characters)".to_string());
        }
        
        Ok(())
    }
    
    pub fn normalize_url(&self) -> String {
        self.url.trim().to_lowercase()
    }
}

// ============================================================================
// URL VALIDATION LOGIC
// ============================================================================

pub fn validate_panama_invoice_url(url: &str) -> UrlValidation {
    let normalized_url = url.trim().to_lowercase();
    
    // Define allowed domains and patterns for Panama electronic invoices
    let allowed_patterns = vec![
        // CUFE pattern
        ("consultas/facturasporcufe", "CUFE"),
        // QR patterns - add your specific allowed URLs here
        ("dgi-fep.mef.gob.pa", "QR"),
        ("fep.mef.gob.pa", "QR"),
        // Add more patterns as needed
    ];
    
    for (pattern, process_type) in allowed_patterns {
        if normalized_url.contains(pattern) {
            // Extract domain
            let domain = extract_domain(&normalized_url);
            
            return UrlValidation {
                is_valid: true,
                process_type: Some(process_type.to_string()),
                domain,
                normalized_url: Some(normalized_url),
            };
        }
    }
    
    UrlValidation {
        is_valid: false,
        process_type: None,
        domain: extract_domain(&normalized_url),
        normalized_url: Some(normalized_url),
    }
}

fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed_url) = url::Url::parse(url) {
        parsed_url.host_str().map(|s| s.to_string())
    } else {
        None
    }
}

// ============================================================================
// RESPONSE HELPERS
// ============================================================================

impl ProcessUrlResponse {
    pub fn success(
        process_type: &str,
        invoice_id: Option<i32>,
        cufe: Option<String>,
        processing_time_ms: u64,
    ) -> Self {
        Self {
            success: true,
            message: format!("URL processed successfully ({})", process_type),
            process_type: Some(process_type.to_string()),
            invoice_id,
            cufe,
            processing_time_ms: Some(processing_time_ms),
        }
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            process_type: None,
            invoice_id: None,
            cufe: None,
            processing_time_ms: None,
        }
    }
    
    pub fn duplicate(cufe: &str, processing_time_ms: u64) -> Self {
        Self {
            success: true,
            message: format!("Invoice already processed recently (CUFE: {})", cufe),
            process_type: Some("DUPLICATE".to_string()),
            invoice_id: None,
            cufe: Some(cufe.to_string()),
            processing_time_ms: Some(processing_time_ms),
        }
    }
}

impl UrlValidationResponse {
    pub fn valid(process_type: &str, domain: Option<String>) -> Self {
        Self {
            is_valid: true,
            process_type: Some(process_type.to_string()),
            domain,
            error: None,
        }
    }
    
    pub fn invalid(error: &str, domain: Option<String>) -> Self {
        Self {
            is_valid: false,
            process_type: None,
            domain,
            error: Some(error.to_string()),
        }
    }
}

// ============================================================================
// RATE LIMITING HELPERS
// ============================================================================

pub struct RateLimitConfig {
    pub max_requests_per_hour: i32,
    pub max_requests_per_minute: i32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_hour: 50,
            max_requests_per_minute: 10,
        }
    }
}

pub async fn check_rate_limit(
    db_pool: &sqlx::PgPool,
    user_id: i32,
    config: &RateLimitConfig,
) -> Result<bool, sqlx::Error> {
    // Check hourly limit
    let hourly_count: (i64,) = sqlx::query_as(UrlProcessingQueries::GET_USER_PROCESSING_STATS)
        .bind(user_id)
        .fetch_one(db_pool)
        .await?;
    
    if hourly_count.0 >= config.max_requests_per_hour as i64 {
        return Ok(false);
    }
    
    // Check minute limit
    let minute_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM public.url_processing_logs WHERE user_id = $1 AND created_at > NOW() - INTERVAL '1 minute'"
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await?;
    
    if minute_count.0 >= config.max_requests_per_minute as i64 {
        return Ok(false);
    }
    
    Ok(true)
}

// ============================================================================
// PROCESSING UTILITIES
// ============================================================================

pub fn extract_cufe_from_url(url: &str) -> Option<String> {
    // Extract CUFE from URL patterns
    if url.contains("consultas/facturasporcufe") {
        // Try to extract CUFE parameter from URL
        if let Ok(parsed_url) = url::Url::parse(url) {
            for (key, value) in parsed_url.query_pairs() {
                if key.to_lowercase() == "cufe" {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

pub fn sanitize_url_for_logging(url: &str) -> String {
    // Remove sensitive parameters while keeping the URL structure for logging
    if let Ok(mut parsed_url) = url::Url::parse(url) {
        parsed_url.set_query(None);
        parsed_url.to_string()
    } else {
        url.to_string()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_panama_invoice_url() {
        // Test CUFE URL
        let cufe_url = "https://example.com/consultas/facturasporcufe?cufe=123456";
        let validation = validate_panama_invoice_url(cufe_url);
        assert!(validation.is_valid);
        assert_eq!(validation.process_type, Some("CUFE".to_string()));
        
        // Test invalid URL
        let invalid_url = "https://google.com";
        let validation = validate_panama_invoice_url(invalid_url);
        assert!(!validation.is_valid);
        assert_eq!(validation.process_type, None);
    }
    
    #[test]
    fn test_extract_cufe_from_url() {
        let url = "https://example.com/consultas/facturasporcufe?cufe=ABC123XYZ";
        let cufe = extract_cufe_from_url(url);
        assert_eq!(cufe, Some("ABC123XYZ".to_string()));
        
        let url_no_cufe = "https://example.com/other";
        let cufe = extract_cufe_from_url(url_no_cufe);
        assert_eq!(cufe, None);
    }
    
    #[test]
    fn test_process_url_request_validation() {
        // Valid request
        let valid_req = ProcessUrlRequest {
            url: "https://example.com/test".to_string(),
            source: Some("APP".to_string()),
        };
        assert!(valid_req.validate().is_ok());
        
        // Invalid request - empty URL
        let invalid_req = ProcessUrlRequest {
            url: "".to_string(),
            source: None,
        };
        assert!(invalid_req.validate().is_err());
        
        // Invalid request - no protocol
        let invalid_req = ProcessUrlRequest {
            url: "example.com/test".to_string(),
            source: None,
        };
        assert!(invalid_req.validate().is_err());
    }
}

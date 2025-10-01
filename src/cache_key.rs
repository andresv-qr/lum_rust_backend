//! Cache key generation utilities
//! 
//! This module provides consistent cache key generation
//! across the application for different data types.

/// Generate cache key for invoice headers
pub fn invoice_headers(limit: u32, offset: u32, filters: &str) -> String {
    format!("invoice_headers_{}_{}_filters:{}", limit, offset, filters)
}

/// Generate cache key for user metrics by email
pub fn metrics_user_email(email: &str) -> String {
    format!("metrics_user_email:{}", email)
}

/// Generate cache key for user metrics by user ID
pub fn metrics_user_id(user_id: i64) -> String {
    format!("metrics_user_id:{}", user_id)
}

/// Generate cache key for QR scan results
pub fn qr_scan_l2(qr_hash: &str) -> String {
    format!("qr_scan_l2:{}", qr_hash)
}

/// Generate cache key for OCR results
pub fn ocr_result_l2(doc_hash: &str) -> String {
    format!("ocr_result_l2:{}", doc_hash)
}

/// Generate cache key for user sessions
pub fn user_session(user_id: i64, session_id: &str) -> String {
    format!("user_session:{}:{}", user_id, session_id)
}

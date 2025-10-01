//! Centralized TTL constants for caching
//! 
//! This module provides consistent cache time-to-live values
//! across the application with environment variable overrides.

use std::env;

// Default TTL constants (in seconds)
pub const TTL_INVOICE_HEADERS: u64 = 300; // 5 minutes
pub const TTL_METRICS: u64 = 300; // 5 minutes 
pub const TTL_QR_SCAN_L2: u64 = 1800; // 30 minutes
pub const TTL_OCR_RESULT_L2: u64 = 3600; // 1 hour
pub const TTL_USER_SESSION: u64 = 900; // 15 minutes
pub const TTL_DEFAULT: u64 = 300; // 5 minutes

/// Get TTL with environment variable override
pub fn ttl_with_env(env_key: &str, default_ttl: u64) -> u64 {
    env::var(env_key)
        .map(|val| val.parse::<u64>().unwrap_or(default_ttl))
        .unwrap_or(default_ttl)
}

/// Get invoice headers TTL from environment or default
pub fn get_invoice_headers_ttl() -> u64 {
    ttl_with_env("TTL_INVOICE_HEADERS_SECONDS", TTL_INVOICE_HEADERS)
}

/// Get metrics TTL from environment or default
pub fn get_metrics_ttl() -> u64 {
    ttl_with_env("TTL_METRICS_SECONDS", TTL_METRICS)
}

/// Get QR scan L2 TTL from environment or default
pub fn get_qr_scan_l2_ttl() -> u64 {
    ttl_with_env("TTL_QR_SCAN_L2_SECONDS", TTL_QR_SCAN_L2)
}

/// Get OCR result L2 TTL from environment or default
pub fn get_ocr_result_l2_ttl() -> u64 {
    ttl_with_env("TTL_OCR_RESULT_L2_SECONDS", TTL_OCR_RESULT_L2)
}

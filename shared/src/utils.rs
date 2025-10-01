//! Utility functions and helpers

use crate::{error::AppError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a hash for data (useful for caching)
pub fn generate_hash<T: Hash>(data: &T) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Generate a hash for image data
pub fn generate_image_hash(image_data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    image_data.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Validate email format
pub fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.len() > 5 && email.len() < 255
}

/// Validate phone number format (basic validation)
pub fn is_valid_phone(phone: &str) -> bool {
    phone.chars().all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == ' ')
        && phone.len() >= 7
        && phone.len() <= 20
}

/// Sanitize text input
pub fn sanitize_text(text: &str) -> String {
    text.trim()
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Format currency amount
pub fn format_currency(amount: rust_decimal::Decimal) -> String {
    format!("${:.2}", amount)
}

/// Parse CUFE from URL
pub fn extract_cufe_from_url(url: &str) -> Option<String> {
    // Extract CUFE from DGI URL patterns
    if let Some(start) = url.find("cufe=") {
        let cufe_start = start + 5;
        if let Some(end) = url[cufe_start..].find('&') {
            Some(url[cufe_start..cufe_start + end].to_string())
        } else {
            Some(url[cufe_start..].to_string())
        }
    } else {
        None
    }
}

/// Validate CUFE format
pub fn is_valid_cufe(cufe: &str) -> bool {
    // CUFE should be a UUID-like string
    cufe.len() == 36 && cufe.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Generate verification code
pub fn generate_verification_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(100000..999999))
}

/// Calculate processing time
pub fn calculate_processing_time(start: std::time::Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

/// Convert bytes to human readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Truncate text to specified length
pub fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    }
}

/// Parse duration from string (e.g., "30s", "5m", "1h")
pub fn parse_duration(duration_str: &str) -> Result<std::time::Duration> {
    let duration_str = duration_str.trim().to_lowercase();
    
    if let Some(num_str) = duration_str.strip_suffix('s') {
        let seconds: u64 = num_str.parse()
            .map_err(|_| AppError::validation("Invalid duration format"))?;
        Ok(std::time::Duration::from_secs(seconds))
    } else if let Some(num_str) = duration_str.strip_suffix('m') {
        let minutes: u64 = num_str.parse()
            .map_err(|_| AppError::validation("Invalid duration format"))?;
        Ok(std::time::Duration::from_secs(minutes * 60))
    } else if let Some(num_str) = duration_str.strip_suffix('h') {
        let hours: u64 = num_str.parse()
            .map_err(|_| AppError::validation("Invalid duration format"))?;
        Ok(std::time::Duration::from_secs(hours * 3600))
    } else {
        // Default to seconds if no suffix
        let seconds: u64 = duration_str.parse()
            .map_err(|_| AppError::validation("Invalid duration format"))?;
        Ok(std::time::Duration::from_secs(seconds))
    }
}

/// Retry mechanism for async operations
pub async fn retry_async<F, Fut, T, E>(
    mut operation: F,
    max_retries: usize,
    delay: std::time::Duration,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut last_error = None;
    
    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                last_error = Some(error);
                if attempt < max_retries {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}

/// Rate limiting helper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    pub max_requests: u32,
    pub window_duration: std::time::Duration,
    pub requests: std::collections::VecDeque<DateTime<Utc>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_duration: std::time::Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            requests: std::collections::VecDeque::new(),
        }
    }

    pub fn is_allowed(&mut self) -> bool {
        let now = Utc::now();
        let window_start = now - chrono::Duration::from_std(self.window_duration).unwrap();

        // Remove old requests outside the window
        while let Some(&front) = self.requests.front() {
            if front < window_start {
                self.requests.pop_front();
            } else {
                break;
            }
        }

        // Check if we can add a new request
        if self.requests.len() < self.max_requests as usize {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }

    pub fn remaining_requests(&self) -> u32 {
        self.max_requests.saturating_sub(self.requests.len() as u32)
    }

    pub fn reset_time(&self) -> Option<DateTime<Utc>> {
        self.requests.front().map(|&first| {
            first + chrono::Duration::from_std(self.window_duration).unwrap()
        })
    }
}

/// Image processing utilities
pub mod image_utils {
    /// Validate image format
    pub fn is_valid_image_format(data: &[u8]) -> bool {
        // Check for common image headers
        if data.len() < 4 {
            return false;
        }

        // JPEG
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return true;
        }

        // PNG
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return true;
        }

        // GIF
        if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            return true;
        }

        // WebP
        if data.len() >= 12 && data[0..4] == [0x52, 0x49, 0x46, 0x46] && data[8..12] == [0x57, 0x45, 0x42, 0x50] {
            return true;
        }

        false
    }

    /// Get image format from data
    pub fn get_image_format(data: &[u8]) -> Option<&'static str> {
        if data.len() < 4 {
            return None;
        }

        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some("jpeg")
        } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            Some("png")
        } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            Some("gif")
        } else if data.len() >= 12 && data[0..4] == [0x52, 0x49, 0x46, 0x46] && data[8..12] == [0x57, 0x45, 0x42, 0x50] {
            Some("webp")
        } else {
            None
        }
    }

    /// Validate image size
    pub fn is_valid_image_size(data: &[u8], max_size_mb: u64) -> bool {
        let max_size_bytes = max_size_mb * 1024 * 1024;
        data.len() <= max_size_bytes as usize
    }
}

/// Text processing utilities
pub mod text_utils {
    /// Clean and normalize text
    pub fn normalize_text(text: &str) -> String {
        text.trim()
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// Extract numbers from text
    pub fn extract_numbers(text: &str) -> Vec<String> {
        use regex::Regex;
        let re = Regex::new(r"\d+(?:\.\d+)?").unwrap();
        re.find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Extract emails from text
    pub fn extract_emails(text: &str) -> Vec<String> {
        use regex::Regex;
        let re = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
        re.find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Count words in text
    pub fn word_count(text: &str) -> usize {
        text.split_whitespace().count()
    }
}

/// Validation utilities
pub mod validation {
    use super::*;

    /// Validate user ID format
    pub fn is_valid_user_id(user_id: &str) -> bool {
        !user_id.is_empty() && user_id.len() <= 255 && user_id.chars().all(|c| !c.is_control())
    }

    /// Validate source format
    pub fn is_valid_source(source: &str) -> bool {
        matches!(source, "whatsapp" | "telegram" | "email" | "api")
    }

    /// Validate pagination parameters
    pub fn validate_pagination(page: u32, limit: u32) -> Result<()> {
        if page == 0 {
            return Err(AppError::validation("Page must be greater than 0"));
        }
        if limit == 0 || limit > 100 {
            return Err(AppError::validation("Limit must be between 1 and 100"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_email() {
        assert!(is_valid_email("test@example.com"));
        assert!(!is_valid_email("invalid-email"));
        assert!(!is_valid_email("@example.com"));
    }

    #[test]
    fn test_generate_hash() {
        let data = "test data";
        let hash1 = generate_hash(&data);
        let hash2 = generate_hash(&data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sanitize_text() {
        let input = "  Hello\x00World\t  ";
        let expected = "HelloWorld\t";
        assert_eq!(sanitize_text(input), expected);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), std::time::Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), std::time::Duration::from_secs(300));
        assert_eq!(parse_duration("1h").unwrap(), std::time::Duration::from_secs(3600));
    }
}
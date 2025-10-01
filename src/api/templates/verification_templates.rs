use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ============================================================================
// REQUEST MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SendVerificationRequest {
    pub email: String,
    pub method: Option<String>, // "email" or "whatsapp"
}

#[derive(Debug, Deserialize)]
pub struct VerifyAccountRequest {
    pub email: String,
    pub verification_code: String,
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SendVerificationResponse {
    pub success: bool,
    pub message: String,
    pub method: String, // "email" or "whatsapp"
    pub expires_in: i32, // seconds
}

#[derive(Debug, Serialize)]
pub struct VerifyAccountResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<i32>,
}

// ============================================================================
// DATABASE QUERY TEMPLATES
// ============================================================================

pub struct VerificationQueries;

impl VerificationQueries {
    /// Get user by email with verification info
    pub const GET_USER_FOR_VERIFICATION: &'static str = r#"
        SELECT id, email, password_hash, ws_id, telegram_id, name, created_at
        FROM public.dim_users 
        WHERE LOWER(email) = LOWER($1)
        LIMIT 1
    "#;

    /// Update user password
    pub const UPDATE_USER_PASSWORD: &'static str = r#"
        UPDATE public.dim_users 
        SET password = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, email
    "#;

    /// Log verification attempt
    pub const LOG_VERIFICATION_ATTEMPT: &'static str = r#"
        INSERT INTO public.user_verification_logs 
        (user_id, email, verification_type, method, success, ip_address, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
    "#;
}

// ============================================================================
// HELPER STRUCTS
// ============================================================================

#[derive(Debug)]
pub struct UserVerificationData {
    pub id: i32,
    pub email: String,
    pub password: Option<String>,
    pub ws_id: Option<String>,
    pub telegram_id: Option<String>,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VerificationCode {
    pub code: String,
    pub email: String,
    pub code_type: String, // "set_password", "reset_password", "verify_account"
    pub expires_at: DateTime<Utc>,
    pub attempts: i32,
}

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

impl SendVerificationRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.email.trim().is_empty() {
            return Err("Email is required".to_string());
        }
        
        if !is_valid_email(&self.email) {
            return Err("Invalid email format".to_string());
        }
        
        // Validate method if provided
        if let Some(method) = &self.method {
            match method.as_str() {
                "email" | "whatsapp" => {},
                _ => return Err("Method must be 'email' or 'whatsapp'".to_string()),
            }
        }
        
        Ok(())
    }
}

impl VerifyAccountRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.email.trim().is_empty() {
            return Err("Email is required".to_string());
        }
        
        if !is_valid_email(&self.email) {
            return Err("Invalid email format".to_string());
        }
        
        if self.verification_code.trim().is_empty() {
            return Err("Verification code is required".to_string());
        }
        
        if self.verification_code.len() != 6 {
            return Err("Verification code must be 6 digits".to_string());
        }
        
        Ok(())
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

fn is_valid_email(email: &str) -> bool {
    use regex::Regex;
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    email_regex.is_match(email)
}

// ============================================================================
// RESPONSE HELPERS
// ============================================================================

impl SendVerificationResponse {
    pub fn success(method: &str) -> Self {
        Self {
            success: true,
            message: format!("Verification code sent via {}", method),
            method: method.to_string(),
            expires_in: 3600, // 1 hour
        }
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            method: "none".to_string(),
            expires_in: 0,
        }
    }
}

impl VerifyAccountResponse {
    pub fn success(user_id: i32) -> Self {
        Self {
            success: true,
            message: "Account verified successfully".to_string(),
            user_id: Some(user_id),
        }
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            user_id: None,
        }
    }
}

// ============================================================================
// AUTH REQUEST MODELS
// ============================================================================
// Date: September 18, 2025
// Purpose: Request models for unified authentication system
// ============================================================================

use serde::{Deserialize, Serialize};
use validator::Validate;

// ============================================================================
// UNIFIED AUTH REQUEST
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct UnifiedAuthRequest {
    #[validate(custom(function = "validate_provider"))]
    pub provider: String,              // "google" | "email"
    
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    
    // For Google OAuth2
    pub google_id_token: Option<String>,
    
    // For email auth
    #[validate(length(min = 8, max = 128, message = "Password must be between 8 and 128 characters"))]
    pub password: Option<String>,
    pub verification_code: Option<String>,
    
    // Metadata
    pub device_info: Option<DeviceInfo>,
    pub user_agent: Option<String>,
    pub remember_me: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub device_type: String,
    pub os: String,
    pub browser: Option<String>,
    pub app_version: Option<String>,
}

// ============================================================================
// ACCOUNT LINKING REQUEST
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct LinkAccountRequest {
    #[validate(length(min = 1, message = "Linking token is required"))]
    pub linking_token: String,
    
    pub confirmation: bool,           // User confirms linking
    
    #[validate(length(min = 4, max = 8, message = "Verification code must be 4-8 characters"))]
    pub verification_code: Option<String>,
}

// ============================================================================
// EMAIL VERIFICATION REQUEST
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyEmailRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    
    #[validate(length(min = 4, max = 8, message = "Verification code must be 4-8 characters"))]
    pub verification_code: String,
    
    pub purpose: Option<VerificationPurpose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationPurpose {
    EmailVerification,
    AccountLinking,
    PasswordReset,
}

impl std::fmt::Display for VerificationPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationPurpose::EmailVerification => write!(f, "email_verification"),
            VerificationPurpose::AccountLinking => write!(f, "account_linking"),
            VerificationPurpose::PasswordReset => write!(f, "password_reset"),
        }
    }
}

// ============================================================================
// RESEND VERIFICATION REQUEST
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct ResendVerificationRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    
    pub purpose: Option<VerificationPurpose>,
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

use validator::ValidationError;

pub fn validate_provider(provider: &str) -> Result<(), ValidationError> {
    match provider {
        "google" | "email" => Ok(()),
        _ => Err(ValidationError::new("invalid_provider")),
    }
}

pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("password_too_short"));
    }
    
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_digit(10));
    
    if !has_uppercase || !has_lowercase || !has_digit {
        return Err(ValidationError::new("password_too_weak"));
    }
    
    Ok(())
}

// ============================================================================
// USER REGISTRATION REQUEST (LEGACY COMPATIBILITY)
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct UserRegistrationRequest {
    #[validate(email(message = "Invalid email format"))]
    #[validate(length(min = 5, max = 100, message = "Email must be between 5 and 100 characters"))]
    pub email: String,
    
    #[validate(length(min = 8, max = 128, message = "Password must be between 8 and 128 characters"))]
    #[validate(custom(function = "validate_password_strength"))]
    pub password: String,
    
    #[validate(length(min = 1, max = 100, message = "Name must be between 1 and 100 characters"))]
    pub name: String,
    
    pub phone: Option<String>,
    pub country: Option<String>,
}
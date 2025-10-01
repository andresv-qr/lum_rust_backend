// ============================================================================
// AUTH RESPONSE MODELS
// ============================================================================
// Date: September 18, 2025
// Purpose: Response models for unified authentication system
// ============================================================================

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::user::UserResponse;

// ============================================================================
// UNIFIED AUTH RESPONSE
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UnifiedAuthResponse {
    pub success: bool,
    pub response_type: AuthResponseType,
    pub user: Option<UserResponse>,
    pub tokens: Option<AuthTokens>,
    pub linking_token: Option<String>,    // For account linking
    pub verification_required: Option<VerificationRequired>,
    pub message: String,
    pub metadata: serde_json::Value,
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl Default for UnifiedAuthResponse {
    fn default() -> Self {
        Self {
            success: false,
            response_type: AuthResponseType::Error,
            user: None,
            tokens: None,
            linking_token: None,
            verification_required: None,
            message: String::new(),
            metadata: serde_json::json!({}),
            request_id: None,
            timestamp: Utc::now(),
        }
    }
}

// ============================================================================
// AUTH RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthResponseType {
    Success,                  // Login/registro exitoso
    RequiresPassword,         // Email existe, necesita password
    RequiresVerification,     // Necesita verificar email
    RequiresLinking,          // Cuenta existe con otro provider
    AccountLocked,            // Cuenta bloqueada
    Error,                    // Error general
}

// ============================================================================
// AUTH TOKENS
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub token_type: String,
}

impl Default for AuthTokens {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            refresh_token: None,
            expires_in: 86400, // 24 hours
            token_type: "bearer".to_string(),
        }
    }
}

// ============================================================================
// VERIFICATION REQUIRED
// ============================================================================

#[derive(Debug, Serialize)]
pub struct VerificationRequired {
    pub method: String,               // "email", "sms"
    pub destination: String,          // Email masqueado
    pub expires_in: i64,
    pub resend_available_in: Option<i64>,
    pub code_length: Option<usize>,
    pub attempts_remaining: Option<u32>,
}

// ============================================================================
// TOKEN RESPONSE (LEGACY COMPATIBILITY)
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user_id: i64,
    pub email: String,
}

impl From<AuthTokens> for TokenResponse {
    fn from(tokens: AuthTokens) -> Self {
        Self {
            access_token: tokens.access_token,
            token_type: tokens.token_type,
            expires_in: tokens.expires_in,
            user_id: 0, // Will be set by caller
            email: String::new(), // Will be set by caller
        }
    }
}

// ============================================================================
// VERIFICATION RESULT
// ============================================================================

#[derive(Debug)]
pub enum VerificationResult {
    Success(crate::models::user::User),
    InvalidCode,
    Expired,
    TooManyAttempts,
}

// ============================================================================
// ERROR RESPONSE
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub success: bool,
    pub error: ErrorDetails,
    pub request_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retry_after: Option<i64>,
}

// ============================================================================
// ACCOUNT LINKING RESPONSE
// ============================================================================

#[derive(Debug, Serialize)]
pub struct LinkingResponse {
    pub success: bool,
    pub message: String,
    pub existing_providers: Vec<String>,
    pub new_provider: String,
    pub linking_token: String,
    pub verification_required: bool,
    pub expires_in: i64,
}

// ============================================================================
// PROVIDER STATUS RESPONSE
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ProviderStatusResponse {
    pub user_id: i32,
    pub providers: Vec<ProviderInfo>,
    pub primary_provider: Option<String>,
    pub can_add_providers: bool,
    pub max_providers: usize,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub provider_type: String,
    pub provider_email: Option<String>,
    pub is_primary: bool,
    pub linked_at: DateTime<Utc>,
    pub link_method: String,
    pub verified: bool,
}

// ============================================================================
// EMAIL VERIFICATION RESPONSE
// ============================================================================

#[derive(Debug, Serialize)]
pub struct EmailVerificationResponse {
    pub success: bool,
    pub message: String,
    pub email_verified: bool,
    pub user: Option<UserResponse>,
    pub tokens: Option<AuthTokens>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

impl UnifiedAuthResponse {
    pub fn success_with_tokens(user: UserResponse, tokens: AuthTokens, message: &str) -> Self {
        Self {
            success: true,
            response_type: AuthResponseType::Success,
            user: Some(user),
            tokens: Some(tokens),
            message: message.to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        }
    }
    
    pub fn requires_verification(verification: VerificationRequired, message: &str) -> Self {
        Self {
            success: false,
            response_type: AuthResponseType::RequiresVerification,
            verification_required: Some(verification),
            message: message.to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        }
    }
    
    pub fn requires_linking(linking_token: String, verification: Option<VerificationRequired>, message: &str) -> Self {
        Self {
            success: false,
            response_type: AuthResponseType::RequiresLinking,
            linking_token: Some(linking_token),
            verification_required: verification,
            message: message.to_string(),
            timestamp: Utc::now(),
            ..Default::default()
        }
    }
    
    pub fn error(message: &str, metadata: Option<serde_json::Value>) -> Self {
        Self {
            success: false,
            response_type: AuthResponseType::Error,
            message: message.to_string(),
            metadata: metadata.unwrap_or_else(|| serde_json::json!({})),
            timestamp: Utc::now(),
            ..Default::default()
        }
    }
}
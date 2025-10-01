use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

// ============================================================================
// REQUEST MODELS
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
pub struct EmailCheckRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UserRegistrationRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UserLoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserStatusRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct SendVerificationRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyAccountRequest {
    pub email: String,
    pub verification_code: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SetPasswordRequest {
    pub email: String,
    pub verification_code: String,
    #[validate(length(min = 8))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub verification_code: String,
    #[validate(length(min = 8))]
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ProcessUrlRequest {
    pub url: String,
    pub source: Option<String>,
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct EmailCheckResponse {
    pub exists: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user_id: i64,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct UserStatusResponse {
    pub exists: bool,
    pub has_password: bool,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SendVerificationResponse {
    pub success: bool,
    pub message: String,
    pub method: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyAccountResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RegistrationResponse {
    pub success: bool,
    pub message: String,
    pub user_id: i64,
}

#[derive(Debug, Serialize)]
pub struct OCRResponse {
    pub success: bool,
    pub message: String,
    pub cufe: Option<String>,
    pub error_details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub creation_date: DateTime<Utc>,
    pub last_login_date: Option<DateTime<Utc>>,
    pub source: Option<String>,
}

// ============================================================================
// INVOICE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct InvoiceDetail {
    pub cufe: Option<String>,
    pub quantity: Option<f64>,
    pub code: Option<String>,
    pub date: Option<DateTime<Utc>>,
    pub total: Option<f64>,
    pub unit_price: Option<f64>,
    pub amount: Option<f64>,
    pub unit_discount: Option<String>,
    pub description: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceHeader {
    pub date: Option<DateTime<Utc>>,
    pub tot_itbms: Option<f64>,
    pub issuer_name: Option<String>,
    pub no: Option<String>,
    pub tot_amount: Option<f64>,
    pub url: Option<String>,
    pub process_date: Option<DateTime<Utc>>,
    pub reception_date: Option<DateTime<Utc>>,
    pub invoice_type: Option<String>,
    pub cufe: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceDetailResponse {
    pub invoices: Vec<InvoiceDetail>,
    pub count: usize,
    pub query_info: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct InvoiceHeaderResponse {
    pub invoices: Vec<InvoiceHeader>,
    pub count: usize,
    pub query_info: serde_json::Value,
}

// ============================================================================
// JWT MODELS
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub email: String,
    pub exp: usize, // Expiration time
    pub iat: usize, // Issued at
}

// ============================================================================
// ERROR MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<String>,
}

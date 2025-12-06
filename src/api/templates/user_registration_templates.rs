use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use regex::Regex;
use std::collections::HashMap;

// ============================================================================
// REQUEST MODELS
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
    #[validate(custom(function = "validate_name_format"))]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailCheckRequest {
    #[serde(deserialize_with = "deserialize_email")]
    pub email: String,
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UserRegistrationResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user_id: i32,
    pub email: String,
    pub name: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct EmailCheckResponse {
    pub exists: bool,
    pub message: String,
    pub has_password: Option<bool>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegistrationErrorResponse {
    pub success: bool,
    pub error: String,
    pub error_code: String,
    pub details: Option<HashMap<String, Vec<String>>>,
}

// ============================================================================
// DATABASE MODELS
// ============================================================================

#[derive(Debug)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub source: String,
    pub user_id_val: String,
}

#[derive(Debug)]
pub struct ExistingUser {
    pub id: i64,
    pub email: String,
    pub name: Option<String>,
    pub has_password: bool,
    pub source: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

/// Validate password strength requirements
fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let mut errors = Vec::new();
    
    if password.len() < 8 {
        errors.push("Password must be at least 8 characters long");
    }
    
    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Password must contain at least one uppercase letter");
    }
    
    if !password.chars().any(|c| c.is_lowercase()) {
        errors.push("Password must contain at least one lowercase letter");
    }
    
    if !password.chars().any(|c| c.is_numeric()) {
        errors.push("Password must contain at least one digit");
    }
    
    // Check for special characters (optional but recommended)
    if !password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) {
        // This is a warning, not an error
    }
    
    if !errors.is_empty() {
        let mut validation_error = ValidationError::new("password_strength");
        validation_error.message = Some(format!("Password validation failed: {}", errors.join(", ")).into());
        return Err(validation_error);
    }
    
    Ok(())
}

/// Validate name format (no special characters, reasonable length)
fn validate_name_format(name: &str) -> Result<(), ValidationError> {
    let name = name.trim();
    
    if name.is_empty() {
        let mut validation_error = ValidationError::new("name_empty");
        validation_error.message = Some("Name cannot be empty".into());
        return Err(validation_error);
    }
    
    // Allow letters, spaces, hyphens, and apostrophes
    let name_regex = Regex::new(r"^[a-zA-ZÀ-ÿ\s\-'\.]+$").unwrap();
    if !name_regex.is_match(name) {
        let mut validation_error = ValidationError::new("name_format");
        validation_error.message = Some("Name can only contain letters, spaces, hyphens, and apostrophes".into());
        return Err(validation_error);
    }
    
    // Check for reasonable length after trimming
    if name.len() > 100 {
        let mut validation_error = ValidationError::new("name_length");
        validation_error.message = Some("Name is too long (maximum 100 characters)".into());
        return Err(validation_error);
    }
    
    Ok(())
}

/// Custom email deserializer that normalizes email format
fn deserialize_email<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let email: String = String::deserialize(deserializer)?;
    Ok(email.trim().to_lowercase())
}

// ============================================================================
// QUERY TEMPLATES
// ============================================================================

pub struct UserRegistrationQueries;

impl UserRegistrationQueries {
    pub const CHECK_EMAIL_EXISTS: &'static str = r#"
        SELECT 
            id, 
            email, 
            name, 
            CASE WHEN password_hash IS NOT NULL AND password_hash != '' THEN true ELSE false END as has_password,
            source,
            created_at
        FROM public.dim_users 
        WHERE LOWER(email) = LOWER($1)
        LIMIT 1
    "#;
    
    pub const INSERT_NEW_USER: &'static str = r#"
        INSERT INTO public.dim_users (
            email, password_hash, name, source, user_id_val, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, NOW(), NOW()
        )
        ON CONFLICT (email) DO NOTHING
        RETURNING id, email, name, created_at
    "#;
    
    pub const GET_USER_BY_ID: &'static str = r#"
        SELECT id, email, name, created_at
        FROM public.dim_users 
        WHERE id = $1
    "#;
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Sanitize and normalize user input
pub fn sanitize_user_input(input: &str) -> String {
    input.trim().to_string()
}

/// Validate email format using regex
pub fn is_valid_email(email: &str) -> bool {
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    email_regex.is_match(email)
}

/// Generate user-friendly error messages
pub fn format_validation_errors(errors: &validator::ValidationErrors) -> HashMap<String, Vec<String>> {
    let mut formatted_errors = HashMap::new();
    
    for (field, field_errors) in errors.field_errors() {
        let mut field_messages = Vec::new();
        
        for error in field_errors {
            let message = match error.message {
                Some(ref msg) => msg.to_string(),
                None => match error.code.as_ref() {
                    "email" => "Invalid email format".to_string(),
                    "length" => "Invalid length".to_string(),
                    "password_strength" => "Password does not meet strength requirements".to_string(),
                    "name_format" => "Invalid name format".to_string(),
                    _ => "Invalid value".to_string(),
                }
            };
            field_messages.push(message);
        }
        
        formatted_errors.insert(field.to_string(), field_messages);
    }
    
    formatted_errors
}

// ============================================================================
// RESPONSE HELPERS
// ============================================================================

impl UserRegistrationResponse {
    pub fn success(
        access_token: String,
        expires_in: i64,
        user_id: i32,
        email: String,
        name: String,
    ) -> Self {
        Self {
            access_token,
            token_type: "bearer".to_string(),
            expires_in,
            user_id,
            email: email.clone(),
            name: name.clone(),
            message: format!("User {} registered successfully", email),
        }
    }
}

impl EmailCheckResponse {
    pub fn exists(email: &str, has_password: bool, source: Option<String>) -> Self {
        Self {
            exists: true,
            message: format!("Email {} is already registered", email),
            has_password: Some(has_password),
            source,
        }
    }
    
    pub fn not_exists(email: &str) -> Self {
        Self {
            exists: false,
            message: format!("Email {} is available for registration", email),
            has_password: None,
            source: None,
        }
    }
}

impl RegistrationErrorResponse {
    pub fn validation_error(details: HashMap<String, Vec<String>>) -> Self {
        Self {
            success: false,
            error: "Validation failed".to_string(),
            error_code: "VALIDATION_ERROR".to_string(),
            details: Some(details),
        }
    }
    
    pub fn email_exists(email: &str) -> Self {
        Self {
            success: false,
            error: format!("Email {} is already registered", email),
            error_code: "EMAIL_EXISTS".to_string(),
            details: None,
        }
    }
    
    pub fn database_error() -> Self {
        Self {
            success: false,
            error: "Database operation failed".to_string(),
            error_code: "DATABASE_ERROR".to_string(),
            details: None,
        }
    }
    
    pub fn internal_error(message: &str) -> Self {
        Self {
            success: false,
            error: message.to_string(),
            error_code: "INTERNAL_ERROR".to_string(),
            details: None,
        }
    }
}

// ============================================================================
// CONSTANTS
// ============================================================================

pub const EMAIL_APP_SOURCE: &str = "EMAIL";
pub const MAX_REGISTRATION_ATTEMPTS_PER_HOUR: u32 = 5;
pub const JWT_EXPIRATION_HOURS: i64 = 24 * 90;  // 90 days

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query templates for user registration operations
pub struct RegisterQueryTemplates;

impl RegisterQueryTemplates {
    /// check_email_exists - Check if email already exists in the system
    pub fn check_email_exists_query() -> &'static str {
        "SELECT id FROM public.dim_users WHERE email = $1"
    }
    
    /// insert_new_user - Insert new user into the database
    pub fn insert_new_user_query() -> &'static str {
        "INSERT INTO public.dim_users (email, password, name, created_at, updated_at) 
         VALUES ($1, $2, $3, NOW(), NOW()) 
         RETURNING id, email, name, created_at"
    }
    
    /// Cache key prefix for register operations
    pub fn get_cache_key_prefix() -> &'static str {
        "register"
    }
}

/// Request model for user registration
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct RegisterRequest {
    #[validate(email(message = "Email must be a valid email address"))]
    pub email: String,
    
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
    
    #[validate(length(min = 1, max = 100, message = "Name must be between 1 and 100 characters"))]
    pub name: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// Response model for successful user registration
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct RegisterResponse {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub registration_success: bool,
    pub welcome_message: String,
}

/// Response model for email existence check
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct EmailExistsResponse {
    pub id: Option<i64>,
    pub exists: bool,
}

/// Password strength validation
pub struct PasswordValidator;

impl PasswordValidator {
    pub fn is_strong(password: &str) -> bool {
        if password.len() < 8 {
            return false;
        }
        
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        
        has_uppercase && has_lowercase && has_digit
    }
    
    pub fn get_strength_message() -> &'static str {
        "Password must be at least 8 characters long and include uppercase letters, lowercase letters, and numbers"
    }
}

/// Cache invalidation patterns for register operations
pub struct RegisterCachePatterns;

impl RegisterCachePatterns {
    pub fn invalidate_patterns(email: &str) -> Vec<String> {
        vec![
            format!("register_*_{}", email),
            format!("user_*_{}", email),
            format!("email_check_*_{}", email),
            "register_*".to_string(),
        ]
    }
}

/// Registration business logic helpers
pub struct RegistrationHelpers;

impl RegistrationHelpers {
    pub fn normalize_email(email: &str) -> String {
        email.to_lowercase().trim().to_string()
    }
    
    pub fn normalize_name(name: &str) -> String {
        name.trim().to_string()
    }
    
    pub fn generate_welcome_message(name: &str) -> String {
        format!(
            "🎉 ¡Bienvenido a Lüm, {}! 🎉\n\n✅ Tu cuenta ha sido creada exitosamente\n🚀 Ya puedes empezar a ganar Lümis con tus facturas\n📱 Envía una imagen con código QR para comenzar\n\n💡 Usa /ayuda para ver todos los comandos disponibles",
            name
        )
    }
    
    pub fn generate_error_message(error_type: &str) -> String {
        match error_type {
            "email_exists" => "❌ Este email ya está registrado en el sistema. ¿Ya tienes una cuenta? Intenta iniciar sesión.".to_string(),
            "weak_password" => format!("❌ {}", PasswordValidator::get_strength_message()),
            "invalid_email" => "❌ Por favor, proporciona un email válido.".to_string(),
            "invalid_name" => "❌ El nombre debe tener entre 1 y 100 caracteres.".to_string(),
            "database_error" => "❌ Error interno del servidor. Por favor, intenta más tarde.".to_string(),
            _ => "❌ Error desconocido durante el registro.".to_string(),
        }
    }
}

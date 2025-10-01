use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query templates for authentication operations
pub struct AuthQueryTemplates;

impl AuthQueryTemplates {
    /// authenticate_user - Validate user credentials and get user data
    pub fn authenticate_user_query() -> &'static str {
        "SELECT id, email, password, name, created_at, updated_at, is_active 
         FROM public.dim_users 
         WHERE email = $1 AND is_active = true"
    }
    
    /// get_user_by_id - Get user data by ID for status check
    pub fn get_user_by_id_query() -> &'static str {
        "SELECT id, email, name, created_at, updated_at, is_active, last_login_at
         FROM public.dim_users 
         WHERE id = $1 AND is_active = true"
    }
    
    /// update_last_login - Update user's last login timestamp
    pub fn update_last_login_query() -> &'static str {
        "UPDATE public.dim_users 
         SET last_login_at = NOW(), updated_at = NOW() 
         WHERE id = $1"
    }
    
    /// Cache key prefix for auth operations
    pub fn get_cache_key_prefix() -> &'static str {
        "auth"
    }
}

/// Request model for user login
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct LoginRequest {
    #[validate(email(message = "Email must be a valid email address"))]
    pub email: String,
    
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember_me: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_info: Option<String>,
}

/// Request model for status check
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct StatusCheckRequest {
    #[validate(range(min = 1, message = "User ID must be a positive number"))]
    pub user_id: i64,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_permissions: Option<bool>,
}

/// Response model for successful login
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginResponse {
    pub user_id: i64,
    pub email: String,
    pub name: String,
    pub login_success: bool,
    pub session_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub welcome_back_message: String,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    
    // Compatibility fields for frontend client
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Response model for status check
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct StatusResponse {
    pub user_id: i64,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub session_valid: bool,
    pub account_status: String,
}

/// Internal user data from database
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct UserAuthData {
    pub id: i64,
    pub email: String,
    pub password_hash: Option<String>,  // Changed to Option to handle NULL values
    pub name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

/// Session management helpers
pub struct SessionManager;

impl SessionManager {
    /// Generate a secure session token
    pub fn generate_session_token(user_id: i64) -> String {
        use uuid::Uuid;
        format!("lum_session_{}_{}", user_id, Uuid::new_v4())
    }
    
    /// Calculate session expiration (24 hours from now)
    pub fn calculate_expiration(remember_me: bool) -> chrono::DateTime<chrono::Utc> {
        let hours = if remember_me { 24 * 30 } else { 24 }; // 30 days vs 24 hours
        chrono::Utc::now() + chrono::Duration::hours(hours)
    }
    
    /// Validate session token format
    pub fn is_valid_token_format(token: &str) -> bool {
        token.starts_with("lum_session_") && token.len() > 20
    }
}

/// Authentication business logic helpers
pub struct AuthHelpers;

impl AuthHelpers {
    pub fn normalize_email(email: &str) -> String {
        email.to_lowercase().trim().to_string()
    }
    
    pub fn generate_welcome_back_message(name: &str, last_login: Option<chrono::DateTime<chrono::Utc>>) -> String {
        match last_login {
            Some(last) => {
                let days_ago = (chrono::Utc::now() - last).num_days();
                if days_ago == 0 {
                    format!("🎉 ¡Bienvenido de vuelta, {}! 👋", name)
                } else if days_ago == 1 {
                    format!("🎉 ¡Hola {}, te extrañamos! Última vez: ayer 📅", name)
                } else {
                    format!("🎉 ¡Hola {}, te extrañamos! Última vez: hace {} días 📅", name, days_ago)
                }
            }
            None => {
                format!("🎉 ¡Bienvenido {}, es tu primer login! 🚀", name)
            }
        }
    }
    
    pub fn get_account_status_message(is_active: bool, last_login: Option<chrono::DateTime<chrono::Utc>>) -> String {
        if !is_active {
            return "❌ Cuenta inactiva".to_string();
        }
        
        match last_login {
            Some(last) => {
                let days_ago = (chrono::Utc::now() - last).num_days();
                if days_ago == 0 {
                    "✅ Activo - Conectado hoy".to_string()
                } else if days_ago <= 7 {
                    format!("✅ Activo - Última conexión: hace {} días", days_ago)
                } else {
                    format!("⚠️ Activo - Inactivo por {} días", days_ago)
                }
            }
            None => "✅ Activo - Primer login".to_string()
        }
    }
    
    pub fn generate_error_message(error_type: &str) -> String {
        match error_type {
            "invalid_credentials" => "❌ Email o contraseña incorrectos. Verifica tus datos e intenta nuevamente.".to_string(),
            "user_not_found" => "❌ No existe una cuenta con este email. ¿Necesitas registrarte?".to_string(),
            "account_inactive" => "❌ Tu cuenta está inactiva. Contacta soporte para reactivarla.".to_string(),
            "invalid_email" => "❌ Por favor, proporciona un email válido.".to_string(),
            "missing_password" => "❌ La contraseña es requerida.".to_string(),
            "invalid_user_id" => "❌ ID de usuario inválido.".to_string(),
            "session_expired" => "❌ Tu sesión ha expirado. Por favor, inicia sesión nuevamente.".to_string(),
            "database_error" => "❌ Error interno del servidor. Por favor, intenta más tarde.".to_string(),
            _ => "❌ Error desconocido durante la autenticación.".to_string(),
        }
    }
}

/// Cache invalidation patterns for auth operations
pub struct AuthCachePatterns;

impl AuthCachePatterns {
    pub fn invalidate_patterns(user_id: i64, email: &str) -> Vec<String> {
        vec![
            format!("auth_*_{}", user_id),
            format!("user_*_{}", user_id),
            format!("auth_*_{}", email),
            format!("session_*_{}", user_id),
            "auth_*".to_string(),
        ]
    }
}

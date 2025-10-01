use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{error, info, warn};
use validator::Validate;

use crate::api::auth::hash_password;
use crate::api::models::{
    EmailCheckRequest, EmailCheckResponse, MessageResponse, RegistrationResponse,
    SendVerificationRequest, SendVerificationResponse, SetPasswordRequest, UserProfile,
    UserRegistrationRequest, VerifyAccountRequest, VerifyAccountResponse, ResetPasswordRequest,
};
use crate::state::AppState;

/// Check email availability endpoint
pub async fn check_email_availability(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EmailCheckRequest>,
) -> Result<Json<EmailCheckResponse>, StatusCode> {
    // Validate input
    if let Err(_) = payload.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let email = payload.email.to_lowercase().trim().to_string();

    // Check if email exists in database
    let exists_result = sqlx::query!(
        "SELECT id FROM public.dim_users WHERE email = $1",
        email
    )
    .fetch_optional(&state.db_pool)
    .await;

    match exists_result {
        Ok(Some(_)) => Ok(Json(EmailCheckResponse {
            exists: true,
            message: "El email ya está registrado en el sistema".to_string(),
        })),
        Ok(None) => Ok(Json(EmailCheckResponse {
            exists: false,
            message: "El email está disponible para registro".to_string(),
        })),
        Err(e) => {
            error!("Database error checking email availability: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Register new user endpoint
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserRegistrationRequest>,
) -> Result<Json<RegistrationResponse>, StatusCode> {
    // Validate input
    if let Err(_) = payload.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let email = payload.email.to_lowercase().trim().to_string();
    let name = payload.name.trim().to_string();

    // Validate password strength
    if !is_password_strong(&payload.password) {
        return Ok(Json(RegistrationResponse {
            success: false,
            message: "La contraseña debe tener al menos 8 caracteres, incluir mayúsculas, minúsculas y números".to_string(),
            user_id: 0,
        }));
    }

    // Check if user already exists
    let existing_user = sqlx::query!(
        "SELECT id FROM public.dim_users WHERE email = $1",
        email
    )
    .fetch_optional(&state.db_pool)
    .await;

    match existing_user {
        Ok(Some(_)) => Ok(Json(RegistrationResponse {
            success: false,
            message: "El usuario ya existe en el sistema".to_string(),
            user_id: 0,
        })),
        Ok(None) => {
            // Hash password
            let password_hash = match hash_password(&payload.password) {
                Ok(hash) => hash,
                Err(e) => {
                    error!("Error hashing password: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            };

            // Insert new user
            let insert_result = sqlx::query!(
                r#"
                INSERT INTO public.dim_users (email, password_hash, name)
                VALUES ($1, $2, $3)
                RETURNING id
                "#,
                email,
                password_hash,
                name
            )
            .fetch_one(&state.db_pool)
            .await;

            match insert_result {
                Ok(user) => {
                    info!("New user registered: {} (ID: {})", email, user.id);
                    Ok(Json(RegistrationResponse {
                        success: true,
                        message: "Usuario registrado exitosamente".to_string(),
                        user_id: user.id,
                    }))
                }
                Err(e) => {
                    error!("Database error registering user: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            error!("Database error checking existing user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get user profile endpoint (requires authentication)
pub async fn get_user_profile(
    State(_state): State<Arc<AppState>>,
    // TODO: Add JWT authentication middleware
) -> Result<Json<UserProfile>, StatusCode> {
    // For now, return a placeholder response
    // In a real implementation, we'd extract user ID from JWT token
    warn!("get_user_profile endpoint called but JWT authentication not implemented yet");
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Send verification code endpoint
pub async fn send_verification_code(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SendVerificationRequest>,
) -> Result<Json<SendVerificationResponse>, StatusCode> {
    let email = payload.email.to_lowercase().trim().to_string();

    // Check if user exists
    let user_exists = sqlx::query!(
        "SELECT id FROM public.dim_users WHERE email = $1",
        email
    )
    .fetch_optional(&state.db_pool)
    .await;

    match user_exists {
        Ok(Some(_)) => {
            // TODO: Implement actual email sending logic
            // For now, return success response
            info!("Verification code requested for user: {}", email);
            Ok(Json(SendVerificationResponse {
                success: true,
                message: "Código de verificación enviado por email".to_string(),
                method: "email".to_string(),
            }))
        }
        Ok(None) => Ok(Json(SendVerificationResponse {
            success: false,
            message: "Usuario no encontrado en el sistema".to_string(),
            method: "email".to_string(),
        })),
        Err(e) => {
            error!("Database error sending verification code: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Verify account endpoint
pub async fn verify_account(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<VerifyAccountRequest>,
) -> Result<Json<VerifyAccountResponse>, StatusCode> {
    let email = payload.email.to_lowercase().trim().to_string();

    // TODO: Implement actual verification code validation
    // For now, accept any 6-digit code
    if payload.verification_code.len() == 6 && payload.verification_code.chars().all(|c| c.is_ascii_digit()) {
        info!("Account verified for user: {}", email);
        Ok(Json(VerifyAccountResponse {
            success: true,
            message: "Cuenta verificada exitosamente".to_string(),
        }))
    } else {
        Ok(Json(VerifyAccountResponse {
            success: false,
            message: "Código de verificación inválido".to_string(),
        }))
    }
}

/// Set user password endpoint
pub async fn set_user_password(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetPasswordRequest>,
) -> Result<Json<MessageResponse>, StatusCode> {
    // Validate input
    if let Err(_) = payload.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let email = payload.email.to_lowercase().trim().to_string();

    // Validate password strength
    if !is_password_strong(&payload.new_password) {
        return Ok(Json(MessageResponse {
            message: "La contraseña debe tener al menos 8 caracteres, incluir mayúsculas, minúsculas y números".to_string(),
        }));
    }

    // TODO: Validate verification code
    // For now, accept any 6-digit code
    if payload.verification_code.len() != 6 || !payload.verification_code.chars().all(|c| c.is_ascii_digit()) {
        return Ok(Json(MessageResponse {
            message: "Código de verificación inválido".to_string(),
        }));
    }

    // Hash new password
    let password_hash = match hash_password(&payload.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Error hashing password: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Update password in database
    let update_result = sqlx::query!(
        "UPDATE public.dim_users SET password_hash = $1 WHERE email = $2",
        password_hash,
        email
    )
    .execute(&state.db_pool)
    .await;

    match update_result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                info!("Password updated for user: {}", email);
                Ok(Json(MessageResponse {
                    message: "Contraseña actualizada exitosamente".to_string(),
                }))
            } else {
                Ok(Json(MessageResponse {
                    message: "Usuario no encontrado".to_string(),
                }))
            }
        }
        Err(e) => {
            error!("Database error updating password: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Reset user password endpoint
pub async fn reset_user_password(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<MessageResponse>, StatusCode> {
    // Validate input
    if let Err(_) = payload.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let email = payload.email.to_lowercase().trim().to_string();

    // Validate password strength
    if !is_password_strong(&payload.new_password) {
        return Ok(Json(MessageResponse {
            message: "La contraseña debe tener al menos 8 caracteres, incluir mayúsculas, minúsculas y números".to_string(),
        }));
    }

    // TODO: Validate verification code
    // For now, accept any 6-digit code
    if payload.verification_code.len() != 6 || !payload.verification_code.chars().all(|c| c.is_ascii_digit()) {
        return Ok(Json(MessageResponse {
            message: "Código de verificación inválido".to_string(),
        }));
    }

    // Hash new password
    let password_hash = match hash_password(&payload.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Error hashing password: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Update password in database
    let update_result = sqlx::query!(
        "UPDATE public.dim_users SET password_hash = $1 WHERE email = $2",
        password_hash,
        email
    )
    .execute(&_state.db_pool)
    .await;

    match update_result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                info!("Password reset for user: {}", email);
                Ok(Json(MessageResponse {
                    message: "Contraseña restablecida exitosamente".to_string(),
                }))
            } else {
                Ok(Json(MessageResponse {
                    message: "Usuario no encontrado".to_string(),
                }))
            }
        }
        Err(e) => {
            error!("Database error resetting password: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Validate password strength
fn is_password_strong(password: &str) -> bool {
    password.len() >= 8
        && password.chars().any(|c| c.is_uppercase())
        && password.chars().any(|c| c.is_lowercase())
        && password.chars().any(|c| c.is_ascii_digit())
}

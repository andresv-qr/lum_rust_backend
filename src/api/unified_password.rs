use axum::{
    extract::{Json, State},
    response::Json as ResponseJson,
    routing::post,
    Router,
};
use std::sync::Arc;
use std::time::Instant;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use bcrypt::{hash, DEFAULT_COST};
use tracing::{info, error, warn, debug};
use uuid::Uuid;

use crate::api::common::{ApiResponse, ApiError};
use crate::api::verification_v4::send_email_verification;
use crate::state::AppState;

fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// Log authentication/verification event to audit table
async fn log_verification_event(
    db_pool: &sqlx::PgPool,
    user_id: Option<i32>,
    event_type: &str,
    success: bool,
    error_code: Option<&str>,
    error_message: Option<&str>,
    request_id: &str,
    purpose: Option<&str>,
) {
    let metadata = serde_json::json!({
        "request_id": request_id,
        "timestamp": chrono::Utc::now(),
        "purpose": purpose,
        "verification_type": "unified_password_system"
    });

    let result = sqlx::query!(
        r#"SELECT public.log_auth_event(
            $1, $2, $3, NULL, NULL, $4, $5, $6, $7, NULL, $8
        )"#,
        user_id,
        event_type,
        "email_verification", // provider
        success,
        error_code,
        error_message,
        metadata,
        request_id
    )
    .fetch_optional(db_pool)
    .await;

    if let Err(e) = result {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Failed to log verification event to audit table"
        );
    } else {
        debug!(
            request_id = %request_id,
            event_type = %event_type,
            success = success,
            "✅ Verification event logged to audit table"
        );
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PasswordCodePurpose {
    #[serde(rename = "reset_password")]
    ResetPassword,
    #[serde(rename = "first_time_setup")]
    FirstTimeSetup,
    #[serde(rename = "change_password")]
    ChangePassword,
    #[serde(rename = "email_verification")]
    EmailVerification,
}

impl std::fmt::Display for PasswordCodePurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordCodePurpose::ResetPassword => write!(f, "reset_password"),
            PasswordCodePurpose::FirstTimeSetup => write!(f, "first_time_setup"),
            PasswordCodePurpose::ChangePassword => write!(f, "change_password"),
            PasswordCodePurpose::EmailVerification => write!(f, "email_verification"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RequestPasswordCodeRequest {
    pub email: String,
    pub purpose: PasswordCodePurpose,
}

#[derive(Debug, Serialize)]
pub struct RequestPasswordCodeResponse {
    pub email: String,
    pub code_expires_at: DateTime<Utc>,
    pub purpose: PasswordCodePurpose,
    pub instructions: String,
}

#[derive(Debug, Deserialize)]
pub struct SetPasswordWithCodeRequest {
    pub email: String,
    pub verification_code: String,
    pub new_password: String,
    pub confirmation_password: String,
}

#[derive(Debug, Serialize)]
pub struct SetPasswordWithCodeResponse {
    pub user_id: i32,
    pub email: String,
    pub password_updated_at: DateTime<Utc>,
    pub login_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailOnlyRequest {
    pub email: String,
    pub verification_code: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailOnlyResponse {
    pub user_id: i32,
    pub email: String,
    pub verified: bool,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SetPasswordWithEmailCodeRequest {
    pub email: String,
    pub verification_code: String,
    pub new_password: String,
    pub confirmation_password: String,
}

#[derive(Debug, Serialize)]
pub struct SetPasswordWithEmailCodeResponse {
    pub user_id: i32,
    pub email: String,
    pub email_verified: bool,
    pub password_set: bool,
    pub password_updated_at: DateTime<Utc>,
    pub login_token: Option<String>,
}

// Validación de contraseña
pub fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 {
        return Err(ApiError::bad_request("Password must be at least 8 characters long"));
    }
    if password.len() > 128 {
        return Err(ApiError::bad_request("Password must be less than 128 characters"));
    }
    
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));
    
    if !has_upper || !has_lower || !has_digit || !has_special {
        return Err(ApiError::bad_request(
            "Password must contain at least one uppercase letter, one lowercase letter, one number, and one special character"
        ));
    }
    
    Ok(())
}

// Generar código de verificación
pub fn generate_verification_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(100000..999999))
}

// Request password code endpoint
pub async fn request_password_code(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RequestPasswordCodeRequest>,
) -> Result<ResponseJson<ApiResponse<RequestPasswordCodeResponse>>, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        email = %payload.email,
        purpose = %payload.purpose,
        "🔐 Processing password code request"
    );
    
    // Validar formato de email
    if !payload.email.contains('@') || payload.email.len() < 5 {
        return Err(ApiError::bad_request("Invalid email format"));
    }
    
    // Verificar que el usuario existe
    let user = sqlx::query!(
        "SELECT id, email, password_hash FROM public.dim_users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while checking user"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    let user = user.ok_or_else(|| {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ User not found for password code request"
        );
        ApiError::not_found("User not found")
    })?;
    
    // Validar purpose según el estado del usuario
    match payload.purpose {
        PasswordCodePurpose::FirstTimeSetup => {
            if user.password_hash.is_some() {
                return Err(ApiError::bad_request("User already has a password set"));
            }
        }
        PasswordCodePurpose::ResetPassword => {
            if user.password_hash.is_none() {
                return Err(ApiError::bad_request("User has no password to reset. Use first_time_setup instead"));
            }
        }
        PasswordCodePurpose::ChangePassword => {
            if user.password_hash.is_none() {
                return Err(ApiError::bad_request("User has no password to change. Use first_time_setup instead"));
            }
        }
        PasswordCodePurpose::EmailVerification => {
            // Email verification is allowed for any user, regardless of password status
            info!(
                request_id = %request_id,
                email = %payload.email,
                "📧 Email verification code requested"
            );
        }
    }
    
    // Rate limiting - máximo 3 códigos por hora por email
    let recent_codes = sqlx::query!(
        "SELECT COUNT(*) as count FROM password_verification_codes 
         WHERE email = $1 AND created_at > NOW() - INTERVAL '1 hour'",
        payload.email
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while checking rate limit"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    if recent_codes.count.unwrap_or(0) >= 3 {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ Rate limit exceeded for password code requests"
        );
        return Err(ApiError::too_many_requests("Too many verification codes requested. Try again in 1 hour"));
    }
    
    // Invalidar códigos anteriores del mismo tipo
    sqlx::query!(
        "UPDATE password_verification_codes 
         SET used_at = NOW() 
         WHERE email = $1 AND purpose = $2 AND used_at IS NULL",
        payload.email,
        payload.purpose.to_string()
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while invalidating old codes"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Generar nuevo código
    let code = generate_verification_code();
    let expires_at = Utc::now() + Duration::minutes(15); // 15 minutos de validez
    
    // Guardar en base de datos
    sqlx::query!(
        "INSERT INTO password_verification_codes (user_id, email, code, purpose, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
        user.id as i32,
        payload.email,
        code,
        payload.purpose.to_string(),
        expires_at
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while saving verification code"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Enviar email de verificación
    if let Err(e) = send_email_verification(&payload.email, &code, &request_id).await {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Failed to send verification email, but code was saved successfully"
        );
        // No retornamos error porque el código ya se guardó correctamente
        // El usuario puede intentar de nuevo o usar el código existente
    } else {
        info!(
            request_id = %request_id,
            email = %payload.email,
            "✅ Verification email sent successfully"
        );
    }
    
    let instructions = match payload.purpose {
        PasswordCodePurpose::ResetPassword => "Use este código para restablecer tu contraseña. El código expira en 15 minutos.",
        PasswordCodePurpose::FirstTimeSetup => "Use este código para establecer tu primera contraseña. El código expira en 15 minutos.",
        PasswordCodePurpose::ChangePassword => "Use este código para cambiar tu contraseña. El código expira en 15 minutos.",
        PasswordCodePurpose::EmailVerification => "Use este código para verificar su dirección de email. El código expira en 15 minutos.",
    };
    
    // Log audit event
    log_verification_event(
        &state.db_pool,
        Some(user.id as i32),
        "password_reset",
        true,
        None,
        None,
        &request_id,
        Some("password_reset"),
    ).await;
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        processing_time_ms = processing_time,
        "✅ Password code request processed successfully"
    );
    
    let response = RequestPasswordCodeResponse {
        email: payload.email,
        code_expires_at: expires_at,
        purpose: payload.purpose,
        instructions: instructions.to_string(),
    };
    
    Ok(ResponseJson(ApiResponse::success(
        response,
        request_id,
        Some(processing_time),
        false,
    )))
}

// Set password with code endpoint
pub async fn set_password_with_code(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetPasswordWithCodeRequest>,
) -> Result<ResponseJson<ApiResponse<SetPasswordWithCodeResponse>>, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        email = %payload.email,
        "🔐 Processing set password with code request"
    );
    
    // Validar que las contraseñas coinciden
    if payload.new_password != payload.confirmation_password {
        return Err(ApiError::bad_request("Passwords do not match"));
    }
    
    // Validar fortaleza de contraseña
    validate_password(&payload.new_password)?;
    
    // Buscar código válido
    let verification = sqlx::query!(
        "SELECT id, user_id, purpose, expires_at, used_at, attempts, max_attempts
         FROM password_verification_codes 
         WHERE email = $1 AND code = $2 AND used_at IS NULL
         ORDER BY created_at DESC
         LIMIT 1",
        payload.email,
        payload.verification_code
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while checking verification code"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    let verification = verification.ok_or_else(|| {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ Invalid or expired verification code"
        );
        ApiError::bad_request("Invalid or expired verification code")
    })?;
    
    // Verificar que no está expirado
    if verification.expires_at < Utc::now() {
        warn!(
            request_id = %request_id,
            expires_at = %verification.expires_at,
            "⚠️ Verification code has expired"
        );
        return Err(ApiError::bad_request("Verification code has expired"));
    }
    
    // Verificar intentos
    if verification.attempts >= verification.max_attempts {
        warn!(
            request_id = %request_id,
            attempts = verification.attempts,
            max_attempts = verification.max_attempts,
            "⚠️ Too many attempts for verification code"
        );
        return Err(ApiError::bad_request("Too many attempts. Request a new code"));
    }
    
    // Incrementar intentos
    sqlx::query!(
        "UPDATE password_verification_codes SET attempts = attempts + 1 WHERE id = $1",
        verification.id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while updating attempts"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Hash de la nueva contraseña
    let password_hash = hash(&payload.new_password, DEFAULT_COST)
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to hash password"
            );
            ApiError::internal_server_error("Failed to hash password")
        })?;
    
    // Actualizar contraseña del usuario
    let updated_user = sqlx::query!(
        "UPDATE public.dim_users SET password_hash = $1, updated_at = NOW() 
         WHERE id = $2 
         RETURNING id, email, updated_at",
        password_hash,
        verification.user_id as i64
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while updating user password"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Marcar código como usado
    sqlx::query!(
        "UPDATE password_verification_codes SET used_at = NOW() WHERE id = $1",
        verification.id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while marking code as used"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Generar JWT token para login automático
    let login_token = match crate::utils::create_jwt_token(updated_user.id, &payload.email) {
        Ok(token) => Some(token),
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to generate login token"
            );
            None
        }
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        user_id = updated_user.id,
        processing_time_ms = processing_time,
        "✅ Password set successfully with code"
    );
    
    let response = SetPasswordWithCodeResponse {
        user_id: updated_user.id as i32,
        email: payload.email,
        password_updated_at: updated_user.updated_at.unwrap_or(Utc::now()),
        login_token,
    };
    
    Ok(ResponseJson(ApiResponse::success(
        response,
        request_id,
        Some(processing_time),
        false,
    )))
}

/// Verificar email sin establecer contraseña
/// Utiliza códigos generados con purpose="email_verification"
pub async fn verify_email_only(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyEmailOnlyRequest>,
) -> Result<Json<VerifyEmailOnlyResponse>, ApiError> {
    let start_time = Instant::now();
    let request_id = generate_request_id();
    
    info!(
        request_id = %request_id,
        email = %payload.email,
        "📧 Processing email verification request"
    );
    
    // Buscar código válido con purpose="email_verification"
    let verification = sqlx::query!(
        "SELECT id, user_id, purpose, expires_at, used_at, attempts, max_attempts
         FROM password_verification_codes 
         WHERE email = $1 AND code = $2 AND purpose = 'email_verification' AND used_at IS NULL
         ORDER BY created_at DESC
         LIMIT 1",
        payload.email,
        payload.verification_code
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while fetching verification code"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    let verification = verification.ok_or_else(|| {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ Invalid or expired email verification code"
        );
        
        // Log audit event for failed verification
        tokio::spawn({
            let db_pool = state.db_pool.clone();
            let request_id = request_id.clone();
            async move {
                log_verification_event(
                    &db_pool,
                    None,
                    "email_verification",
                    false,
                    Some("invalid_code"),
                    Some("Invalid or expired verification code"),
                    &request_id,
                    Some("email_verification"),
                ).await;
            }
        });
        
        ApiError::bad_request("Invalid or expired email verification code")
    })?;
    
    // Verificar expiración
    if verification.expires_at < Utc::now() {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ Verification code expired"
        );
        return Err(ApiError::bad_request("Verification code expired"));
    }
    
    // Verificar intentos máximos
    if verification.attempts >= verification.max_attempts {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            attempts = verification.attempts,
            max_attempts = verification.max_attempts,
            "⚠️ Maximum attempts exceeded"
        );
        return Err(ApiError::bad_request("Maximum verification attempts exceeded"));
    }
    
    // Marcar código como usado
    sqlx::query!(
        "UPDATE password_verification_codes SET used_at = NOW() WHERE id = $1",
        verification.id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while marking code as used"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Log audit event
    log_verification_event(
        &state.db_pool,
        Some(verification.user_id),
        "email_verification",
        true,
        None,
        None,
        &request_id,
        Some("email_verification"),
    ).await;
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        user_id = verification.user_id,
        processing_time_ms = processing_time,
        "✅ Email verified successfully"
    );
    
    let response = VerifyEmailOnlyResponse {
        user_id: verification.user_id,
        email: payload.email,
        verified: true,
        verified_at: Utc::now(),
    };
    
    Ok(Json(response))
}

/// Establecer contraseña usando código de verificación de email
/// Utiliza códigos generados con send-verification (purpose="email_verification")
/// Esto permite el flujo: send-verification → set-password-with-email-code
pub async fn set_password_with_email_code(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetPasswordWithEmailCodeRequest>,
) -> Result<Json<SetPasswordWithEmailCodeResponse>, ApiError> {
    let start_time = Instant::now();
    let request_id = generate_request_id();
    
    info!(
        request_id = %request_id,
        email = %payload.email,
        "🔐 Processing set password with email code request"
    );
    
    // Validar que las contraseñas coincidan
    if payload.new_password != payload.confirmation_password {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ Password confirmation mismatch"
        );
        return Err(ApiError::bad_request("Password confirmation does not match"));
    }
    
    // Validar contraseña
    validate_password(&payload.new_password)?;
    
    // Buscar código válido con purpose="email_verification"
    let verification = sqlx::query!(
        "SELECT id, user_id, purpose, expires_at, used_at, attempts, max_attempts
         FROM password_verification_codes 
         WHERE email = $1 AND code = $2 AND purpose = 'email_verification' AND used_at IS NULL
         ORDER BY created_at DESC
         LIMIT 1",
        payload.email,
        payload.verification_code
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while fetching verification code"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    let verification = verification.ok_or_else(|| {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ Invalid or expired email verification code"
        );
        ApiError::bad_request("Invalid or expired email verification code")
    })?;
    
    // Verificar expiración
    if verification.expires_at < Utc::now() {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ Email verification code expired"
        );
        return Err(ApiError::bad_request("Email verification code expired"));
    }
    
    // Verificar intentos máximos
    if verification.attempts >= verification.max_attempts {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            attempts = verification.attempts,
            max_attempts = verification.max_attempts,
            "⚠️ Maximum attempts exceeded for email verification code"
        );
        return Err(ApiError::bad_request("Maximum verification attempts exceeded"));
    }
    
    // Verificar que el usuario no tenga contraseña ya establecida
    let user = sqlx::query!(
        "SELECT id, email, password_hash FROM public.dim_users WHERE LOWER(email) = LOWER($1)",
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while fetching user"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    let user = user.ok_or_else(|| {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            "⚠️ User not found"
        );
        ApiError::not_found("User not found")
    })?;
    
    // Verificar que no tenga contraseña ya establecida
    if user.password_hash.is_some() {
        warn!(
            request_id = %request_id,
            email = %payload.email,
            user_id = user.id,
            "⚠️ User already has password set"
        );
        return Err(ApiError::bad_request("User already has a password set. Use reset password flow instead"));
    }
    
    // Hash de la nueva contraseña
    let password_hash = hash(&payload.new_password, DEFAULT_COST)
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to hash password"
            );
            ApiError::internal_server_error("Failed to hash password")
        })?;
    
    // Actualizar contraseña del usuario
    let updated_user = sqlx::query!(
        "UPDATE public.dim_users SET password_hash = $1, updated_at = NOW() 
         WHERE id = $2 
         RETURNING id, email, updated_at",
        password_hash,
        user.id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while updating user password"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Marcar código de verificación como usado
    sqlx::query!(
        "UPDATE password_verification_codes SET used_at = NOW() WHERE id = $1",
        verification.id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(
            request_id = %request_id,
            error = %e,
            "❌ Database error while marking verification code as used"
        );
        ApiError::internal_server_error("Database error")
    })?;
    
    // Generar JWT token para login automático
    let login_token = match crate::utils::create_jwt_token(updated_user.id, &payload.email) {
        Ok(token) => Some(token),
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to generate login token"
            );
            None
        }
    };
    
    // Log audit event
    log_verification_event(
        &state.db_pool,
        Some(updated_user.id as i32),
        "password_change",
        true,
        None,
        None,
        &request_id,
        Some("email_verification"),
    ).await;
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        user_id = updated_user.id,
        processing_time_ms = processing_time,
        "✅ Password set successfully with email verification code"
    );
    
    let response = SetPasswordWithEmailCodeResponse {
        user_id: updated_user.id as i32,
        email: payload.email,
        email_verified: true,
        password_set: true,
        password_updated_at: updated_user.updated_at.unwrap_or(Utc::now()),
        login_token,
    };
    
    Ok(Json(response))
}

// ============================================================================
// ROUTER FUNCTIONS
// ============================================================================

/// Router for public unified verification endpoints
/// Sistema unificado para verificación de email y gestión de contraseñas
pub fn create_unified_password_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/passwords/request-code", post(request_password_code))
        .route("/api/v4/passwords/set-with-code", post(set_password_with_code))
}

/// Wrapper para compatibilidad con send-verification
/// Redirige a request-code con purpose="email_verification"
pub async fn send_verification_unified(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let email = payload.get("email")
        .and_then(|e| e.as_str())
        .ok_or_else(|| ApiError::bad_request("Email is required"))?;
    
    let request = RequestPasswordCodeRequest {
        email: email.to_string(),
        purpose: PasswordCodePurpose::EmailVerification,
    };
    
    // Llamar a la función unificada
    match request_password_code(State(state), Json(request)).await {
        Ok(_response) => {
            // Convertir response para compatibilidad
            let compatible_response = serde_json::json!({
                "success": true,
                "message": "Verification code sent successfully",
                "method": "email"
            });
            Ok(Json(compatible_response))
        }
        Err(e) => Err(e),
    }
}

/// Router for unified verification endpoints (replaces old verification_v4)
/// Compatible con endpoints existentes pero usando sistema PostgreSQL unificado
pub fn create_unified_verification_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/send-verification", post(send_verification_unified))
        .route("/verify-account", post(verify_email_only))
        .route("/set-password-with-email-code", post(set_password_with_email_code))
}
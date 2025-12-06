use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;
use tracing::{error, info};
use validator::Validate;

use crate::api::models::{
    Claims, TokenResponse, UserLoginRequest, UserStatusRequest, UserStatusResponse,
};
use crate::state::AppState;

/// JWT secret key - should be loaded from environment
const JWT_SECRET: &str = "your-super-secret-jwt-key-change-in-production";
const JWT_EXPIRATION_HOURS: i64 = 24 * 90;  // 90 days

/// Login endpoint - authenticates user and returns JWT token
pub async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserLoginRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    // Validate input
    if let Err(_) = payload.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let email = payload.email.to_lowercase().trim().to_string();

    // Query user from database
    let user_result = sqlx::query!(
        r#"
        SELECT id, email, password_hash, name
        FROM public.dim_users 
        WHERE email = $1 AND password_hash IS NOT NULL
        "#,
        email
    )
    .fetch_optional(&state.db_pool)
    .await;

    let user = match user_result {
        Ok(Some(user)) => user,
        Ok(None) => {
            info!("Login attempt for non-existent user: {}", email);
            return Err(StatusCode::UNAUTHORIZED);
        }
        Err(e) => {
            error!("Database error during login: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Verify password
    let password_hash = user.password_hash.unwrap_or_default();
    if !verify(&payload.password, &password_hash).unwrap_or(false) {
        info!("Invalid password for user: {}", email);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Note: Skipping last_login_date update as column may not exist
    // In production, this would be handled by a separate audit table

    // Create JWT token
    let now = Utc::now();
    let exp = now + Duration::hours(JWT_EXPIRATION_HOURS);

    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone().unwrap_or_default(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    ) {
        Ok(token) => token,
        Err(e) => {
            error!("Error creating JWT token: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!("Successful login for user: {} (ID: {})", email, user.id);

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".to_string(),
        expires_in: JWT_EXPIRATION_HOURS * 3600, // seconds
        user_id: user.id,
        email: user.email.unwrap_or_default(),
    }))
}

/// Check user status endpoint - checks if user exists and has password
pub async fn check_user_status(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UserStatusRequest>,
) -> Result<Json<UserStatusResponse>, StatusCode> {
    let email = payload.email.to_lowercase().trim().to_string();

    // Query user from database
    let user_result = sqlx::query!(
        r#"
        SELECT id, email, password_hash
        FROM public.dim_users 
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(&state.db_pool)
    .await;

    match user_result {
        Ok(Some(user)) => {
            let has_password = user.password_hash.is_some() && !user.password_hash.as_ref().unwrap().is_empty();
            
            Ok(Json(UserStatusResponse {
                exists: true,
                has_password,
                source: None, // Source column doesn't exist in current schema
                message: if has_password {
                    "Usuario existe y tiene contraseña configurada".to_string()
                } else {
                    "Usuario existe pero no tiene contraseña configurada".to_string()
                },
            }))
        }
        Ok(None) => Ok(Json(UserStatusResponse {
            exists: false,
            has_password: false,
            source: None,
            message: "Usuario no existe en el sistema".to_string(),
        })),
        Err(e) => {
            error!("Database error checking user status: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Hash password using bcrypt
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

/// Verify password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

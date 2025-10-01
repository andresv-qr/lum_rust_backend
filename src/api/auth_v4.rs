use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;
use validator::Validate;
use bcrypt::{hash, DEFAULT_COST};
use sqlx::Row;

use crate::{
    api::{
        common::{ApiResponse, ApiError},
        models::TokenResponse,
        templates::auth_templates::{
            AuthQueryTemplates, AuthHelpers, LoginRequest, StatusResponse,
            UserAuthData, StatusCheckRequest
        },
        templates::user_registration_templates::{
            UserRegistrationRequest,
            sanitize_user_input, is_valid_email,
            EMAIL_APP_SOURCE, JWT_EXPIRATION_HOURS
        },
        auth::verify_password, // Reuse existing password verification
    },
    state::AppState,
    utils::create_jwt_token,
};

/// POST /api/v4/auth/login - User login (returns TokenResponse for frontend compatibility)
pub async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        email = %req.email,
        remember_me = req.remember_me.unwrap_or(false),
        "🔐 Processing user login request"
    );

    // Step 1: Validate input data
    if let Err(validation_errors) = req.validate() {
        warn!(
            request_id = %request_id,
            errors = ?validation_errors,
            "❌ Login validation failed"
        );
        
        let error_message = validation_errors
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                format!("{}: {}", field, errors[0].message.as_ref().unwrap_or(&"Invalid value".into()))
            })
            .collect::<Vec<_>>()
            .join(", ");
            
        return Err(ApiError::new("VALIDATION_FAILED", &error_message));
    }

    // Step 2: Normalize email
    let email = AuthHelpers::normalize_email(&req.email);
    
    // Step 3: Get user from database
    let user_result = sqlx::query_as::<_, UserAuthData>(
        "SELECT id, email, password_hash, name, 
                COALESCE(created_at, NOW()) as created_at, 
                COALESCE(updated_at, NOW()) as updated_at, 
                true as is_active 
         FROM public.dim_users WHERE email = $1"
    )
    .bind(&email)
    .fetch_optional(&state.db_pool)
    .await;

    let user_data = match user_result {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!(
                request_id = %request_id,
                email = %email,
                "❌ User not found for login"
            );
            
            return Err(ApiError::new("AUTHENTICATION_FAILED", "User not found"));
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Database error during login"
            );
            
            return Err(ApiError::new("DATABASE_ERROR", "Failed to query user data"));
        }
    };

    // Step 4: Check if user has password set and verify
    let password_valid = match &user_data.password_hash {
        Some(stored_hash) => {
            // User has password, verify it
            match verify_password(&req.password, stored_hash) {
                Ok(valid) => valid,
                Err(e) => {
                    error!(
                        request_id = %request_id,
                        error = %e,
                        "❌ Error verifying password"
                    );
                    return Err(ApiError::new("AUTHENTICATION_ERROR", "Password verification failed"));
                }
            }
        }
        None => {
            // User doesn't have password set - should not use regular login
            warn!(
                request_id = %request_id,
                email = %email,
                "❌ User attempted login but has no password set - should use account setup flow"
            );
            return Err(ApiError::new(
                "ACCOUNT_SETUP_REQUIRED", 
                "This account requires password setup. Please use the account setup process."
            ));
        }
    };

    if !password_valid {
        warn!(
            request_id = %request_id,
            email = %email,
            "❌ Invalid password provided"
        );
        
        return Err(ApiError::new("AUTHENTICATION_FAILED", "Invalid credentials"));
    }

    // Step 5: Check if account is active
    if !user_data.is_active {
        warn!(
            request_id = %request_id,
            user_id = user_data.id,
            "❌ Inactive account attempted login"
        );
        
        return Err(ApiError::new("ACCOUNT_INACTIVE", "User account is not active"));
    }

    // Step 6: Generate JWT token and update last login
    let remember_me = req.remember_me.unwrap_or(false);
    let expires_in = if remember_me {
        7 * 24 * 3600 // 7 days for remember_me
    } else {
        24 * 3600 // 24 hours normal
    };

    // Generate JWT token
    let access_token = match create_jwt_token(user_data.id, &user_data.email) {
        Ok(token) => token,
        Err(e) => {
            error!(
                request_id = %request_id,
                user_id = user_data.id,
                error = %e,
                "❌ Failed to create JWT token"
            );
            return Err(ApiError::new("TOKEN_ERROR", "Failed to generate authentication token"));
        }
    };

    // Update last login timestamp
    let update_result = sqlx::query(AuthQueryTemplates::update_last_login_query())
        .bind(user_data.id)
        .execute(&state.db_pool)
        .await;

    if let Err(e) = update_result {
        warn!(
            request_id = %request_id,
            user_id = user_data.id,
            error = %e,
            "⚠️ Failed to update last login timestamp"
        );
    }

    // Step 7: Generate response
    let _welcome_message = AuthHelpers::generate_welcome_back_message(
        user_data.name.as_deref().unwrap_or("User"), 
        None
    );
    
    // Create TokenResponse for frontend compatibility (JWT format)
    let token_response = TokenResponse {
        access_token,
        token_type: "bearer".to_string(),
        expires_in,
        user_id: user_data.id,
        email: user_data.email.clone(),
    };

    let execution_time = start_time.elapsed();
    
    info!(
        request_id = %request_id,
        user_id = user_data.id,
        email = %user_data.email,
        execution_time_ms = execution_time.as_millis(),
        remember_me = remember_me,
        "🎉 User login successful - JWT token generated for frontend compatibility"
    );

    Ok(Json(token_response))
}

/// POST /api/v4/auth/register - User registration (returns TokenResponse for frontend compatibility)
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UserRegistrationRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        email = %req.email,
        "🔐 Processing user registration request"
    );

    // Step 1: Validate input data
    if let Err(validation_errors) = req.validate() {
        warn!(
            request_id = %request_id,
            errors = ?validation_errors,
            "❌ Registration validation failed"
        );
        
        let error_message = validation_errors
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                format!("{}: {}", field, errors[0].message.as_ref().unwrap_or(&"Invalid value".into()))
            })
            .collect::<Vec<_>>()
            .join(", ");
            
        return Err(ApiError::new("VALIDATION_FAILED", &error_message));
    }

    // Step 2: Sanitize inputs
    let email = sanitize_user_input(&req.email).to_lowercase();
    let name = sanitize_user_input(&req.name);
    let password = req.password;
    
    // Step 3: Additional email validation
    if !is_valid_email(&email) {
        warn!(
            request_id = %request_id,
            email = %email,
            "❌ Invalid email format"
        );
        return Err(ApiError::new("VALIDATION_FAILED", "Invalid email format"));
    }

    // Step 4: Check if user already exists
    let existing_user_result = sqlx::query(
        "SELECT id, email FROM public.dim_users WHERE email = $1"
    )
    .bind(&email)
    .fetch_optional(&state.db_pool)
    .await;

    match existing_user_result {
        Ok(Some(_)) => {
            warn!(
                request_id = %request_id,
                email = %email,
                "❌ Email already registered"
            );
            return Err(ApiError::new("EMAIL_EXISTS", "Email already registered"));
        }
        Ok(None) => {
            info!(
                request_id = %request_id,
                email = %email,
                "✅ Email is available for registration"
            );
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Database error checking user existence"
            );
            return Err(ApiError::new("DATABASE_ERROR", "Failed to check email availability"));
        }
    }

    // Step 5: Hash password
    let password_hash = match hash(&password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to hash password"
            );
            return Err(ApiError::new("INTERNAL_ERROR", "Password processing failed"));
        }
    };

    // Step 6: Create new user
    let user_insert_result = sqlx::query(
        "INSERT INTO public.dim_users (email, password_hash, name, source, user_id_val, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, NOW(), NOW()) 
         RETURNING id"
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(&name)
    .bind(EMAIL_APP_SOURCE)
    .bind(&email) // For email source, ID is the email itself
    .fetch_one(&state.db_pool)
    .await;

    let user_id = match user_insert_result {
        Ok(row) => {
            let id: i32 = row.get("id");
            info!(
                request_id = %request_id,
                user_id = id,
                email = %email,
                "✅ Successfully created user"
            );
            id
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Failed to create user"
            );
            return Err(ApiError::new("DATABASE_ERROR", "Failed to create user account"));
        }
    };

    // Step 7: Generate JWT token
    let expires_in = JWT_EXPIRATION_HOURS * 3600;
    let access_token = match create_jwt_token(user_id as i64, &email) {
        Ok(token) => token,
        Err(e) => {
            error!(
                request_id = %request_id,
                user_id = user_id,
                error = %e,
                "❌ Failed to create JWT token"
            );
            return Err(ApiError::new("TOKEN_ERROR", "Failed to generate authentication token"));
        }
    };

    // Step 8: Create response
    let token_response = TokenResponse {
        access_token,
        token_type: "bearer".to_string(),
        expires_in,
        user_id: user_id as i64,
        email: email.clone(),
    };

    let execution_time = start_time.elapsed();
    
    info!(
        request_id = %request_id,
        user_id = user_id,
        email = %email,
        execution_time_ms = execution_time.as_millis(),
        "🎉 User registration successful - TokenResponse format for frontend compatibility"
    );

    Ok(Json(token_response))
}

/// GET /api/v4/auth/status/:user_id - Check user status
pub async fn check_user_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(user_id): axum::extract::Path<i64>,
) -> Result<Json<ApiResponse<StatusResponse>>, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        user_id = user_id,
        "🔍 Processing user status check"
    );

    // Step 1: Validate user ID
    if user_id <= 0 {
        warn!(
            request_id = %request_id,
            user_id = user_id,
            "❌ Invalid user ID provided"
        );
        
        return Err(ApiError::new("VALIDATION_FAILED", "Invalid user ID"));
    }

    // Step 2: Get user from database
    let user_result = sqlx::query_as::<_, StatusResponse>(
        "SELECT id as user_id, email, name, 
                true as is_active, NOW() as created_at, NOW() as last_login_at, 
                true as session_valid, '' as account_status
         FROM public.dim_users 
         WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await;

    match user_result {
        Ok(Some(mut user_status)) => {
            // Generate account status message
            user_status.account_status = AuthHelpers::get_account_status_message(
                user_status.is_active, 
                user_status.last_login_at
            );
            
            let execution_time = start_time.elapsed();
            
            info!(
                request_id = %request_id,
                user_id = user_id,
                is_active = user_status.is_active,
                execution_time_ms = execution_time.as_millis(),
                "✅ User status check completed"
            );

            Ok(Json(ApiResponse {
                success: true,
                data: Some(user_status),
                error: None,
                request_id,
                timestamp: chrono::Utc::now(),
                execution_time_ms: Some(execution_time.as_millis() as u64),
                cached: false,
            }))
        }
        Ok(None) => {
            warn!(
                request_id = %request_id,
                user_id = user_id,
                "❌ User not found for status check"
            );
            
            Err(ApiError::new("USER_NOT_FOUND", "User not found"))
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                user_id = user_id,
                error = %e,
                "❌ Database error during status check"
            );
            
            Err(ApiError::new("DATABASE_ERROR", "Failed to query user data"))
        }
    }
}

/// POST /api/v4/auth/status - Check user status with request body
pub async fn check_user_status_post(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StatusCheckRequest>,
) -> Result<Json<ApiResponse<StatusResponse>>, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    
    info!(
        request_id = %request_id,
        user_id = payload.user_id,
        "🔍 Processing user status check (POST)"
    );

    // Validate input
    if let Err(validation_errors) = payload.validate() {
        warn!(
            request_id = %request_id,
            errors = ?validation_errors,
            "❌ Status check validation failed"
        );
        
        return Err(ApiError::new("VALIDATION_FAILED", &format!("Input validation errors: {:?}", validation_errors)));
    }

    // Reuse the GET endpoint logic
    check_user_status(State(state), axum::extract::Path(payload.user_id)).await
}

/// Create the auth V4 router (relative paths for nesting)
pub fn create_auth_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login_user))
        .route("/register", post(register_user))
        .route("/status/:user_id", get(check_user_status))
        .route("/status", post(check_user_status_post))
}

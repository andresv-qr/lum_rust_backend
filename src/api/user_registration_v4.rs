use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use validator::Validate;
use bcrypt::{hash, DEFAULT_COST};
use sqlx::Row;

use crate::state::AppState;
use crate::api::templates::user_registration_templates::{
    UserRegistrationRequest, UserRegistrationResponse, EmailCheckRequest, EmailCheckResponse,
    RegistrationErrorResponse, NewUser, ExistingUser, UserRegistrationQueries,
    sanitize_user_input, is_valid_email, format_validation_errors,
    EMAIL_APP_SOURCE, JWT_EXPIRATION_HOURS
};
use crate::utils::create_jwt_token;

// ============================================================================
// API HANDLERS
// ============================================================================

/// Register a new user with email, password, and name
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UserRegistrationRequest>,
) -> Result<Json<UserRegistrationResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!("Request {}: User registration attempt for email: {}", request_id, req.email);
    
    // Validate request data
    if let Err(validation_errors) = req.validate() {
        let formatted_errors = format_validation_errors(&validation_errors);
        warn!("Request {}: Validation failed for registration: {:?}", request_id, formatted_errors);
        
        return Ok(Json(UserRegistrationResponse::from(
            RegistrationErrorResponse::validation_error(formatted_errors)
        )));
    }
    
    // Sanitize inputs
    let email = sanitize_user_input(&req.email).to_lowercase();
    let name = sanitize_user_input(&req.name);
    let password = req.password;
    
    // Additional email validation
    if !is_valid_email(&email) {
        warn!("Request {}: Invalid email format: {}", request_id, email);
        return Ok(Json(UserRegistrationResponse::from(
            RegistrationErrorResponse::validation_error(
                [("email".to_string(), vec!["Invalid email format".to_string()])]
                    .iter().cloned().collect()
            )
        )));
    }
    
    // Check if user already exists
    match check_user_exists(&state.db_pool, &email, &request_id).await {
        Ok(Some(_existing_user)) => {
            warn!("Request {}: Email {} already registered", request_id, email);
            return Ok(Json(UserRegistrationResponse::from(
                RegistrationErrorResponse::email_exists(&email)
            )));
        }
        Ok(None) => {
            info!("Request {}: Email {} is available for registration", request_id, email);
        }
        Err(e) => {
            error!("Request {}: Database error checking user existence: {}", request_id, e);
            return Ok(Json(UserRegistrationResponse::from(
                RegistrationErrorResponse::database_error()
            )));
        }
    }
    
    // Hash password
    let password_hash = match hash(&password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Request {}: Failed to hash password: {}", request_id, e);
            return Ok(Json(UserRegistrationResponse::from(
                RegistrationErrorResponse::internal_error("Password processing failed")
            )));
        }
    };
    
    // Create new user
    let new_user = NewUser {
        email: email.clone(),
        password_hash,
        name: name.clone(),
        source: EMAIL_APP_SOURCE.to_string(),
        user_id_val: email.clone(), // For email source, ID is the email itself
    };
    
    // Insert user into database
    match create_user(&state.db_pool, &new_user, &request_id).await {
        Ok(user_id) => {
            info!("Request {}: Successfully created user with ID: {}", request_id, user_id);
            
            // Generate JWT token
            let expires_in = JWT_EXPIRATION_HOURS * 3600;
            match create_jwt_token(user_id as i64, &email) {
                Ok(access_token) => {
                    let processing_time = start_time.elapsed().as_millis();
                    info!("Request {}: User registration completed successfully in {}ms", 
                          request_id, processing_time);
                    
                    Ok(Json(UserRegistrationResponse::success(
                        access_token,
                        expires_in,
                        user_id,
                        email,
                        name,
                    )))
                }
                Err(e) => {
                    error!("Request {}: Failed to create JWT token: {}", request_id, e);
                    Ok(Json(UserRegistrationResponse::from(
                        RegistrationErrorResponse::internal_error("Token generation failed")
                    )))
                }
            }
        }
        Err(e) => {
            error!("Request {}: Failed to create user: {}", request_id, e);
            Ok(Json(UserRegistrationResponse::from(
                RegistrationErrorResponse::database_error()
            )))
        }
    }
}

/// Check if an email is already registered
pub async fn check_email_exists(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EmailCheckRequest>,
) -> Result<Json<EmailCheckResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    info!("Request {}: Checking email existence: {}", request_id, req.email);
    
    let email = sanitize_user_input(&req.email).to_lowercase();
    
    // Validate email format
    if !is_valid_email(&email) {
        warn!("Request {}: Invalid email format: {}", request_id, email);
        return Err(StatusCode::BAD_REQUEST);
    }
    
    match check_user_exists(&state.db_pool, &email, &request_id).await {
        Ok(Some(existing_user)) => {
            info!("Request {}: Email {} exists with password: {}", 
                  request_id, email, existing_user.has_password);
            
            Ok(Json(EmailCheckResponse::exists(
                &email,
                existing_user.has_password,
                existing_user.source,
            )))
        }
        Ok(None) => {
            info!("Request {}: Email {} is available", request_id, email);
            Ok(Json(EmailCheckResponse::not_exists(&email)))
        }
        Err(e) => {
            error!("Request {}: Database error checking email: {}", request_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============================================================================
// DATABASE OPERATIONS
// ============================================================================

/// Check if a user exists by email
async fn check_user_exists(
    pool: &sqlx::PgPool,
    email: &str,
    request_id: &str,
) -> Result<Option<ExistingUser>, sqlx::Error> {
    info!("Request {}: Querying database for email: {}", request_id, email);
    
    let row = sqlx::query(UserRegistrationQueries::CHECK_EMAIL_EXISTS)
        .bind(email)
        .fetch_optional(pool)
        .await?;
    
    if let Some(row) = row {
        let existing_user = ExistingUser {
            id: row.get("id"),
            email: row.get("email"),
            name: row.get("name"),
            has_password: row.get("has_password"),
            source: row.get("source"),
            created_at: row.get("created_at"),
        };
        
        info!("Request {}: Found existing user with ID: {}", request_id, existing_user.id);
        Ok(Some(existing_user))
    } else {
        info!("Request {}: No existing user found for email", request_id);
        Ok(None)
    }
}

/// Create a new user in the database
async fn create_user(
    pool: &sqlx::PgPool,
    new_user: &NewUser,
    request_id: &str,
) -> Result<i32, sqlx::Error> {
    info!("Request {}: Creating new user in database: {}", request_id, new_user.email);
    
    let row = sqlx::query(UserRegistrationQueries::INSERT_NEW_USER)
        .bind(&new_user.email)
        .bind(&new_user.password_hash)
        .bind(&new_user.name)
        .bind(&new_user.source)
        .bind(&new_user.user_id_val)
        .fetch_one(pool)
        .await?;
    
    let user_id: i32 = row.get("id");
    info!("Request {}: Successfully created user with ID: {}", request_id, user_id);
    
    Ok(user_id)
}

// ============================================================================
// RESPONSE CONVERSION
// ============================================================================

impl From<RegistrationErrorResponse> for UserRegistrationResponse {
    fn from(error: RegistrationErrorResponse) -> Self {
        Self {
            access_token: String::new(),
            token_type: "bearer".to_string(),
            expires_in: 0,
            user_id: 0,
            email: String::new(),
            name: String::new(),
            message: error.error,
        }
    }
}

// ============================================================================
// ROUTER CREATION
// ============================================================================

/// Router for public endpoints (no JWT required)
pub fn create_user_registration_v4_public_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/users/register", post(register_user))
        .route("/api/v4/users/check-email", post(check_email_exists))
}

/// Router for protected endpoints (JWT required) - currently empty
pub fn create_user_registration_v4_protected_router() -> Router<Arc<AppState>> {
    Router::new()
    // No protected endpoints in user registration yet
}

/// Legacy router - kept for backward compatibility but should be phased out
pub fn create_user_registration_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register_user))
        .route("/check-email", post(check_email_exists))
}

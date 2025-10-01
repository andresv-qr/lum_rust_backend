use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{
        common::ApiResponse,
        models::ErrorResponse,
        templates::register_templates::*,
    },
    state::AppState,
};

/// POST /api/v4/register - Register a new user
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<RegisterResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    info!(
        request_id = %request_id,
        email = %payload.email,
        name = %payload.name,
        "🎯 Processing user registration request"
    );

    // Step 1: Validate input data
    if let Err(validation_errors) = payload.validate() {
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
            
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Validation failed".to_string(),
                message: error_message,
                details: Some(format!("Request ID: {}", request_id)),
            }),
        ));
    }

    // Step 2: Normalize input data
    let email = RegistrationHelpers::normalize_email(&payload.email);
    let _name = RegistrationHelpers::normalize_name(&payload.name);
    
    // Step 3: Validate password strength
    if !PasswordValidator::is_strong(&payload.password) {
        warn!(
            request_id = %request_id,
            email = %email,
            "❌ Weak password provided"
        );
        
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Weak password".to_string(),
                message: RegistrationHelpers::generate_error_message("weak_password"),
                details: Some(format!("Request ID: {}", request_id)),
            }),
        ));
    }

    // Step 4: Check if email already exists
    let email_check_result = sqlx::query_as::<_, EmailExistsResponse>(
        "SELECT id, true as exists FROM public.dim_users WHERE email = $1"
    )
    .bind(&email)
    .fetch_optional(&state.db_pool)
    .await;

    match email_check_result {
        Ok(Some(_)) => {
            warn!(
                request_id = %request_id,
                email = %email,
                "❌ Email already exists in system"
            );
            
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "Email already exists".to_string(),
                    message: RegistrationHelpers::generate_error_message("email_exists"),
                    details: Some(format!("Request ID: {}", request_id)),
                }),
            ));
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
                "❌ Database error checking email existence"
            );
            
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                    message: RegistrationHelpers::generate_error_message("database_error"),
                    details: Some(format!("Request ID: {}", request_id)),
                }),
            ));
        }
    }

    // Step 5: Hash password (using existing hash_password function)
    let password_hash = match crate::api::auth::hash_password(&payload.password) {
        Ok(hash) => hash,
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "❌ Error hashing password"
            );
            
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Password hashing failed".to_string(),
                    message: RegistrationHelpers::generate_error_message("database_error"),
                    details: Some(format!("Request ID: {}", request_id)),
                }),
            ));
        }
    };

    // Step 6: Insert new user into database
    let insert_result = sqlx::query!(
        r#"
        INSERT INTO public.dim_users (name, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, name, email
        "#,
        payload.name,
        email,
        password_hash
    )
    .fetch_one(&state.db_pool)
    .await;

    match insert_result {
        Ok(user_record) => {
            let execution_time = start_time.elapsed();
            let user_name = user_record.name.as_deref().unwrap_or_default();

            info!(
                request_id = %request_id,
                user_id = user_record.id,
                email = %user_record.email.as_deref().unwrap_or_default(),
                execution_time_ms = execution_time.as_millis(),
                "🎉 User registered successfully"
            );

            let response_data = RegisterResponse {
                id: user_record.id,
                name: user_name.to_string(),
                email: user_record.email.unwrap_or_default(),
                welcome_message: RegistrationHelpers::generate_welcome_message(user_name),
                registration_success: true,
                created_at: chrono::Utc::now(), 
            };

            Ok(Json(ApiResponse {
                success: true,
                data: Some(response_data),
                error: None,
                request_id,
                timestamp: chrono::Utc::now(),
                execution_time_ms: Some(execution_time.as_millis() as u64),
                cached: false,
            }))
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                email = %email,
                error = %e,
                "❌ Database error inserting new user"
            );
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Database error".to_string(),
                    message: RegistrationHelpers::generate_error_message("database_error"),
                    details: Some(format!("Request ID: {}", request_id)),
                }),
            ))
        }
    }
}

/// Create the register V4 router
pub fn create_register_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(register_user))
}

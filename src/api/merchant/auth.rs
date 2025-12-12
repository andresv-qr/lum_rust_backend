// ============================================================================
// MERCHANT AUTHENTICATION - Login para comercios
// ============================================================================

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::state::AppState;

/// Request body for merchant login
#[derive(Debug, Deserialize)]
pub struct MerchantLoginRequest {
    pub merchant_name: String,
    pub api_key: String,
}

/// Response for successful login
#[derive(Debug, Serialize)]
pub struct MerchantLoginResponse {
    pub success: bool,
    pub token: String,
    pub merchant: MerchantInfo,
}

/// Merchant information
#[derive(Debug, Serialize)]
pub struct MerchantInfo {
    pub merchant_id: String,
    pub merchant_name: String,
    pub expires_in: i64,
}

/// Merchant login endpoint
/// 
/// # Endpoint
/// POST /api/v1/merchant/auth/login
/// 
/// # Request Body
/// ```json
/// {
///   "email": "merchant@starbucks.com",
///   "password": "secure_password"
/// }
/// ```
/// 
/// # Returns
/// - 200 OK: Login successful with JWT token
/// - 401 Unauthorized: Invalid credentials
/// - 500 Internal Server Error: Database error
pub async fn merchant_login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MerchantLoginRequest>,
) -> Result<Json<MerchantLoginResponse>, ApiError> {
    info!("Merchant login attempt for: {}", payload.merchant_name);
    
    // Query merchant from database (case insensitive)
    let merchant = sqlx::query!(
        r#"
        SELECT 
            merchant_id::text,
            merchant_name,
            api_key_hash,
            is_active
        FROM rewards.merchants
        WHERE LOWER(merchant_name) = LOWER($1)
        "#,
        payload.merchant_name
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Database error during merchant login: {}", e);
        ApiError::InternalError("Error al consultar comercio".to_string())
    })?
    .ok_or_else(|| {
        error!("Merchant not found: {}", payload.merchant_name);
        ApiError::Unauthorized("Credenciales inválidas".to_string())
    })?;
    
    // Check if merchant is active
    if !merchant.is_active.unwrap_or(false) {
        error!("Inactive merchant attempted login: {}", payload.merchant_name);
        return Err(ApiError::Unauthorized("Comercio inactivo".to_string()));
    }
    
    // Verify API key with bcrypt
    let is_valid = bcrypt::verify(&payload.api_key, &merchant.api_key_hash)
        .map_err(|e| {
            error!("Error verifying API key: {}", e);
            ApiError::InternalError("Error en verificación".to_string())
        })?;
    
    if !is_valid {
        error!("Invalid API key for merchant: {}", payload.merchant_name);
        return Err(ApiError::Unauthorized("Credenciales inválidas".to_string()));
    }
    
    // Generate JWT token for merchant
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    use chrono::{Utc, Duration};
    
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "lumis_jwt_secret_super_seguro_production_2024_rust_server_key".to_string());
    
    let exp = Utc::now() + Duration::hours(8); // 8 hours expiry for merchants
    
    let claims = json!({
        "sub": merchant.merchant_id,
        "merchant_name": merchant.merchant_name,
        "role": "merchant",
        "exp": exp.timestamp(),
        "iat": Utc::now().timestamp(),
    });
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| {
        error!("JWT encoding error: {}", e);
        ApiError::InternalError("Error al generar token".to_string())
    })?;
    
    info!("Merchant login successful: {} ({})", 
        merchant.merchant_name, 
        merchant.merchant_id.as_deref().unwrap_or("unknown")
    );
    
    Ok(Json(MerchantLoginResponse {
        success: true,
        token,
        merchant: MerchantInfo {
            merchant_id: merchant.merchant_id.unwrap_or_else(|| "unknown".to_string()),
            merchant_name: merchant.merchant_name,
            expires_in: 28800, // 8 hours in seconds
        },
    }))
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        
        let body = Json(serde_json::json!({
            "success": false,
            "error": message,
        }));
        
        (status, body).into_response()
    }
}

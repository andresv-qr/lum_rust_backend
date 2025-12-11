use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::LazyLock;
use tracing::{error, info, warn};

use crate::{
    api::models::ErrorResponse,
};

// ============================================================================
// STATIC ERROR MESSAGES - PERFORMANCE: Avoid allocations per request
// ============================================================================
mod error_messages {
    pub const ERR_MISSING_AUTH: &str = "Missing Authorization header";
    pub const MSG_AUTH_REQUIRED: &str = "Authentication required. Please provide a valid Bearer token.";
    pub const ERR_INVALID_AUTH_FORMAT: &str = "Invalid Authorization header format";
    pub const MSG_BEARER_REQUIRED: &str = "Authorization header must start with 'Bearer '.";
    pub const ERR_EMPTY_TOKEN: &str = "Empty JWT token";
    pub const MSG_PROVIDE_TOKEN: &str = "Please provide a valid JWT token.";
    pub const ERR_TOKEN_EXPIRED: &str = "Token expired";
    pub const MSG_SESSION_EXPIRED: &str = "Your session has expired. Please log in again.";
    pub const ERR_INVALID_TOKEN: &str = "Invalid token";
    pub const MSG_INVALID_CREDENTIALS: &str = "Could not validate credentials. Please log in again.";
}
use error_messages::*;

/// JWT Claims structure matching the token payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,     // Standard JWT subject field (user_id as string)
    pub email: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: Option<String>, // JWT ID for revocation
}

/// Merchant JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MerchantClaims {
    pub sub: String,     // merchant_id as string (UUID)
    pub merchant_name: String,
    pub role: String,    // Should be "merchant"
    pub exp: i64,
    pub iat: i64,
    #[serde(default)]
    pub merchant_id: Option<uuid::Uuid>,  // Optional for backward compatibility
}

impl MerchantClaims {
    /// Helper method to get merchant_id, parsing from sub if not present
    pub fn get_merchant_id(&self) -> Option<uuid::Uuid> {
        self.merchant_id.or_else(|| uuid::Uuid::parse_str(&self.sub).ok())
    }
}

impl JwtClaims {
    /// Helper method to parse user_id from sub field
    /// Returns i32 for compatibility with database schema
    pub fn user_id(&self) -> Result<i32, String> {
        self.sub.parse::<i32>()
            .map_err(|_| format!("Invalid user_id in token: '{}'", self.sub))
    }
    
    /// Helper method to get user_id as i64 if needed
    pub fn user_id_i64(&self) -> Result<i64, String> {
        self.sub.parse::<i64>()
            .map_err(|_| format!("Invalid user_id in token: '{}'", self.sub))
    }
}

/// Alias for compatibility
pub type Claims = JwtClaims;

/// Current user data extracted from JWT
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user_id: i64,   // Converted from sub
    pub email: String,
    pub token: String,
}

/// JWT configuration constants
pub const JWT_ALGORITHM: Algorithm = Algorithm::HS256;

/// JWT secret initialized lazily once (optimización: evita env::var en cada request)
/// SECURITY: Falla en startup si JWT_SECRET no está configurado - NO usar fallback
static JWT_SECRET: LazyLock<String> = LazyLock::new(|| {
    env::var("JWT_SECRET")
        .expect("CRITICAL: JWT_SECRET environment variable must be set. Server cannot start without a secure JWT secret.")
});

/// Get JWT secret (ahora retorna &'static str)
fn get_jwt_secret() -> &'static str {
    &JWT_SECRET
}

/// Helper to create ErrorResponse with static strings (avoids allocation)
#[inline]
fn static_error(error: &'static str, message: &'static str) -> ErrorResponse {
    ErrorResponse {
        error: error.to_string(),
        message: message.to_string(),
        details: None,
    }
}

/// Extract and validate JWT token from Authorization header
pub async fn extract_current_user(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Extract Authorization header
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                Json(static_error(ERR_MISSING_AUTH, MSG_AUTH_REQUIRED)),
            )
        })?;

    // Check Bearer prefix
    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid Authorization header format");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(static_error(ERR_INVALID_AUTH_FORMAT, MSG_BEARER_REQUIRED)),
        ));
    }

    // Extract token
    let token = auth_header.trim_start_matches("Bearer ").trim();
    if token.is_empty() {
        warn!("Empty JWT token");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(static_error(ERR_EMPTY_TOKEN, MSG_PROVIDE_TOKEN)),
        ));
    }

    // TODO: Check Redis cache for token validation (similar to Python implementation)
    // For now, we'll decode directly

    // Decode and validate JWT
    let jwt_secret = get_jwt_secret();
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(JWT_ALGORITHM);

    let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)
        .map_err(|e| {
            // Log detailed error internally for debugging
            error!("JWT validation failed: {}", e);
            // Return generic message to client (no internal details exposed)
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => (
                    StatusCode::UNAUTHORIZED,
                    Json(static_error(ERR_TOKEN_EXPIRED, MSG_SESSION_EXPIRED)),
                ),
                _ => (
                    StatusCode::UNAUTHORIZED,
                    Json(static_error(ERR_INVALID_TOKEN, MSG_INVALID_CREDENTIALS)),
                ),
            }
        })?;

    let claims = token_data.claims;
    
    // Convert sub (string) to user_id (i64)
    let user_id = claims.sub.parse::<i64>()
        .map_err(|_| {
            error!("Invalid user_id in JWT sub field: {}", claims.sub);
            (
                StatusCode::UNAUTHORIZED,
                Json(static_error(ERR_INVALID_TOKEN, MSG_INVALID_CREDENTIALS)),
            )
        })?;
    
    // Create CurrentUser and add to request extensions
    let current_user = CurrentUser {
        user_id,
        email: claims.email.clone(),
        token: token.to_string(),
    };

    info!(
        user_id = current_user.user_id,
        email = %current_user.email,
        "🔐 JWT authentication successful"
    );

    // Add current user to request extensions for handlers to access
    request.extensions_mut().insert(current_user);

    // Continue to the handler
    Ok(next.run(request).await)
}

/// Extractor for getting current user from request extensions
/// Use this in handler functions that require authentication
pub fn get_current_user_from_request(request: &Request) -> Result<CurrentUser, (StatusCode, Json<ErrorResponse>)> {
    extract_user_from_headers(request.headers())
}

/// Extract user from headers (for handlers that don't use middleware)
pub fn extract_user_from_headers(headers: &HeaderMap) -> Result<CurrentUser, (StatusCode, Json<ErrorResponse>)> {
    // Extract Authorization header
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                Json(static_error(ERR_MISSING_AUTH, MSG_AUTH_REQUIRED)),
            )
        })?;

    // Check Bearer prefix
    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid Authorization header format");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(static_error(ERR_INVALID_AUTH_FORMAT, MSG_BEARER_REQUIRED)),
        ));
    }

    // Extract token
    let token = &auth_header[7..]; // Remove "Bearer " prefix

    // Validate and decode JWT token
    match verify_jwt_token(token) {
        Ok(claims) => {
            // Convert sub (string) to user_id (i64)
            let user_id = claims.sub.parse::<i64>()
                .map_err(|_| {
                    error!("Invalid user_id in JWT sub field: {}", claims.sub);
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(static_error(ERR_INVALID_TOKEN, MSG_INVALID_CREDENTIALS)),
                    )
                })?;
                
            info!("Successfully authenticated user: {}", user_id);
            Ok(CurrentUser {
                user_id,
                email: claims.email,
                token: token.to_string(),
            })
        }
        Err(e) => {
            warn!("JWT validation failed: {}", e);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(static_error(ERR_INVALID_TOKEN, MSG_INVALID_CREDENTIALS)),
            ))
        }
    }
}

/// Helper function to validate JWT token (for use in handlers)
pub fn verify_jwt_token(token: &str) -> Result<JwtClaims, String> {
    let jwt_secret = get_jwt_secret();
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(JWT_ALGORITHM);

    decode::<JwtClaims>(token, &decoding_key, &validation)
        .map(|token_data| token_data.claims)
        .map_err(|e| format!("JWT validation failed: {}", e))
}

/// Middleware function to require authentication
pub async fn require_auth(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // First run the extract_current_user middleware
    match extract_current_user(request.headers().clone(), request, next).await {
        Ok(response) => Ok(response),
        Err((status, _)) => Err(status),
    }
}

/// Merchant authentication middleware
/// Extracts and validates merchant JWT tokens
pub async fn extract_merchant(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Get Authorization header
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Authorization header");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Missing Authorization header".to_string(),
                    message: "Authentication required. Please provide a valid Bearer token.".to_string(),
                    details: None,
                }),
            )
        })?;

    // Check Bearer prefix
    if !auth_header.starts_with("Bearer ") {
        warn!("Invalid Authorization header format");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid Authorization header format".to_string(),
                message: "Authorization header must start with 'Bearer '.".to_string(),
                details: None,
            }),
        ));
    }

    // Extract token
    let token = auth_header.trim_start_matches("Bearer ").trim();
    if token.is_empty() {
        warn!("Empty JWT token");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Empty JWT token".to_string(),
                message: "Please provide a valid JWT token.".to_string(),
                details: None,
            }),
        ));
    }

    // Decode and validate JWT
    let jwt_secret = get_jwt_secret();
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(JWT_ALGORITHM);

    let token_data = decode::<MerchantClaims>(token, &decoding_key, &validation)
        .map_err(|e| {
            error!("Merchant JWT validation failed: {}", e);
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Token expired".to_string(),
                        message: "Your session has expired. Please log in again.".to_string(),
                        details: Some(format!("JWT error: {}", e)),
                    }),
                ),
                _ => (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Invalid token".to_string(),
                        message: "Could not validate credentials. Please log in again.".to_string(),
                        details: Some(format!("JWT error: {}", e)),
                    }),
                ),
            }
        })?;

    let claims = token_data.claims;
    
    // Verify role is "merchant"
    if claims.role != "merchant" {
        error!("Invalid role in merchant token: {}", claims.role);
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Invalid token role".to_string(),
                message: "This endpoint requires a merchant token.".to_string(),
                details: None,
            }),
        ));
    }
    
    info!("🏪 Merchant authentication successful: {} ({})", 
          claims.merchant_name, claims.sub);
    
    // Store merchant info in request extensions
    request.extensions_mut().insert(claims.clone());
    
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[test]
    fn test_jwt_validation() {
        let claims = JwtClaims {
            sub: "1".to_string(),  // user_id as string
            email: "test@example.com".to_string(),
            // Test token valid for 1 hour (access tokens typically short-lived)
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            iat: chrono::Utc::now().timestamp(),
            jti: Some("test-jti".to_string()),
        };

        let header = Header::new(JWT_ALGORITHM);
        let jwt_secret = get_jwt_secret();
        let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());
        
        let token = encode(&header, &claims, &encoding_key).unwrap();
        let validated_claims = verify_jwt_token(&token).unwrap();
        
        assert_eq!(validated_claims.sub, "1");
        assert_eq!(validated_claims.email, "test@example.com");
    }
}

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
use tracing::{error, info, warn};

use crate::{
    api::models::ErrorResponse,
};

/// JWT Claims structure matching the token payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,     // Standard JWT subject field (user_id as string)
    pub email: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: Option<String>, // JWT ID for revocation
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

/// Get JWT secret from environment variable with fallback
fn get_jwt_secret() -> String {
    env::var("JWT_SECRET")
        .unwrap_or_else(|_| "lumis_jwt_secret_super_seguro_production_2024_rust_server_key".to_string())
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

    // TODO: Check Redis cache for token validation (similar to Python implementation)
    // For now, we'll decode directly

    // Decode and validate JWT
    let jwt_secret = get_jwt_secret();
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let validation = Validation::new(JWT_ALGORITHM);

    let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)
        .map_err(|e| {
            error!("JWT validation failed: {}", e);
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
    
    // Convert sub (string) to user_id (i64)
    let user_id = claims.sub.parse::<i64>()
        .map_err(|_| {
            error!("Invalid user_id in JWT sub field: {}", claims.sub);
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid token".to_string(),
                    message: "Could not validate credentials. Please log in again.".to_string(),
                    details: Some("Invalid user ID format".to_string()),
                }),
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
                message: "Authorization header must be in format: Bearer <token>".to_string(),
                details: None,
            }),
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
                        Json(ErrorResponse {
                            error: "Invalid token".to_string(),
                            message: "Invalid user ID format in token".to_string(),
                            details: None,
                        }),
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
                Json(ErrorResponse {
                    error: "Invalid or expired token".to_string(),
                    message: format!("JWT validation failed: {}", e),
                    details: None,
                }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[test]
    fn test_jwt_validation() {
        let claims = JwtClaims {
            sub: "1".to_string(),  // user_id as string
            email: "test@example.com".to_string(),
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

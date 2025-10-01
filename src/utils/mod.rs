// Utils module for utility functions
use axum::extract::Request;
use axum::body::Body;
use uuid::Uuid;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,   // Standard JWT subject field (user_id as string)
    email: String,
    exp: i64,      // Expiration timestamp
    iat: i64,      // Issued at timestamp 
    jti: Option<String>,  // JWT ID (optional for compatibility)
}

// Utility function to get request ID from request headers or generate new one
pub fn get_request_id(request: &Request<Body>) -> String {
    // Try to get request ID from headers, otherwise generate new UUID
    request.headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

// Utility function to create JWT token with proper signing
pub fn create_jwt_token(user_id: i64, email: &str) -> Result<String, String> {
    let jwt_secret = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "lumis_jwt_secret_super_seguro_production_2024_rust_server_key".to_string());
    
    let now = Utc::now();
    let expiration = now + chrono::Duration::hours(24); // 24 hours expiration
    
    let claims = JwtClaims {
        sub: user_id.to_string(),  // Convert user_id to string for standard JWT 'sub' field
        email: email.to_string(),
        exp: expiration.timestamp(),
        iat: now.timestamp(),
        jti: Some(Uuid::new_v4().to_string()),
    };

    let encoding_key = EncodingKey::from_secret(jwt_secret.as_bytes());
    
    encode(&Header::default(), &claims, &encoding_key)
        .map_err(|e| format!("Failed to create JWT token: {}", e))
}

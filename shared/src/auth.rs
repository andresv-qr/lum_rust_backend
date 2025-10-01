//! Authentication and authorization service

use crate::{config::AuthConfig, error::AppError, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone)]
pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_ttl: Duration,
    refresh_token_ttl: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub email: String,
    pub source: String,
    pub roles: Vec<String>,
    pub exp: i64, // Expiration time
    pub iat: i64, // Issued at
    pub jti: String, // JWT ID
    pub token_type: TokenType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenType {
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "refresh")]
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    pub email: String,
    pub source: String,
    pub roles: Vec<String>,
}

impl AuthService {
    pub fn new(config: &AuthConfig) -> Result<Self> {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        Ok(Self {
            encoding_key,
            decoding_key,
            access_token_ttl: Duration::seconds(config.access_token_ttl_seconds as i64),
            refresh_token_ttl: Duration::seconds(config.refresh_token_ttl_seconds as i64),
        })
    }

    /// Generate a token pair (access + refresh tokens)
    pub fn generate_token_pair(
        &self,
        user_id: &str,
        email: &str,
        source: &str,
        roles: Vec<String>,
    ) -> Result<TokenPair> {
        let now = Utc::now();
        let jti = uuid::Uuid::new_v4().to_string();

        // Access token
        let access_claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            source: source.to_string(),
            roles: roles.clone(),
            exp: (now + self.access_token_ttl).timestamp(),
            iat: now.timestamp(),
            jti: jti.clone(),
            token_type: TokenType::Access,
        };

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to generate access token: {}", e)))?;

        // Refresh token
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            source: source.to_string(),
            roles,
            exp: (now + self.refresh_token_ttl).timestamp(),
            iat: now.timestamp(),
            jti,
            token_type: TokenType::Refresh,
        };

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to generate refresh token: {}", e)))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_ttl.num_seconds(),
        })
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::default();
        
        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AppError::authentication("Token has expired")
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    AppError::authentication("Invalid token")
                }
                jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    AppError::authentication("Invalid token signature")
                }
                _ => AppError::authentication(format!("Token validation failed: {}", e)),
            })?;

        Ok(token_data.claims)
    }

    /// Extract user context from token
    pub fn extract_user_context(&self, token: &str) -> Result<UserContext> {
        let claims = self.validate_token(token)?;
        
        Ok(UserContext {
            user_id: claims.sub,
            email: claims.email,
            source: claims.source,
            roles: claims.roles,
        })
    }

    /// Check if user has required role
    pub fn has_role(&self, user_context: &UserContext, required_role: &str) -> bool {
        user_context.roles.contains(&required_role.to_string())
    }

    /// Check if user has any of the required roles
    pub fn has_any_role(&self, user_context: &UserContext, required_roles: &[&str]) -> bool {
        let user_roles: HashSet<String> = user_context.roles.iter().cloned().collect();
        let required_roles: HashSet<String> = required_roles.iter().map(|r| r.to_string()).collect();
        
        !user_roles.is_disjoint(&required_roles)
    }

    /// Refresh an access token using a refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair> {
        let claims = self.validate_token(refresh_token)?;

        // Verify it's a refresh token
        match claims.token_type {
            TokenType::Refresh => {}
            TokenType::Access => {
                return Err(AppError::authentication("Invalid token type for refresh"));
            }
        }

        // Generate new token pair
        self.generate_token_pair(&claims.sub, &claims.email, &claims.source, claims.roles)
    }

    /// Hash password
    pub fn hash_password(&self, password: &str) -> Result<String> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::internal(format!("Password hashing failed: {}", e)))
    }

    /// Verify password
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        bcrypt::verify(password, hash)
            .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))
    }

    /// Extract token from Authorization header
    pub fn extract_token_from_header<'a>(&self, auth_header: &'a str) -> Result<&'a str> {
        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::authentication("Invalid authorization header format"));
        }

        let token = &auth_header[7..]; // Remove "Bearer " prefix
        if token.is_empty() {
            return Err(AppError::authentication("Empty token"));
        }

        Ok(token)
    }
}

/// Default roles for the system
pub struct Roles;

impl Roles {
    pub const USER: &'static str = "user";
    pub const ADMIN: &'static str = "admin";
    pub const MODERATOR: &'static str = "moderator";
    pub const SYSTEM: &'static str = "system";
}

/// Middleware helper for extracting user context from request headers
pub mod middleware {
    use super::*;
    use axum::{
        extract::{Request, State},
        middleware::Next,
        response::Response,
    };
    use std::sync::Arc;

    pub async fn auth_middleware(
        State(auth_service): State<Arc<AuthService>>,
        mut request: Request,
        next: Next,
    ) -> std::result::Result<Response, AppError> {
        let headers = request.headers();
        
        if let Some(auth_header) = headers.get("authorization") {
            let auth_str = auth_header
                .to_str()
                .map_err(|_| AppError::authentication("Invalid authorization header"))?;

            let token = auth_service.extract_token_from_header(auth_str)?;
            let user_context = auth_service.extract_user_context(token)?;

            // Add user context to request extensions
            request.extensions_mut().insert(user_context);
        }

        Ok(next.run(request).await)
    }

    pub async fn require_auth_middleware(
        State(auth_service): State<Arc<AuthService>>,
        mut request: Request,
        next: Next,
    ) -> std::result::Result<Response, AppError> {
        let headers = request.headers();
        
        let auth_header = headers
            .get("authorization")
            .ok_or_else(|| AppError::authentication("Missing authorization header"))?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| AppError::authentication("Invalid authorization header"))?;

        let token = auth_service.extract_token_from_header(auth_str)?;
        let user_context = auth_service.extract_user_context(token)?;

        // Add user context to request extensions
        request.extensions_mut().insert(user_context);

        Ok(next.run(request).await)
    }

    pub fn require_role(required_role: &'static str) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<Response, AppError>> + Send>> + Clone {
        move |request: Request, next: Next| {
            Box::pin(async move {
                let user_context = request
                    .extensions()
                    .get::<UserContext>()
                    .ok_or_else(|| AppError::authentication("User context not found"))?;

                if !user_context.roles.contains(&required_role.to_string()) {
                    return Err(AppError::authorization(format!(
                        "Required role '{}' not found",
                        required_role
                    )));
                }

                Ok(next.run(request).await)
            })
        }
    }
}

/// Helper for extracting user context from request extensions
pub fn extract_user_context(request: &axum::extract::Request) -> Result<&UserContext> {
    request
        .extensions()
        .get::<UserContext>()
        .ok_or_else(|| AppError::authentication("User context not found"))
}
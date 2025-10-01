// ============================================================================
// TOKEN SERVICE
// ============================================================================
// Date: September 18, 2025
// Purpose: Service for managing temporary tokens (linking, verification, etc.)
// ============================================================================

use uuid::Uuid;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use serde::{Serialize, Deserialize};
use tracing::{info, error, warn};
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::services::redis_service::RedisService;
use crate::models::auth_provider::{LinkingTokenData, ProviderType};
use crate::models::auth_request::VerificationPurpose;

// ============================================================================
// TOKEN SERVICE
// ============================================================================

#[derive(Clone)]
pub struct TokenService {
    pub redis: RedisService,
    pub linking_token_ttl: ChronoDuration,
    pub verification_code_ttl: ChronoDuration,
}

impl TokenService {
    pub fn new(
        redis: RedisService,
        linking_token_ttl: ChronoDuration,
        verification_code_ttl: ChronoDuration,
    ) -> Self {
        Self {
            redis,
            linking_token_ttl,
            verification_code_ttl,
        }
    }

    // ========================================================================
    // LINKING TOKENS
    // ========================================================================

    /// Generate a linking token for account linking flow
    pub async fn generate_linking_token(
        &self,
        existing_user_id: i64,
        new_provider: ProviderType,
        new_provider_id: String,
        request_id: &str,
    ) -> Result<String, TokenServiceError> {
        let token = Uuid::new_v4().to_string();
        let key = format!("linking_token:{}", token);
        
        let data = LinkingTokenData {
            existing_user_id,
            new_provider: new_provider.clone(),
            new_provider_id,
            created_at: Utc::now(),
            expires_at: Utc::now() + self.linking_token_ttl,
        };

        self.redis
            .set_with_ttl(&key, &data, self.linking_token_ttl.num_seconds() as u64)
            .await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    existing_user_id = %existing_user_id,
                    new_provider = ?new_provider,
                    error = %e,
                    "❌ Failed to store linking token in Redis"
                );
                TokenServiceError::RedisError(e.to_string())
            })?;

        info!(
            request_id = %request_id,
            existing_user_id = %existing_user_id,
            new_provider = ?new_provider,
            token_prefix = %&token[0..8],
            ttl_minutes = %self.linking_token_ttl.num_minutes(),
            "🔗 Generated linking token successfully"
        );

        Ok(token)
    }

    /// Validate and consume a linking token
    pub async fn validate_linking_token(
        &self,
        token: &str,
        request_id: &str,
    ) -> Result<LinkingTokenData, TokenServiceError> {
        let key = format!("linking_token:{}", token);
        
        let data: LinkingTokenData = self.redis
            .get(&key)
            .await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    token_prefix = %&token[0..8.min(token.len())],
                    error = %e,
                    "❌ Failed to retrieve linking token from Redis"
                );
                TokenServiceError::RedisError(e.to_string())
            })?
            .ok_or_else(|| {
                warn!(
                    request_id = %request_id,
                    token_prefix = %&token[0..8.min(token.len())],
                    "🚫 Linking token not found or expired"
                );
                TokenServiceError::TokenNotFound
            })?;

        // Check if token has expired (additional safety check)
        if data.expires_at < Utc::now() {
            self.redis
                .delete(&key)
                .await
                .map_err(|e| TokenServiceError::RedisError(e.to_string()))?;
            
            warn!(
                request_id = %request_id,
                token_prefix = %&token[0..8.min(token.len())],
                expired_at = %data.expires_at,
                "🚫 Linking token has expired"
            );
            return Err(TokenServiceError::TokenExpired);
        }

        // Consume the token (delete it)
        self.redis
            .delete(&key)
            .await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    token_prefix = %&token[0..8.min(token.len())],
                    error = %e,
                    "❌ Failed to delete linking token from Redis"
                );
                TokenServiceError::RedisError(e.to_string())
            })?;

        info!(
            request_id = %request_id,
            existing_user_id = %data.existing_user_id,
            new_provider = ?data.new_provider,
            token_prefix = %&token[0..8.min(token.len())],
            "✅ Linking token validated and consumed successfully"
        );

        Ok(data)
    }

    // ========================================================================
    // VERIFICATION CODES
    // ========================================================================

    /// Generate a verification code for email verification
    pub async fn generate_verification_code(
        &self,
        email: &str,
        purpose: VerificationPurpose,
        request_id: &str,
    ) -> Result<String, TokenServiceError> {
        use rand::Rng;
        
        let code = rand::thread_rng()
            .gen_range(100000..=999999)
            .to_string();
        
        let key = format!("verification:{}:{}", email, purpose);
        
        let data = VerificationCodeData {
            code: code.clone(),
            email: email.to_string(),
            purpose: purpose.clone(),
            attempts: 0,
            created_at: Utc::now(),
            expires_at: Utc::now() + self.verification_code_ttl,
        };

        self.redis
            .set_with_ttl(&key, &data, self.verification_code_ttl.num_seconds() as u64)
            .await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    email = %email,
                    purpose = %purpose,
                    error = %e,
                    "❌ Failed to store verification code in Redis"
                );
                TokenServiceError::RedisError(e.to_string())
            })?;

        info!(
            request_id = %request_id,
            email = %email,
            purpose = %purpose,
            code_prefix = %&code[0..2],
            ttl_minutes = %self.verification_code_ttl.num_minutes(),
            "📧 Generated verification code successfully"
        );

        Ok(code)
    }

    /// Validate a verification code
    pub async fn validate_verification_code(
        &self,
        email: &str,
        code: &str,
        purpose: VerificationPurpose,
        request_id: &str,
    ) -> Result<(), TokenServiceError> {
        let key = format!("verification:{}:{}", email, purpose);
        
        let mut data: VerificationCodeData = self.redis
            .get(&key)
            .await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    email = %email,
                    purpose = %purpose,
                    error = %e,
                    "❌ Failed to retrieve verification code from Redis"
                );
                TokenServiceError::RedisError(e.to_string())
            })?
            .ok_or_else(|| {
                warn!(
                    request_id = %request_id,
                    email = %email,
                    purpose = %purpose,
                    "🚫 Verification code not found or expired"
                );
                TokenServiceError::CodeNotFound
            })?;

        // Check if code has expired
        if data.expires_at < Utc::now() {
            self.delete_verification_code(email, purpose.clone()).await?;
            warn!(
                request_id = %request_id,
                email = %email,
                purpose = %purpose,
                expired_at = %data.expires_at,
                "🚫 Verification code has expired"
            );
            return Err(TokenServiceError::CodeExpired);
        }

        // Increment attempts
        data.attempts += 1;

        // Check if too many attempts
        if data.attempts > 3 {
            self.delete_verification_code(email, purpose.clone()).await?;
            warn!(
                email = %email,
                purpose = %purpose,
                attempts = data.attempts,
                "🚫 Too many verification attempts"
            );
            return Err(TokenServiceError::TooManyAttempts);
        }

        // Validate code
        if data.code != code {
            // Update attempts in Redis
            self.redis
                .set_with_ttl(&key, &data, self.verification_code_ttl.num_seconds() as u64)
                .await
                .map_err(|e| TokenServiceError::RedisError(e.to_string()))?;
            
            warn!(
                request_id = %request_id,
                email = %email,
                purpose = %purpose,
                attempts = data.attempts,
                "🚫 Invalid verification code"
            );
            return Err(TokenServiceError::InvalidCode);
        }

        // Code is valid, delete it
        self.delete_verification_code(email, purpose.clone()).await?;
        info!(
            email = %email,
            purpose = %purpose,
            attempts = data.attempts,
            "✅ Verification code validated successfully"
        );

        Ok(())
    }

    /// Delete a verification code
    pub async fn delete_verification_code(
        &self,
        email: &str,
        purpose: VerificationPurpose,
    ) -> Result<(), TokenServiceError> {
        let key = format!("verification:{}:{}", email, purpose);
        
        self.redis
            .delete(&key)
            .await
            .map_err(|e| TokenServiceError::RedisError(e.to_string()))?;
        
        Ok(())
    }
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationCodeData {
    pub code: String,
    pub email: String,
    pub purpose: VerificationPurpose,
    pub attempts: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum TokenServiceError {
    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Token not found or expired")]
    TokenNotFound,

    #[error("Token has expired")]
    TokenExpired,

    #[error("Verification code not found or expired")]
    CodeNotFound,

    #[error("Verification code has expired")]
    CodeExpired,

    #[error("Invalid verification code")]
    InvalidCode,

    #[error("Too many verification attempts")]
    TooManyAttempts,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl TokenService {
    /// Generate JWT access token for authenticated user
    pub async fn generate_access_token(
        &self,
        user_id: i64,
        email: &str,
    ) -> Result<String, TokenServiceError> {
        #[derive(Debug, Serialize, Deserialize)]
        struct Claims {
            sub: String,
            email: String,
            iat: i64,
            exp: i64,
        }
        
        let now = chrono::Utc::now();
        let expiration = now + chrono::Duration::hours(24); // 24 hour expiration
        
        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            iat: now.timestamp(),
            exp: expiration.timestamp(),
        };
        
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "lumis_jwt_secret_super_seguro_production_2024_rust_server_key".to_string());
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        ).map_err(|e| TokenServiceError::RedisError(format!("JWT encoding error: {}", e)))?;
        
        info!(
            user_id = %user_id,
            email = %email,
            expires_at = %expiration,
            "🔑 Generated access token successfully"
        );
        
        Ok(token)
    }
}
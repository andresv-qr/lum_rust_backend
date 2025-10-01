// ============================================================================
// GOOGLE OAUTH2 SERVICE
// ============================================================================
// Date: September 18, 2025
// Purpose: Google OAuth2 ID token validation and user data extraction
// ============================================================================

use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};

use std::time::{Duration, Instant};
use tracing::{info, error, warn, debug};
use uuid::Uuid;

use crate::models::auth_provider::{GoogleUser, GoogleClaims, GoogleCerts};
use crate::services::redis_service::{RedisService, RedisKeys};

// ============================================================================
// GOOGLE SERVICE
// ============================================================================

#[derive(Clone)]
pub struct GoogleService {
    client_id: String,
    http_client: ReqwestClient,
    redis: RedisService,
    cert_cache_ttl: Duration,
}

impl GoogleService {
    /// Create a new GoogleService instance
    pub fn new(client_id: String, http_client: ReqwestClient, redis: RedisService) -> Self {
        Self {
            client_id,
            http_client,
            redis,
            cert_cache_ttl: Duration::from_secs(24 * 3600), // 24 hours
        }
    }

    /// Validate Google ID token and extract user information
    pub async fn validate_id_token(&self, id_token: &str) -> Result<GoogleUser, GoogleAuthError> {
        let request_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();

        info!(
            request_id = %request_id,
            "🔍 Starting Google ID token validation"
        );

        // 1. Decode header to get key ID (kid)
        let header = jsonwebtoken::decode_header(id_token)
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    error = %e,
                    "❌ Failed to decode ID token header"
                );
                GoogleAuthError::InvalidTokenFormat
            })?;

        let kid = header.kid.ok_or_else(|| {
            error!(
                request_id = %request_id,
                "❌ ID token missing key ID (kid)"
            );
            GoogleAuthError::MissingKeyId
        })?;

        debug!(
            request_id = %request_id,
            kid = %kid,
            algorithm = ?header.alg,
            "📋 ID token header decoded"
        );

        // 2. Get Google public certificates
        let certs = self.get_google_certificates(&request_id).await?;

        // 3. Find the certificate with matching kid
        let jwk = certs.keys.iter()
            .find(|k| k.kid == kid)
            .ok_or_else(|| {
                error!(
                    request_id = %request_id,
                    kid = %kid,
                    available_kids = ?certs.keys.iter().map(|k| &k.kid).collect::<Vec<_>>(),
                    "❌ Certificate not found for key ID"
                );
                GoogleAuthError::CertificateNotFound
            })?;

        debug!(
            request_id = %request_id,
            kid = %kid,
            key_type = %jwk.kty,
            "🔑 Found matching certificate"
        );

        // 4. Construct RSA public key from JWK
        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    error = %e,
                    "❌ Failed to construct RSA key from JWK"
                );
                GoogleAuthError::InvalidCertificate
            })?;

        // 5. Set up token validation parameters
        let mut validation = Validation::new(Algorithm::RS256);
        
        // Use a flexible audience validation
        let client_ids: Vec<&str> = self.client_id.split(',').map(|s| s.trim()).collect();
        validation.set_audience(&client_ids);
        
        validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
        validation.validate_exp = true;
        validation.validate_nbf = false;

        debug!(
            request_id = %request_id,
            audiences = ?client_ids,
            "🔧 Token validation configured for multiple audiences"
        );

        // 6. Decode and validate the token
        let token_data = decode::<GoogleClaims>(id_token, &decoding_key, &validation)
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    error = %e,
                    "❌ Token validation failed"
                );
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => GoogleAuthError::TokenExpired,
                    jsonwebtoken::errors::ErrorKind::InvalidAudience => GoogleAuthError::InvalidAudience,
                    jsonwebtoken::errors::ErrorKind::InvalidIssuer => GoogleAuthError::InvalidIssuer,
                    _ => GoogleAuthError::TokenValidationFailed,
                }
            })?;

        // 7. Additional validations
        let claims = token_data.claims;

        // Validate email verification
        if !claims.email_verified {
            warn!(
                request_id = %request_id,
                email = %claims.email,
                "⚠️ Google user email not verified"
            );
            return Err(GoogleAuthError::EmailNotVerified);
        }

        // 8. Convert to GoogleUser
        let google_user = GoogleUser::from(claims.clone());

        let execution_time = start_time.elapsed();
        info!(
            request_id = %request_id,
            user_id = %google_user.id,
            email = %google_user.email,
            name = ?google_user.name,
            email_verified = google_user.email_verified,
            execution_time_ms = execution_time.as_millis(),
            "✅ Google ID token validated successfully"
        );

        Ok(google_user)
    }

    /// Get Google public certificates (with caching)
    async fn get_google_certificates(&self, request_id: &str) -> Result<GoogleCerts, GoogleAuthError> {
        let cache_key = RedisKeys::google_certs();

        // Try to get from cache first
        match self.redis.get::<GoogleCerts>(&cache_key).await {
            Ok(Some(cached_certs)) => {
                debug!(
                    request_id = %request_id,
                    "📦 Using cached Google certificates"
                );
                return Ok(cached_certs);
            }
            Ok(None) => {
                debug!(
                    request_id = %request_id,
                    "🔄 No cached certificates, fetching from Google"
                );
            }
            Err(e) => {
                warn!(
                    request_id = %request_id,
                    error = %e,
                    "⚠️ Failed to read certificates from cache, fetching from Google"
                );
            }
        }

        // Fetch from Google
        let certs = self.fetch_google_certificates(request_id).await?;

        // Cache the certificates
        if let Err(e) = self.redis.set_with_ttl(&cache_key, &certs, self.cert_cache_ttl.as_secs()).await {
            warn!(
                request_id = %request_id,
                error = %e,
                "⚠️ Failed to cache Google certificates"
            );
        } else {
            info!(
                request_id = %request_id,
                ttl_hours = self.cert_cache_ttl.as_secs() / 3600,
                "💾 Google certificates cached"
            );
        }

        Ok(certs)
    }

    /// Fetch Google certificates from Google's endpoint
    async fn fetch_google_certificates(&self, request_id: &str) -> Result<GoogleCerts, GoogleAuthError> {
        const GOOGLE_CERTS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

        debug!(
            request_id = %request_id,
            url = GOOGLE_CERTS_URL,
            "🌐 Fetching Google certificates"
        );

        let response = self.http_client
            .get(GOOGLE_CERTS_URL)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    error = %e,
                    "❌ Failed to fetch Google certificates"
                );
                GoogleAuthError::CertificateFetchFailed
            })?;

        if !response.status().is_success() {
            error!(
                request_id = %request_id,
                status = %response.status(),
                "❌ Google certificates request failed"
            );
            return Err(GoogleAuthError::CertificateFetchFailed);
        }

        let certs: GoogleCerts = response.json().await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    error = %e,
                    "❌ Failed to parse Google certificates JSON"
                );
                GoogleAuthError::CertificateParseError
            })?;

        info!(
            request_id = %request_id,
            cert_count = certs.keys.len(),
            "✅ Google certificates fetched successfully"
        );

        Ok(certs)
    }

    /// Validate Google user profile data
    pub fn validate_google_user(&self, user: &GoogleUser) -> Result<(), GoogleAuthError> {
        // Validate email format
        if !is_valid_email(&user.email) {
            return Err(GoogleAuthError::InvalidEmail);
        }

        // Validate Google ID format (should be numeric string)
        if user.id.is_empty() || !user.id.chars().all(|c| c.is_ascii_digit()) {
            return Err(GoogleAuthError::InvalidGoogleId);
        }

        // Validate email verification
        if !user.email_verified {
            return Err(GoogleAuthError::EmailNotVerified);
        }

        Ok(())
    }

    /// Get user info from Google API (alternative method)
    pub async fn get_user_info(&self, access_token: &str) -> Result<GoogleUser, GoogleAuthError> {
        let request_id = Uuid::new_v4().to_string();
        
        info!(
            request_id = %request_id,
            "🔍 Fetching Google user info via API"
        );

        let response = self.http_client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    error = %e,
                    "❌ Failed to fetch Google user info"
                );
                GoogleAuthError::UserInfoFetchFailed
            })?;

        if !response.status().is_success() {
            error!(
                request_id = %request_id,
                status = %response.status(),
                "❌ Google user info request failed"
            );
            return Err(GoogleAuthError::UserInfoFetchFailed);
        }

        let user_info: GoogleUserInfo = response.json().await
            .map_err(|e| {
                error!(
                    request_id = %request_id,
                    error = %e,
                    "❌ Failed to parse Google user info JSON"
                );
                GoogleAuthError::UserInfoParseError
            })?;

        let google_user = GoogleUser {
            id: user_info.id,
            email: user_info.email,
            name: user_info.name,
            picture: user_info.picture,
            email_verified: user_info.verified_email.unwrap_or(false),
        };

        info!(
            request_id = %request_id,
            user_id = %google_user.id,
            email = %google_user.email,
            "✅ Google user info fetched successfully"
        );

        Ok(google_user)
    }

    /// Health check for Google OAuth service
    pub async fn health_check(&self) -> Result<GoogleHealthStatus, GoogleAuthError> {
        let start_time = Instant::now();

        // Test certificate fetching
        let cert_result = self.fetch_google_certificates("health_check").await;
        let cert_status = cert_result.is_ok();
        let cert_error = cert_result.err().map(|e| format!("{:?}", e));

        // Test Redis connectivity
        let redis_result = self.redis.health_check().await;
        let redis_status = redis_result.is_ok();
        let redis_error = redis_result.err().map(|e| e.to_string());

        let total_time = start_time.elapsed();

        Ok(GoogleHealthStatus {
            overall_healthy: cert_status && redis_status,
            certificate_fetch: cert_status,
            redis_connectivity: redis_status,
            response_time_ms: total_time.as_millis() as u64,
            cert_error,
            redis_error,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Clear certificate cache (for maintenance)
    pub async fn clear_certificate_cache(&self) -> Result<(), GoogleAuthError> {
        let cache_key = RedisKeys::google_certs();
        
        self.redis.delete(&cache_key).await
            .map_err(|_| GoogleAuthError::CacheClearFailed)?;

        info!("🧹 Google certificate cache cleared");
        Ok(())
    }

    /// Get service configuration info
    pub fn get_config_info(&self) -> GoogleConfigInfo {
        GoogleConfigInfo {
            client_id: mask_client_id(&self.client_id),
            cert_cache_ttl_hours: self.cert_cache_ttl.as_secs() / 3600,
            validation_issuer: "https://accounts.google.com".to_string(),
            certificate_endpoint: "https://www.googleapis.com/oauth2/v3/certs".to_string(),
        }
    }
}

// ============================================================================
// HELPER TYPES AND FUNCTIONS
// ============================================================================

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
    verified_email: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct GoogleHealthStatus {
    pub overall_healthy: bool,
    pub certificate_fetch: bool,
    pub redis_connectivity: bool,
    pub response_time_ms: u64,
    pub cert_error: Option<String>,
    pub redis_error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct GoogleConfigInfo {
    pub client_id: String, // Masked
    pub cert_cache_ttl_hours: u64,
    pub validation_issuer: String,
    pub certificate_endpoint: String,
}

fn is_valid_email(email: &str) -> bool {
    use regex::Regex;
    let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    email_regex.is_match(email)
}

fn mask_client_id(client_id: &str) -> String {
    if client_id.len() > 10 {
        format!("{}...{}", &client_id[..6], &client_id[client_id.len()-4..])
    } else {
        "***".to_string()
    }
}

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum GoogleAuthError {
    #[error("Invalid token format")]
    InvalidTokenFormat,

    #[error("Missing key ID in token header")]
    MissingKeyId,

    #[error("Certificate not found for key ID")]
    CertificateNotFound,

    #[error("Invalid certificate")]
    InvalidCertificate,

    #[error("Token has expired")]
    TokenExpired,

    #[error("Invalid audience")]
    InvalidAudience,

    #[error("Invalid issuer")]
    InvalidIssuer,

    #[error("Token validation failed")]
    TokenValidationFailed,

    #[error("Email not verified by Google")]
    EmailNotVerified,

    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Invalid Google ID")]
    InvalidGoogleId,

    #[error("Failed to fetch certificates")]
    CertificateFetchFailed,

    #[error("Failed to parse certificates")]
    CertificateParseError,

    #[error("Failed to fetch user info")]
    UserInfoFetchFailed,

    #[error("Failed to parse user info")]
    UserInfoParseError,

    #[error("Failed to clear certificate cache")]
    CacheClearFailed,

    #[error("Redis error: {0}")]
    RedisError(String),
}
// ============================================================================
// AUTH PROVIDER MODELS
// ============================================================================
// Date: September 18, 2025
// Purpose: Models for managing authentication providers (Google, Email, etc.)
// ============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuthProviderLink {
    pub id: i32,
    pub user_id: i32,
    pub provider_type: ProviderType,
    pub provider_id: String,
    pub provider_email: Option<String>,
    pub linked_at: DateTime<Utc>,
    pub link_method: LinkMethod,
    pub is_primary: bool,
    #[sqlx(try_from = "serde_json::Value")]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar")]
pub enum ProviderType {
    #[sqlx(rename = "email")]
    Email,
    #[sqlx(rename = "google")]
    Google,
    #[sqlx(rename = "facebook")]
    Facebook,
    #[sqlx(rename = "apple")]
    Apple,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Email => write!(f, "email"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::Facebook => write!(f, "facebook"),
            ProviderType::Apple => write!(f, "apple"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "email" => Ok(ProviderType::Email),
            "google" => Ok(ProviderType::Google),
            "facebook" => Ok(ProviderType::Facebook),
            "apple" => Ok(ProviderType::Apple),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum LinkMethod {
    #[sqlx(rename = "automatic")]
    Automatic,
    #[sqlx(rename = "manual")]
    Manual,
    #[sqlx(rename = "admin")]
    Admin,
}

impl std::fmt::Display for LinkMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkMethod::Automatic => write!(f, "automatic"),
            LinkMethod::Manual => write!(f, "manual"),
            LinkMethod::Admin => write!(f, "admin"),
        }
    }
}

// ============================================================================
// GOOGLE USER DATA MODEL
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUser {
    pub id: String,                    // Google User ID
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub email_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleClaims {
    pub iss: String,                   // Issuer
    pub aud: String,                   // Audience
    pub sub: String,                   // Subject (Google User ID)
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub exp: usize,                    // Expiration time
    pub iat: usize,                    // Issued at
}

impl From<GoogleClaims> for GoogleUser {
    fn from(claims: GoogleClaims) -> Self {
        Self {
            id: claims.sub,
            email: claims.email,
            name: claims.name,
            picture: claims.picture,
            email_verified: claims.email_verified,
        }
    }
}

// ============================================================================
// GOOGLE CERTIFICATES MODEL
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleCerts {
    pub keys: Vec<GoogleJwk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleJwk {
    pub kid: String,
    pub kty: String,
    pub n: String,
    pub e: String,
}

// ============================================================================
// LINKING TOKEN DATA
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkingTokenData {
    pub existing_user_id: i64,  // Changed to i64 for consistency
    pub new_provider: ProviderType,
    pub new_provider_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
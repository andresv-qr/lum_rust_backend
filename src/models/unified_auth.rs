// ============================================================================
// UNIFIED AUTHENTICATION REQUEST/RESPONSE MODELS
// ============================================================================
// Date: September 19, 2025
// Purpose: Models for unified authentication endpoint
// ============================================================================

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::models::{
    user::{User, AccountStatus},
    auth_provider::ProviderType,
};

// ============================================================================
// REQUEST MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UnifiedAuthRequest {
    #[serde(flatten)]
    pub provider_data: ProviderData,
    
    /// Client information for audit
    pub client_info: Option<ClientInfo>,
    
    /// Whether to create account if not exists (for registration)
    #[serde(default)]
    pub create_if_not_exists: bool,
    
    /// Linking token for account linking flows
    pub linking_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "provider")]
pub enum ProviderData {
    #[serde(rename = "email")]
    Email {
        email: String,
        password: String,
        /// For registration only
        name: Option<String>,
    },
    
    #[serde(rename = "google")]
    Google {
        /// Google ID token from OAuth flow
        id_token: String,
        /// Optional access token for additional info
        access_token: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientInfo {
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub device_id: Option<String>,
    pub app_version: Option<String>,
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UnifiedAuthResponse {
    #[serde(flatten)]
    pub result: AuthResult,
    
    /// Authentication metadata
    pub metadata: AuthMetadata,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum AuthResult {
    #[serde(rename = "success")]
    Success {
        user: AuthenticatedUser,
        token: String,
        expires_at: DateTime<Utc>,
    },
    
    #[serde(rename = "account_linking_required")]
    AccountLinkingRequired {
        message: String,
        linking_token: String,
        expires_at: DateTime<Utc>,
        existing_providers: Vec<String>,
        new_provider: String,
    },
    
    #[serde(rename = "email_verification_required")]
    EmailVerificationRequired {
        message: String,
        user_id: i64,
        email: String,
    },
    
    #[serde(rename = "account_suspended")]
    AccountSuspended {
        message: String,
        reason: Option<String>,
        until: Option<DateTime<Utc>>,
    },
    
    #[serde(rename = "error")]
    Error {
        message: String,
        error_code: String,
        retry_after: Option<u64>,
    },
}

#[derive(Debug, Serialize)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub providers: Vec<String>,
    pub primary_provider: String,
    pub email_verified: bool,
    pub account_status: AccountStatus,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AuthMetadata {
    pub request_id: String,
    pub provider_used: String,
    pub is_new_user: bool,
    pub linking_performed: bool,
    pub execution_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// CONVERSION IMPLEMENTATIONS
// ============================================================================

impl From<User> for AuthenticatedUser {
    fn from(user: User) -> Self {
        let providers = user.auth_providers
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| vec!["email".to_string()]);
            
        let primary_provider = user.last_login_provider
            .unwrap_or_else(|| providers.first().cloned().unwrap_or_else(|| "email".to_string()));
        
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            avatar_url: user.avatar_url,
            providers,
            primary_provider,
            email_verified: user.email_verified_at.is_some(),
            account_status: user.account_status,
            created_at: user.created_at,
            last_login_at: user.last_login_at,
        }
    }
}

impl ProviderData {
    pub fn provider_type(&self) -> ProviderType {
        match self {
            ProviderData::Email { .. } => ProviderType::Email,
            ProviderData::Google { .. } => ProviderType::Google,
        }
    }
    
    pub fn email(&self) -> Option<&str> {
        match self {
            ProviderData::Email { email, .. } => Some(email),
            ProviderData::Google { .. } => None, // Will be extracted from token
        }
    }
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConfigResponse {
    pub providers: Vec<String>,
    pub features: Vec<String>,
    pub version: String,
}
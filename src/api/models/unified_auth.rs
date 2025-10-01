use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

// ============================================================================
// PROVIDER TYPES AND ENUMS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "varchar")]
#[serde(rename_all = "lowercase")]
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

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::Email => "email",
            ProviderType::Google => "google",
            ProviderType::Facebook => "facebook",
            ProviderType::Apple => "apple",
        }
    }
    
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "email" => Ok(ProviderType::Email),
            "google" => Ok(ProviderType::Google),
            "facebook" => Ok(ProviderType::Facebook),
            "apple" => Ok(ProviderType::Apple),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "varchar")]
#[serde(rename_all = "lowercase")]
pub enum LinkMethod {
    #[sqlx(rename = "automatic")]
    Automatic,
    #[sqlx(rename = "manual")]
    Manual,
    #[sqlx(rename = "admin")]
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "varchar")]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    #[sqlx(rename = "active")]
    Active,
    #[sqlx(rename = "suspended")]
    Suspended,
    #[sqlx(rename = "pending_verification")]
    PendingVerification,
    #[sqlx(rename = "locked")]
    Locked,
}

// ============================================================================
// EXTENDED USER MODEL
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UnifiedUser {
    pub id: i32,
    pub email: String,
    pub password_hash: Option<String>,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub avatar_url: Option<String>,
    
    // Unified auth fields
    #[sqlx(json)]
    pub auth_providers: Vec<String>,
    pub google_id: Option<String>,
    #[sqlx(json)]
    pub auth_metadata: serde_json::Value,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_login_provider: Option<String>,
    #[sqlx(try_from = "String")]
    pub account_status: AccountStatus,
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UnifiedUser {
    pub fn has_provider(&self, provider: &ProviderType) -> bool {
        self.auth_providers.contains(&provider.to_string())
    }
    
    pub fn is_email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
    
    pub fn is_active(&self) -> bool {
        self.account_status == AccountStatus::Active
    }
    
    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }
    
    pub fn get_display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            // Extract name from email if no name is set
            self.email.split('@').next().unwrap_or("User").to_string()
        })
    }
}

// ============================================================================
// AUTH PROVIDER LINK MODEL
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuthProviderLink {
    pub id: i32,
    pub user_id: i32,
    #[sqlx(try_from = "String")]
    pub provider_type: ProviderType,
    pub provider_id: String,
    pub provider_email: Option<String>,
    pub linked_at: DateTime<Utc>,
    #[sqlx(try_from = "String")]
    pub link_method: LinkMethod,
    pub is_primary: bool,
    #[sqlx(json)]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// AUTH AUDIT LOG MODEL
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuthAuditLog {
    pub id: i32,
    pub user_id: Option<i32>,
    pub event_type: String,
    pub provider: Option<String>,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    #[sqlx(json)]
    pub metadata: serde_json::Value,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// USER CREATION AND UPDATE STRUCTS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUnifiedUser {
    pub email: String,
    pub password_hash: Option<String>,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub avatar_url: Option<String>,
    pub auth_providers: Vec<String>,
    pub google_id: Option<String>,
    pub auth_metadata: serde_json::Value,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_login_provider: String,
    pub account_status: AccountStatus,
}

impl Default for CreateUnifiedUser {
    fn default() -> Self {
        Self {
            email: String::new(),
            password_hash: None,
            name: None,
            phone: None,
            country: None,
            avatar_url: None,
            auth_providers: vec!["email".to_string()],
            google_id: None,
            auth_metadata: serde_json::json!({}),
            email_verified_at: None,
            last_login_provider: "email".to_string(),
            account_status: AccountStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUnifiedUser {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub avatar_url: Option<String>,
    pub auth_providers: Option<Vec<String>>,
    pub google_id: Option<String>,
    pub auth_metadata: Option<serde_json::Value>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_login_provider: Option<String>,
    pub account_status: Option<AccountStatus>,
}

// ============================================================================
// CREATE PROVIDER LINK STRUCT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuthProviderLink {
    pub user_id: i32,
    pub provider_type: ProviderType,
    pub provider_id: String,
    pub provider_email: Option<String>,
    pub link_method: LinkMethod,
    pub is_primary: bool,
    pub metadata: serde_json::Value,
}

impl Default for CreateAuthProviderLink {
    fn default() -> Self {
        Self {
            user_id: 0,
            provider_type: ProviderType::Email,
            provider_id: String::new(),
            provider_email: None,
            link_method: LinkMethod::Automatic,
            is_primary: false,
            metadata: serde_json::json!({}),
        }
    }
}

// ============================================================================
// USER WITH PROVIDERS (COMBINED VIEW)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithProviders {
    #[serde(flatten)]
    pub user: UnifiedUser,
    pub provider_links: Vec<AuthProviderLink>,
}

impl UserWithProviders {
    pub fn get_primary_provider(&self, provider_type: &ProviderType) -> Option<&AuthProviderLink> {
        self.provider_links
            .iter()
            .find(|link| link.provider_type == *provider_type && link.is_primary)
    }
    
    pub fn get_all_providers(&self) -> Vec<&ProviderType> {
        self.provider_links
            .iter()
            .map(|link| &link.provider_type)
            .collect()
    }
    
    pub fn has_multiple_providers(&self) -> bool {
        self.provider_links.len() > 1
    }
}

// ============================================================================
// GOOGLE USER DATA STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUser {
    pub id: String,          // Google user ID (sub claim)
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub email_verified: bool,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub locale: Option<String>,
}

impl GoogleUser {
    pub fn to_auth_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "google_id": self.id,
            "email_verified": self.email_verified,
            "given_name": self.given_name,
            "family_name": self.family_name,
            "picture": self.picture,
            "locale": self.locale,
            "imported_at": Utc::now().to_rfc3339()
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleClaims {
    pub iss: String,           // Issuer
    pub aud: String,           // Audience (our client ID)
    pub sub: String,           // Subject (Google user ID)
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub locale: Option<String>,
    pub exp: usize,           // Expiration time
    pub iat: usize,           // Issued at
}

impl From<GoogleClaims> for GoogleUser {
    fn from(claims: GoogleClaims) -> Self {
        Self {
            id: claims.sub,
            email: claims.email,
            name: claims.name,
            picture: claims.picture,
            email_verified: claims.email_verified,
            given_name: claims.given_name,
            family_name: claims.family_name,
            locale: claims.locale,
        }
    }
}

// ============================================================================
// LINKING TOKEN DATA
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkingTokenData {
    pub existing_user_id: i32,
    pub new_provider: ProviderType,
    pub new_provider_id: String,
    pub new_provider_email: String,
    pub new_provider_data: GoogleUser,
    pub expires_at: DateTime<Utc>,
    pub verification_required: bool,
    pub token_id: String,
}

// ============================================================================
// RESPONSE MODELS FOR API
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub auth_providers: Vec<String>,
    pub email_verified: bool,
    pub account_status: AccountStatus,
    pub created_at: DateTime<Utc>,
}

impl From<UnifiedUser> for UserResponse {
    fn from(user: UnifiedUser) -> Self {
        let email_verified = user.is_email_verified();
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            avatar_url: user.avatar_url,
            auth_providers: user.auth_providers,
            email_verified,
            account_status: user.account_status,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderResponse {
    pub provider_type: ProviderType,
    pub provider_email: Option<String>,
    pub is_primary: bool,
    pub linked_at: DateTime<Utc>,
    pub link_method: LinkMethod,
}

impl From<AuthProviderLink> for ProviderResponse {
    fn from(link: AuthProviderLink) -> Self {
        Self {
            provider_type: link.provider_type,
            provider_email: link.provider_email,
            is_primary: link.is_primary,
            linked_at: link.linked_at,
            link_method: link.link_method,
        }
    }
}

// ============================================================================
// CONVERSION HELPERS
// ============================================================================

impl std::str::FromStr for AccountStatus {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(AccountStatus::Active),
            "suspended" => Ok(AccountStatus::Suspended),
            "pending_verification" => Ok(AccountStatus::PendingVerification),
            "locked" => Ok(AccountStatus::Locked),
            _ => Err(format!("Unknown account status: {}", s)),
        }
    }
}

impl From<String> for AccountStatus {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(AccountStatus::Active)
    }
}

impl From<String> for ProviderType {
    fn from(s: String) -> Self {
        ProviderType::from_str(&s).unwrap_or(ProviderType::Email)
    }
}

impl From<String> for LinkMethod {
    fn from(s: String) -> Self {
        match s.as_str() {
            "automatic" => LinkMethod::Automatic,
            "manual" => LinkMethod::Manual,
            "admin" => LinkMethod::Admin,
            _ => LinkMethod::Automatic,
        }
    }
}
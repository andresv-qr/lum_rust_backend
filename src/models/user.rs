use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// UNIFIED USER MODEL FOR AUTHENTICATION SYSTEM
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: Option<String>,  // Made optional to handle legacy data
    pub password_hash: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub is_active: bool,
    
    // Legacy fields still relevant
    pub ws_id: Option<String>,
    pub date_of_birth: Option<String>,
    pub country_origin: Option<String>,
    pub country_residence: Option<String>,
    pub telegram_id: Option<String>,
    pub email_registration_date: Option<String>,  // Temporary fix for type mismatch
    pub ws_registration_date: Option<i32>,       // Temporary fix for type mismatch  
    pub telegram_registration_date: Option<i32>, // Temporary fix for type mismatch
    
    // Unified Auth Fields (simplified to avoid JsonValue issues)
    pub auth_providers: Option<String>,     // JSON string ['email', 'google']
    pub google_id: Option<String>,
    pub auth_metadata: Option<String>,      // JSON string
    pub email_verified_at: Option<String>,       // Temporary fix for type mismatch
    pub last_login_provider: Option<String>,
    pub account_status: AccountStatus,
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
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

impl std::fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountStatus::Active => write!(f, "active"),
            AccountStatus::Suspended => write!(f, "suspended"),
            AccountStatus::PendingVerification => write!(f, "pending_verification"),
            AccountStatus::Locked => write!(f, "locked"),
        }
    }
}

// ============================================================================
// USER RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub email: Option<String>,  // Made optional
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub auth_providers: Vec<String>,
    pub email_verified: bool,
    pub account_status: AccountStatus,
    pub last_login_provider: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        let auth_providers = user.auth_providers
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_else(Vec::new);
            
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            avatar_url: user.avatar_url,
            auth_providers,
            email_verified: user.email_verified_at.is_some(),
            account_status: user.account_status,
            last_login_provider: user.last_login_provider,
            created_at: user.created_at,
        }
    }
}

// ============================================================================
// LEGACY USER STATE (PRESERVED FOR COMPATIBILITY)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserState {
    Survey(SurveyState),
    ProductSearch,
    OcrInvoice,
    WaitingForImage,
    WaitingForImageOcr,
    OffersRadar {
        step: String,
        categories: Vec<String>,
    },
    PriceRange(String), // JSON string containing price range state
}

impl std::fmt::Display for UserState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserState::Survey(_) => write!(f, "Survey"),
            UserState::ProductSearch => write!(f, "ProductSearch"),
            UserState::WaitingForImage => write!(f, "WaitingForImage"),
            UserState::WaitingForImageOcr => write!(f, "WaitingForImageOcr"),
            UserState::OcrInvoice => write!(f, "OcrInvoice"),
            UserState::OffersRadar { .. } => write!(f, "OffersRadar"),
            UserState::PriceRange(_) => write!(f, "PriceRange"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SurveyState {
    pub step: String,
    pub name: Option<String>,
    pub birth_date: Option<String>,
    pub country_of_origin: Option<String>,
    pub country_of_residence: Option<String>,
    pub email: Option<String>,
}

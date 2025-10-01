// ============================================================================
// AUDIT LOG MODELS
// ============================================================================
// Date: September 18, 2025
// Purpose: Models for authentication audit logging and events
// ============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// AUTH AUDIT LOG
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuthAuditLog {
    pub id: i32,
    pub user_id: Option<i32>,
    pub event_type: String,
    pub provider: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    #[sqlx(try_from = "serde_json::Value")]
    pub metadata: serde_json::Value,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// AUTH EVENT BUILDER
// ============================================================================

#[derive(Debug, Clone)]
pub struct AuthEvent {
    pub user_id: Option<i32>,
    pub event_type: AuthEventType,
    pub provider: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
}

impl Default for AuthEvent {
    fn default() -> Self {
        Self {
            user_id: None,
            event_type: AuthEventType::LoginAttempt,
            provider: None,
            ip_address: None,
            user_agent: None,
            success: false,
            error_code: None,
            error_message: None,
            metadata: serde_json::json!({}),
            session_id: None,
            request_id: None,
        }
    }
}

impl AuthEvent {
    pub fn new(event_type: AuthEventType) -> Self {
        Self {
            event_type,
            ..Default::default()
        }
    }
    
    pub fn user_id(mut self, user_id: i32) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn provider(mut self, provider: &str) -> Self {
        self.provider = Some(provider.to_string());
        self
    }
    
    pub fn ip_address(mut self, ip: &str) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }
    
    pub fn user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }
    
    pub fn success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }
    
    pub fn error(mut self, code: &str, message: &str) -> Self {
        self.error_code = Some(code.to_string());
        self.error_message = Some(message.to_string());
        self.success = false;
        self
    }
    
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
    
    pub fn session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }
    
    pub fn request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }
}

// ============================================================================
// AUTH EVENT TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthEventType {
    // Login Events
    LoginAttempt,
    LoginSuccess,
    LoginFailure,
    
    // Registration Events
    RegisterAttempt,
    RegisterSuccess,
    RegisterFailure,
    
    // Provider-specific Events
    GoogleAuth,
    EmailAuth,
    
    // Account Management
    AccountLinking,
    ProviderAdded,
    ProviderRemoved,
    
    // Verification Events
    EmailVerification,
    VerificationCodeSent,
    VerificationCodeUsed,
    
    // Security Events
    PasswordReset,
    PasswordChange,
    AccountLocked,
    AccountUnlocked,
    
    // Session Events
    SessionCreated,
    SessionExpired,
    SessionRevoked,
    
    // Rate Limiting
    RateLimited,
    
    // Errors
    AuthError,
}

impl std::fmt::Display for AuthEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuthEventType::LoginAttempt => "login_attempt",
            AuthEventType::LoginSuccess => "login_success",
            AuthEventType::LoginFailure => "login_failure",
            AuthEventType::RegisterAttempt => "register_attempt",
            AuthEventType::RegisterSuccess => "register_success",
            AuthEventType::RegisterFailure => "register_failure",
            AuthEventType::GoogleAuth => "google_auth",
            AuthEventType::EmailAuth => "email_auth",
            AuthEventType::AccountLinking => "account_linking",
            AuthEventType::ProviderAdded => "provider_added",
            AuthEventType::ProviderRemoved => "provider_removed",
            AuthEventType::EmailVerification => "email_verification",
            AuthEventType::VerificationCodeSent => "verification_code_sent",
            AuthEventType::VerificationCodeUsed => "verification_code_used",
            AuthEventType::PasswordReset => "password_reset",
            AuthEventType::PasswordChange => "password_change",
            AuthEventType::AccountLocked => "account_locked",
            AuthEventType::AccountUnlocked => "account_unlocked",
            AuthEventType::SessionCreated => "session_created",
            AuthEventType::SessionExpired => "session_expired",
            AuthEventType::SessionRevoked => "session_revoked",
            AuthEventType::RateLimited => "rate_limited",
            AuthEventType::AuthError => "auth_error",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// AUDIT QUERY MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub user_id: Option<i32>,
    pub event_type: Option<String>,
    pub provider: Option<String>,
    pub success: Option<bool>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogSummary {
    pub total_events: i64,
    pub successful_events: i64,
    pub failed_events: i64,
    pub unique_users: i64,
    pub unique_ips: i64,
    pub events_by_type: Vec<EventTypeCount>,
    pub events_by_provider: Vec<ProviderCount>,
    pub events_by_day: Vec<DayCount>,
}

#[derive(Debug, Serialize)]
pub struct EventTypeCount {
    pub event_type: String,
    pub count: i64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct ProviderCount {
    pub provider: String,
    pub count: i64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct DayCount {
    pub date: String,
    pub total_events: i64,
    pub successful_events: i64,
    pub failed_events: i64,
}

// ============================================================================
// RATE LIMITING MODELS
// ============================================================================

#[derive(Debug, Clone)]
pub struct RateLimitEntry {
    pub count: u32,
    pub window_start: DateTime<Utc>,
    pub blocked_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    pub allowed: bool,
    pub count: u32,
    pub limit: u32,
    pub window_seconds: i64,
    pub retry_after: Option<i64>,
    pub blocked: bool,
}
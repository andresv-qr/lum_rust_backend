pub mod invoice;
pub mod user;
pub mod whatsapp;
pub mod ocr;
pub mod rewards;

// Unified Authentication Models
pub mod auth_provider;
pub mod auth_request;
pub mod auth_response;
pub mod audit_log;
pub mod unified_auth;  // New unified auth models

// Re-export commonly used unified auth types
pub use auth_provider::{AuthProviderLink, ProviderType, LinkMethod};
pub use auth_request::{UnifiedAuthRequest, LinkAccountRequest, VerifyEmailRequest, ResendVerificationRequest};
pub use auth_response::{UnifiedAuthResponse, AuthResponseType, AuthTokens, VerificationRequired};
pub use audit_log::{AuthAuditLog, AuthEvent, AuthEventType};
pub use unified_auth::{UnifiedAuthRequest as UnifiedRequest, UnifiedAuthResponse as UnifiedResponse, ProviderData, AuthResult, AuthenticatedUser};

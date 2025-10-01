// ============================================================================
// API MODELS MODULE
// ============================================================================
// Exports for all API models used in the unified authentication system
// ============================================================================

pub mod unified_auth;
pub mod legacy_models;

// Re-export commonly used types from unified auth
pub use unified_auth::{
    // Core enums
    ProviderType,
    LinkMethod,
    AccountStatus,
    
    // Main models
    UnifiedUser,
    AuthProviderLink,
    AuthAuditLog,
    UserWithProviders,
    
    // Creation structs
    CreateUnifiedUser,
    UpdateUnifiedUser,
    CreateAuthProviderLink,
    
    // Google OAuth models
    GoogleUser,
    GoogleClaims,
    
    // Linking models
    LinkingTokenData,
    
    // Response models
    UserResponse,
    ProviderResponse,
};

// Re-export legacy models for backward compatibility
pub use legacy_models::*;

// TokenResponse from existing templates - temporarily commented for compilation
// use crate::api::templates::auth_templates::TokenResponse;
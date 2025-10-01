// Re-export domain services from new structure
pub use crate::domains::qr::service as qr_domain_service;
pub use crate::domains::qr::rust_qreader;
pub use crate::domains::qr::python_client as python_qreader_client;
pub use crate::domains::ocr::service as ocr_domain_service;
pub use crate::domains::rewards::service as rewards_service;
pub use crate::domains::invoices::service as invoice_domain_service;

// Re-export shared services from new structure
pub use crate::shared::database as db_service;

// ============================================================================
// UNIFIED AUTH SERVICES
// ============================================================================
pub mod token_service;
pub mod redis_service;
pub mod google_service;
// pub mod unified_auth_service;  // New unified auth service - TEMPORARILY DISABLED
pub mod unified_auth_simple;   // Simplified auth service

// Re-export unified auth services
pub use token_service::TokenService;
pub use redis_service::RedisService;
pub use google_service::GoogleService;
// pub use unified_auth_service::UnifiedAuthService; // Temporarily disabled
pub use crate::shared::redis_service as redis_service_compat;
pub use crate::shared::users as user_service;
pub use crate::shared::whatsapp as whatsapp_service;
pub use crate::shared::dashboard as visual_dashboard_service;

// New OCR services
pub mod ocr_session_service;
pub mod ocr_processing_service;

pub mod ocr_service; // Common OCR service extracted from WhatsApp

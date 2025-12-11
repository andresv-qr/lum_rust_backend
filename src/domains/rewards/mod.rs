pub mod models;
pub mod offer_service;
pub mod qr_generator;
pub mod redemption_service;
pub mod service;
pub mod async_qr;

// Re-exports para facilitar imports
pub use models::*;
pub use offer_service::OfferService;
pub use qr_generator::{QrConfig, QrGenerator, ValidationTokenClaims};
pub use redemption_service::RedemptionService;
pub use service::*;
pub use async_qr::{AsyncQrService, QrGenerationTask, QrWorkerConfig};

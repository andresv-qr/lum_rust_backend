// ============================================================================
// CACHE MODULE - Servicios de cache
// ============================================================================

pub mod offers_cache;
pub mod user_cache;

// Re-export para compatibilidad
pub use offers_cache::{OffersCacheService, OffersCacheConfig, OffersCacheWrapper};
pub use user_cache::*;

/// Daily Game - Sistema de juego diario de constelación
/// 
/// Endpoints:
/// - POST /v4/daily-game/claim - Reclamar recompensa diaria
/// - GET /v4/daily-game/status - Verificar estado del juego

pub mod templates;
pub mod claim;
pub mod status;

// Re-exports para facilitar uso
pub use templates::{
    DailyGameClaimRequest,
    DailyGameClaimResponse,
    DailyGameStatusResponse,
    DailyGameStats,
};

pub use claim::handle_claim;
pub use status::handle_status;

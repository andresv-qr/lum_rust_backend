use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

// ============================================================================
// REQUEST MODELS
// ============================================================================

/// Request para reclamar recompensa diaria
#[derive(Debug, Deserialize)]
pub struct DailyGameClaimRequest {
    /// ID de la estrella seleccionada (star_0 a star_8)
    pub star_id: String,
    
    /// Lümis ganados: 0 (vacía), 1 (normal), o 5 (dorada)
    pub lumis_won: i32,
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

/// Respuesta exitosa del claim
#[derive(Debug, Serialize)]
pub struct DailyGameClaimResponse {
    /// Lümis agregados en esta jugada
    pub lumis_added: i32,
    
    /// Nuevo balance total de Lümis
    pub new_balance: i32,
    
    /// ID de la jugada registrada
    pub play_id: i64,
}

/// Estado del juego diario para el usuario
#[derive(Debug, Serialize)]
pub struct DailyGameStatusResponse {
    /// Si puede jugar hoy
    pub can_play_today: bool,
    
    /// Si ya jugó hoy
    pub has_played_today: bool,
    
    /// Fecha de última jugada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_date: Option<NaiveDate>,
    
    /// Recompensa de hoy (si ya jugó)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todays_reward: Option<i32>,
    
    /// Estadísticas básicas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<DailyGameStats>,
}

/// Estadísticas básicas del juego diario
#[derive(Debug, Serialize)]
pub struct DailyGameStats {
    /// Total de jugadas
    pub total_plays: i32,
    
    /// Total de Lümis ganados
    pub total_lumis_won: i32,
    
    /// Estrellas doradas capturadas (5 Lümis)
    pub golden_stars_captured: i32,
}

// ============================================================================
// VALIDATION
// ============================================================================

impl DailyGameClaimRequest {
    /// Valida que los valores del request sean correctos
    pub fn validate(&self) -> Result<(), String> {
        // Validar lumis_won
        if ![0, 1, 5].contains(&self.lumis_won) {
            return Err(format!(
                "Invalid lumis_won value: {}. Must be 0, 1, or 5",
                self.lumis_won
            ));
        }
        
        // Validar formato de star_id
        if !self.star_id.starts_with("star_") {
            return Err(format!(
                "Invalid star_id format: {}. Must start with 'star_'",
                self.star_id
            ));
        }
        
        // Validar número de estrella (0-8)
        if let Some(num_str) = self.star_id.strip_prefix("star_") {
            match num_str.parse::<u8>() {
                Ok(num) if num <= 8 => Ok(()),
                _ => Err(format!(
                    "Invalid star_id: {}. Must be star_0 to star_8",
                    self.star_id
                )),
            }
        } else {
            Err(format!("Invalid star_id format: {}", self.star_id))
        }
    }
}

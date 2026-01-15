//! Modelos del sistema de redención de Lümis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ======================================================================
// OFERTAS
// ======================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RedemptionOffer {
    pub id: i32,
    pub offer_id: Uuid,
    pub name: String,
    pub name_friendly: Option<String>,
    pub description_friendly: Option<String>,
    pub points: Option<i32>,
    pub lumis_cost: Option<i32>,
    pub offer_category: Option<String>,
    pub merchant_id: Option<Uuid>,
    pub merchant_name: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub stock_quantity: Option<i32>,
    pub max_redemptions_per_user: Option<i32>,
    pub img: Option<String>,
    pub terms_and_conditions: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

impl RedemptionOffer {
    pub fn is_currently_valid(&self) -> bool {
        if let (Some(from), Some(to)) = (self.valid_from, self.valid_to) {
            let now = Utc::now();
            now >= from && now <= to
        } else {
            true
        }
    }

    pub fn has_stock(&self) -> bool {
        self.stock_quantity.map_or(true, |stock| stock > 0)
    }

    pub fn get_cost(&self) -> i32 {
        self.lumis_cost.unwrap_or(self.points.unwrap_or(0))
    }
}

#[derive(Debug, Serialize)]
pub struct OfferListItem {
    pub offer_id: Uuid,
    pub name_friendly: String,
    pub description_friendly: String,
    pub lumis_cost: i32,
    pub category: String,
    pub merchant_name: String,
    pub image_url: Option<String>,
    pub is_available: bool,
    pub stock_remaining: Option<i32>,
    pub max_redemptions_per_user: i32,
    pub user_redemptions_count: i64,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OfferFilters {
    pub category: Option<String>,
    pub min_cost: Option<i32>,
    pub max_cost: Option<i32>,
    pub merchant_id: Option<Uuid>,
    pub offer_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Filtros para endpoint my-offers
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MyOffersFilters {
    /// Estado: active (vigentes sin redimir), redeemed (redimidas), expired (expiradas), all
    pub status: Option<String>,
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Resumen de redenciones de una oferta por el usuario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedemptionsSummary {
    pub total: i32,
    pub pending: i32,
    pub confirmed: i32,
    pub cancelled: i32,
    pub expired: i32,
}

/// Item de oferta para "Mis Ofertas" con información enriquecida
#[derive(Debug, Serialize)]
pub struct MyOfferItem {
    pub offer_id: Uuid,
    pub name_friendly: String,
    pub description_friendly: String,
    pub lumis_cost: i32,
    pub category: String,
    pub merchant_name: String,
    pub image_url: Option<String>,
    /// Estado de la oferta para este usuario: active, redeemed, expired
    pub status: String,
    /// Si la oferta sigue vigente en el catálogo
    pub is_still_available: bool,
    /// Cuántas veces el usuario ha redimido esta oferta
    pub user_redemptions_count: i32,
    /// Máximo permitido por usuario
    pub max_redemptions_per_user: i32,
    /// Última vez que redimió esta oferta
    pub last_redeemed_at: Option<DateTime<Utc>>,
    /// Fecha de expiración de la oferta
    pub offer_expires_at: Option<DateTime<Utc>>,
    /// Resumen de redenciones por estado
    pub redemptions_summary: RedemptionsSummary,
    /// Stock inicial de la oferta (None = ilimitado)
    pub stock_initial: Option<i32>,
    /// Stock restante disponible
    pub stock_remaining: Option<i32>,
    /// Total de redenciones de TODOS los usuarios (no canceladas)
    pub total_redemptions_count: i32,
    /// Porcentaje de uso (0-100) basado en stock o null si ilimitado
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_percentage: Option<f32>,
}

// ======================================================================
// REDENCIONES
// ======================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRedemption {
    pub redemption_id: Uuid,
    pub offer_name: String,
    pub lumis_spent: i32,
    pub new_balance: i32,
    pub redemption_code: String,
    pub short_code: Option<String>, // Nuevo campo
    pub qr_image_url: Option<String>,
    pub qr_landing_url: Option<String>,
    pub redemption_status: String,
    pub code_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub is_used: bool,
    pub merchant_name: Option<String>,
}

impl UserRedemption {
    pub fn can_be_cancelled(&self) -> bool {
        self.redemption_status == "pending"
    }

    pub fn is_active(&self) -> bool {
        self.redemption_status == "pending" && self.code_expires_at > Utc::now()
    }

    pub fn can_be_validated(&self) -> bool {
        self.redemption_status == "pending" 
            && self.code_expires_at > Utc::now() 
            && !self.is_used
    }
}

/// Item de redención para mostrar al usuario
/// Los campos sensibles (código, QR) se ocultan para redenciones usadas/expiradas
#[derive(Debug, Serialize, Deserialize)]
pub struct UserRedemptionItem {
    pub redemption_id: Uuid,
    pub offer_name: String,
    pub merchant_name: Option<String>,
    pub lumis_spent: i32,
    /// Código de redención - None si ya fue usado o expiró
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redemption_code: Option<String>,
    /// Código corto legible - None si ya fue usado o expiró
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_code: Option<String>,
    /// URL del QR - None si ya fue usado o expiró
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_landing_url: Option<String>,
    pub redemption_status: String,
    pub code_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
    /// Indica si el código QR está disponible para mostrar
    pub qr_visible: bool,
    /// Mensaje explicativo del estado
    pub status_message: String,
}

impl UserRedemptionItem {
    /// Crear item con visibilidad correcta según el estado
    pub fn new(
        redemption_id: Uuid,
        offer_name: String,
        merchant_name: Option<String>,
        lumis_spent: i32,
        redemption_code: String,
        short_code: Option<String>,
        qr_landing_url: String,
        redemption_status: String,
        code_expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
        validated_at: Option<DateTime<Utc>>,
    ) -> Self {
        let is_pending = redemption_status == "pending";
        let is_expired = code_expires_at < Utc::now();
        let qr_visible = is_pending && !is_expired;
        
        let status_message = match redemption_status.as_str() {
            "pending" if is_expired => "Código expirado".to_string(),
            "pending" => "Presenta este código en el comercio".to_string(),
            "confirmed" => format!(
                "Canjeado el {}",
                validated_at.map(|d| d.format("%d/%m/%Y %H:%M").to_string())
                    .unwrap_or_else(|| "fecha desconocida".to_string())
            ),
            "cancelled" => "Redención cancelada - Lümis devueltos".to_string(),
            "expired" => "Código expirado sin usar".to_string(),
            _ => "Estado desconocido".to_string(),
        };
        
        Self {
            redemption_id,
            offer_name,
            merchant_name,
            lumis_spent,
            // Ocultar código si no es visible
            redemption_code: if qr_visible { Some(redemption_code) } else { None },
            short_code: if qr_visible { short_code } else { None },
            qr_landing_url: if qr_visible { Some(qr_landing_url) } else { None },
            redemption_status,
            code_expires_at,
            created_at,
            validated_at,
            qr_visible,
            status_message,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserRedemptionStats {
    pub total_redemptions: i64,
    pub pending: i64,
    pub confirmed: i64,
    pub cancelled: i64,
    pub expired: i64,
    pub total_lumis_spent: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateRedemptionRequest {
    pub user_id: i32,
    pub offer_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RedemptionCreatedResponse {
    pub redemption_id: Uuid,
    pub redemption_code: String,
    pub short_code: String, // Nuevo campo
    pub offer_name: String,
    pub lumis_spent: i32,
    pub qr_landing_url: String,
    pub qr_image_url: Option<String>,
    pub code_expires_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub merchant_name: String,
    pub message: String,
    pub new_balance: i32,
}

#[derive(Debug, Serialize)]
pub struct CancellationResponse {
    pub success: bool,
    pub redemption_id: Uuid,
    pub lumis_refunded: i32,
    pub new_balance: i64,
    pub message: String,
}

// ======================================================================
// VALIDATION
// ======================================================================

#[derive(Debug, Serialize)]
pub struct OfferValidation {
    pub can_redeem: bool,
    pub reason: String,
    pub user_balance: i64,
    pub offer_cost: i32,
    pub user_redemptions_count: i32,
    pub max_allowed: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RedemptionValidation {
    pub can_redeem: bool,
    pub reason: String,
    pub user_balance: i32,
    pub offer_cost: Option<i32>,
    pub user_redemptions_count: Option<i32>,
    pub max_allowed: Option<i32>,
}

// ======================================================================
// MERCHANTS
// ======================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Merchant {
    pub merchant_id: Uuid,
    pub merchant_name: String,
    pub merchant_type: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub api_key_hash: String,
    pub webhook_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_redemptions: i32,
    pub total_lumis_redeemed: i64,
}

// ======================================================================
// AUDIT
// ======================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RedemptionAuditLog {
    pub log_id: i64,
    pub redemption_id: Uuid,
    pub action_type: String,
    pub performed_by: Option<String>,
    pub merchant_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<Uuid>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditActionType {
    Created,
    Validated,
    Confirmed,
    Cancelled,
    Expired,
}

impl AuditActionType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Validated => "validated",
            Self::Confirmed => "confirmed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }
}

// ======================================================================
// ERRORS
// ======================================================================

#[derive(Debug, thiserror::Error)]
pub enum RedemptionError {
    #[error("Saldo insuficiente. Tienes {current} Lümis, necesitas {required}")]
    InsufficientBalance { current: i64, required: i32 },

    #[error("Oferta no encontrada")]
    OfferNotFound,

    #[error("Oferta expirada o inactiva")]
    OfferInactive,

    #[error("Sin stock disponible")]
    OutOfStock,

    #[error("Límite de redenciones alcanzado. Máximo: {max}, actual: {current}")]
    MaxRedemptionsReached { max: i32, current: i32 },

    #[error("Redención no encontrada")]
    RedemptionNotFound,

    #[error("No puedes cancelar una redención {status}")]
    CannotCancel { status: String },

    #[error("Código de redención inválido o ya usado")]
    InvalidRedemptionCode,

    #[error("Token de validación expirado o inválido")]
    InvalidValidationToken,

    #[error("Redención ya confirmada anteriormente")]
    AlreadyConfirmed,

    #[error("Código expirado")]
    CodeExpired,

    #[error("Error de base de datos: {0}")]
    Database(String),

    #[error("Error interno: {0}")]
    Internal(String),

    #[error("Error de validación QR: {0}")]
    QRGenerationFailed(String),
}

impl From<sqlx::Error> for RedemptionError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

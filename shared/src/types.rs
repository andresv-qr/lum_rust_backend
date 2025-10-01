//! Common types used across all microservices

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Application sources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppSource {
    #[serde(rename = "whatsapp")]
    WhatsApp,
    #[serde(rename = "telegram")]
    Telegram,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "api")]
    Api,
}

impl std::fmt::Display for AppSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppSource::WhatsApp => write!(f, "whatsapp"),
            AppSource::Telegram => write!(f, "telegram"),
            AppSource::Email => write!(f, "email"),
            AppSource::Api => write!(f, "api"),
        }
    }
}

/// Message types for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    Image,
    Document,
    Interactive,
    CallbackQuery,
    PollAnswer,
}

/// QR Detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrDetectionResult {
    pub success: bool,
    pub data: Option<String>,
    pub detector_model: Option<String>,
    pub processing_time_ms: u64,
    pub confidence: Option<f32>,
}

/// QR Detection request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrDetectionRequest {
    pub image_data: String, // Base64 encoded image
}

/// QR Detection response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrDetectionResponse {
    pub success: bool,
    pub data: Option<String>,
    pub detector_model: Option<String>,
    pub confidence: Option<f32>,
}

/// OCR Processing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRequest {
    pub user_id: i32,
    pub image_data: Vec<u8>,
    pub source: AppSource,
    pub message_id: Option<String>,
}

/// OCR Processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub success: bool,
    pub extracted_text: Option<String>,
    pub confidence: Option<f32>,
    pub processing_time_ms: u64,
    pub cost_lumis: i32,
}

/// Rewards accumulation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsAccumulationRequest {
    pub user_id: i32,
    pub accum_type: String,
    pub context: serde_json::Value,
    pub accum_id: Option<String>,
}

/// Rewards accumulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsAccumulationResult {
    pub success: bool,
    pub points_earned: i32,
    pub new_balance: i32,
    pub message: String,
}

/// Rewards redemption request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsRedemptionRequest {
    pub user_id: i32,
    pub redem_id: String,
    pub quantity: i32,
    pub conditions: Option<serde_json::Value>,
}

/// Rewards redemption result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsRedemptionResult {
    pub success: bool,
    pub new_balance: i32,
    pub message: String,
}

/// User registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationRequest {
    pub user_id: String,
    pub source: AppSource,
    pub email: String,
    pub password: Option<String>,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub email: String,
    pub ws_id: Option<String>,
    pub telegram_id: Option<String>,
    pub registration_date: DateTime<Utc>,
    pub is_active: bool,
}

/// Invoice header data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceHeader {
    pub cufe: String,
    pub issuer_name: String,
    pub no: String,
    pub date: DateTime<Utc>,
    pub tot_amount: Option<Decimal>,
    pub tot_itbms: Option<Decimal>,
    pub user_id: i32,
    pub source: AppSource,
    pub url: String,
    pub process_date: DateTime<Utc>,
    pub reception_date: DateTime<Utc>,
}

/// Invoice detail data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceDetail {
    pub cufe: String,
    pub partkey: String,
    pub code: String,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub itbms: Decimal,
    pub total: Decimal,
}

/// Notification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub user_id: String,
    pub source: AppSource,
    pub message: String,
    pub message_type: MessageType,
    pub reply_to_message_id: Option<String>,
    pub keyboard: Option<serde_json::Value>,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service: String,
    pub status: ServiceStatus,
    pub version: String,
    pub uptime_seconds: u64,
    pub dependencies: Vec<DependencyStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub name: String,
    pub status: ServiceStatus,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: u32,
    pub limit: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self { page: 1, limit: 20 }
    }
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// API Response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            timestamp: Utc::now(),
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
            timestamp: Utc::now(),
        }
    }
}

/// Request ID for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestId(pub Uuid);

impl RequestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
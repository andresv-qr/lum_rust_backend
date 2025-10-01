//! Database models and DTOs

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub ws_id: Option<String>,
    pub telegram_id: Option<String>,
    pub password: Option<String>,
    pub email_registration_date: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Invoice header model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceHeaderModel {
    pub cufe: String,
    pub issuer_name: String,
    pub no: String,
    pub user_phone_number: Option<String>,
    pub user_telegram_id: Option<String>,
    pub time: String,
    pub receptor_address: Option<String>,
    pub tot_itbms: Option<Decimal>,
    pub issuer_dv: Option<String>,
    pub receptor_phone: Option<String>,
    pub user_email: Option<String>,
    pub auth_date: Option<String>,
    pub date: DateTime<Utc>,
    pub receptor_id: Option<String>,
    pub issuer_address: Option<String>,
    pub issuer_ruc: Option<String>,
    pub tot_amount: Option<Decimal>,
    pub receptor_name: Option<String>,
    pub receptor_dv: Option<String>,
    pub issuer_phone: Option<String>,
    pub r#type: String,
    pub origin: String,
    pub user_ws: Option<String>,
    pub user_id: i32,
    pub url: String,
    pub process_date: DateTime<Utc>,
    pub reception_date: DateTime<Utc>,
}

/// Invoice detail model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceDetailModel {
    pub date: String,
    pub cufe: String,
    pub partkey: String,
    pub code: String,
    pub description: String,
    pub information_of_interest: Option<String>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub unit_discount: Decimal,
    pub amount: Decimal,
    pub itbms: Decimal,
    pub total: Decimal,
}

/// Rewards accumulation model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RewardsAccumulation {
    pub id: i32,
    pub user_id: i32,
    pub accum_type: String,
    pub accum_id: String,
    pub dtype: String,
    pub quantity: i32,
    pub date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Rewards redemption model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RewardsRedemption {
    pub id: i32,
    pub user_id: i32,
    pub redem_id: String,
    pub redem_type: String,
    pub redem_key: Option<String>,
    pub dtype: String,
    pub quantity: i32,
    pub date: DateTime<Utc>,
    pub condition1: Option<String>,
    pub condition2: Option<String>,
    pub condition3: Option<String>,
    pub expiration_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// User balance model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserBalance {
    pub user_id: i32,
    pub balance: i32,
    pub last_updated: DateTime<Utc>,
}

/// User interaction log model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserInteraction {
    pub id: i32,
    pub user_id: String,
    pub source: String,
    pub interaction_type: String,
    pub interaction_data: serde_json::Value,
    pub interaction_action: Option<String>,
    pub text: Option<String>,
    pub bot_response: Option<String>,
    pub start_timestamp: DateTime<Utc>,
    pub end_timestamp: DateTime<Utc>,
    pub total_duration: Decimal,
    pub created_at: DateTime<Utc>,
}

/// Trivia model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Trivia {
    pub id: i32,
    pub question: String,
    pub options: serde_json::Value,
    pub correct_answer: String,
    pub category: String,
    pub difficulty: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Survey model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Survey {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub questions: serde_json::Value,
    pub is_active: bool,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Product search model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductSearch {
    pub id: i32,
    pub user_id: i32,
    pub search_term: String,
    pub search_type: String,
    pub results_count: i32,
    pub created_at: DateTime<Utc>,
}

/// OCR processing log model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OcrProcessingLog {
    pub id: i32,
    pub user_id: i32,
    pub image_hash: String,
    pub extracted_text: Option<String>,
    pub confidence: Option<Decimal>,
    pub processing_time_ms: i32,
    pub cost_lumis: i32,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// QR detection log model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QrDetectionLog {
    pub id: i32,
    pub user_id: Option<i32>,
    pub image_hash: String,
    pub detected_data: Option<String>,
    pub detector_model: Option<String>,
    pub processing_time_ms: i32,
    pub confidence: Option<Decimal>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Notification log model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationLog {
    pub id: i32,
    pub user_id: String,
    pub source: String,
    pub message: String,
    pub message_type: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub sent_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Rate limit model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RateLimit {
    pub id: i32,
    pub identifier: String,
    pub request_count: i32,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Service health model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceHealthModel {
    pub id: i32,
    pub service_name: String,
    pub status: String,
    pub version: String,
    pub uptime_seconds: i64,
    pub last_check: DateTime<Utc>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTOs for API requests/responses

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub ws_id: Option<String>,
    pub telegram_id: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub ws_id: Option<String>,
    pub telegram_id: Option<String>,
    pub is_active: bool,
    pub registration_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceHeaderResponse {
    pub cufe: String,
    pub issuer_name: String,
    pub no: String,
    pub date: DateTime<Utc>,
    pub tot_amount: Option<Decimal>,
    pub tot_itbms: Option<Decimal>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSearchRequest {
    pub user_id: i32,
    pub search_term: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSearchResponse {
    pub description: String,
    pub code: String,
    pub issuer: String,
    pub date_invoice: DateTime<Utc>,
    pub unit_price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub cufe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriviaResponse {
    pub id: i32,
    pub question: String,
    pub options: Vec<String>,
    pub category: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriviaAnswerRequest {
    pub trivia_id: i32,
    pub user_id: i32,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriviaAnswerResponse {
    pub correct: bool,
    pub correct_answer: String,
    pub points_earned: i32,
}

/// Conversion implementations
impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            ws_id: user.ws_id,
            telegram_id: user.telegram_id,
            is_active: user.is_active,
            registration_date: user.email_registration_date,
        }
    }
}

impl From<InvoiceHeaderModel> for InvoiceHeaderResponse {
    fn from(invoice: InvoiceHeaderModel) -> Self {
        Self {
            cufe: invoice.cufe,
            issuer_name: invoice.issuer_name,
            no: invoice.no,
            date: invoice.date,
            tot_amount: invoice.tot_amount,
            tot_itbms: invoice.tot_itbms,
            url: invoice.url,
        }
    }
}
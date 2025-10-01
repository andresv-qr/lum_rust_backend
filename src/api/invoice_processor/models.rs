use crate::models::invoice::{InvoiceHeader, InvoiceDetail, InvoicePayment};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInvoiceRequest {
    pub url: String,
    pub user_id: String,
    pub user_email: String,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInvoiceResponse {
    pub status: String,
    pub message: String,
    pub data: Option<InvoiceResponseData>,
    pub error: Option<ErrorDetails>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResponseData {
    pub cufe: String,
    pub invoice_number: String,
    pub issuer_name: String,
    pub tot_amount: f64,
    pub items_count: usize,
    pub processed_date: Option<DateTime<Utc>>,
    pub original_user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    #[serde(rename = "type")]
    pub error_type: String,
    pub details: String,
    pub retry_attempts: Option<u32>,
}

// ============================================================================
// DATABASE & SERVICE MODELS
// ============================================================================

/// This struct holds the full invoice data, combining the extracted data
/// with the request metadata. It uses the canonical InvoiceHeader model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullInvoiceData {
    pub header: InvoiceHeader,
    pub details: Vec<InvoiceDetail>,
    pub payment: InvoicePayment,
}

// This struct was previously named InvoiceData and was causing conflicts.
// It now correctly represents only the metadata from the incoming request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    pub reception_date: DateTime<Utc>,
    pub user_id: i64,
    pub origin: String,
    pub user_email: String,
}

// ============================================================================
// LOGGING MODELS (logs.bot_rust_scrapy table)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotLog {
    pub id: Option<i32>,               // SERIAL PRIMARY KEY
    pub url: String,                   // URL procesada
    pub cufe: Option<String>,          // CUFE extraído (si exitoso)
    pub origin: String,                // Origen de la solicitud
    pub user_id: String,               // ID del usuario solicitante
    pub user_email: String,            // Email del usuario
    pub execution_time_ms: Option<i32>, // Tiempo de ejecución del scraping (ms)
    pub status: String,                // Estado final de la operación
    pub error_message: Option<String>, // Mensaje de error detallado
    pub error_type: Option<String>,    // Tipo de error categorizado
    pub request_timestamp: DateTime<Utc>, // Timestamp de recepción
    pub response_timestamp: Option<DateTime<Utc>>, // Timestamp de respuesta
    pub scraped_fields_count: Option<i32>, // Número de campos extraídos exitosamente
    pub retry_attempts: Option<i32>,   // Número de intentos de retry
}

// ============================================================================
// INTERNAL PROCESSING MODELS
// ============================================================================

#[derive(Debug, Clone)]
pub struct ProcessingContext {
    pub request: ProcessInvoiceRequest,
    pub calculated_type: String,
    pub reception_date: DateTime<Utc>,
    pub process_date: DateTime<Utc>,
    pub log_entry: BotLog,
}

#[derive(Debug, Clone)]
pub struct ScrapingMetrics {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub retry_attempts: u32,
    pub fields_extracted: u32,
}

// ============================================================================
// STATUS AND ERROR TYPE ENUMS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogStatus {
    Success,
    Duplicate,
    ValidationError,
    ScrapingError,
    DatabaseError,
    TimeoutError,
    NetworkError,
}

impl LogStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogStatus::Success => "SUCCESS",
            LogStatus::Duplicate => "DUPLICATE",
            LogStatus::ValidationError => "VALIDATION_ERROR",
            LogStatus::ScrapingError => "SCRAPING_ERROR",
            LogStatus::DatabaseError => "DATABASE_ERROR",
            LogStatus::TimeoutError => "TIMEOUT_ERROR",
            LogStatus::NetworkError => "NETWORK_ERROR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    InvalidUrl,
    MissingFields,
    CufeNotFound,
    HtmlParseError,
    DbConnectionError,
    DbTransactionError,
    Timeout,
    Other,
    Unknown,
}

impl ErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorType::InvalidUrl => "INVALID_URL",
            ErrorType::MissingFields => "MISSING_FIELDS",
            ErrorType::CufeNotFound => "CUFE_NOT_FOUND",
            ErrorType::HtmlParseError => "HTML_PARSE_ERROR",
            ErrorType::DbConnectionError => "DB_CONNECTION_ERROR",
            ErrorType::DbTransactionError => "DB_TRANSACTION_ERROR",
            ErrorType::Timeout => "TIMEOUT",
            ErrorType::Other => "OTHER",
            ErrorType::Unknown => "UNKNOWN",
        }
    }
}

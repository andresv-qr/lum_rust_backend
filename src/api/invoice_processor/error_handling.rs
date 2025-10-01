use crate::api::invoice_processor::models::ErrorType;
use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde_json::json;
use thiserror::Error;

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Error, Debug)]
pub enum InvoiceProcessingError {
    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Scraping failed: {message}")]
    ScrapingError {
        message: String,
        error_type: ErrorType,
        retry_attempts: u32,
    },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Duplicate invoice found: {cufe}")]
    DuplicateInvoice { cufe: String },

    #[error("Timeout after {attempts} attempt(s)")]
    TimeoutError { attempts: u32 },

    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Data parsing error: {0}")]
    DataParsingError(String),
}

// ============================================================================
// HTTP RESPONSE CONVERSION
// ============================================================================

impl IntoResponse for InvoiceProcessingError {
    fn into_response(self) -> Response {
        let (status, error_message, error_details) = match self {
            InvoiceProcessingError::ValidationError { message } => (
                StatusCode::BAD_REQUEST,
                "Datos de entrada inválidos".to_string(),
                json!({ "type": "VALIDATION_ERROR", "details": message }),
            ),
            InvoiceProcessingError::ScrapingError { message, error_type, retry_attempts } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Su factura no pudo ser procesada".to_string(),
                json!({
                    "type": error_type,
                    "details": message,
                    "retry_attempts": retry_attempts
                }),
            ),
            InvoiceProcessingError::DatabaseError { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error en la base de datos".to_string(),
                json!({ "type": "DATABASE_ERROR", "details": message }),
            ),
            InvoiceProcessingError::DuplicateInvoice { cufe } => (
                StatusCode::CONFLICT,
                "Esta factura ya fue procesada anteriormente".to_string(),
                json!({ "type": "DUPLICATE", "cufe": cufe }),
            ),
            InvoiceProcessingError::TimeoutError { attempts } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "El procesamiento de la factura excedió el tiempo límite".to_string(),
                json!({ "type": "TIMEOUT_ERROR", "attempts": attempts }),
            ),
            InvoiceProcessingError::NetworkError(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error de red durante el procesamiento".to_string(),
                json!({ "type": "NETWORK_ERROR", "details": message }),
            ),
            InvoiceProcessingError::DataParsingError(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error parseando los datos extraídos".to_string(),
                json!({ "type": "DATA_PARSING_ERROR", "details": message }),
            ),
        };

        let body = Json(json!({
            "status": "error",
            "message": error_message,
            "error": error_details
        }));

        (status, body).into_response()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

pub fn create_success_response(
    cufe: String,
    invoice_number: String,
    issuer_name: String,
    tot_amount: f64,
    items_count: usize,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "success",
        "message": format!(
            "Su factura de {} por valor de ${} fue procesada exitosamente.",
            issuer_name, tot_amount
        ),
        "data": {
            "cufe": cufe,
            "invoice_number": invoice_number,
            "issuer_name": issuer_name,
            "tot_amount": tot_amount,
            "items_count": items_count
        }
    }))
}

// ============================================================================
// CONVERSION FROM OTHER ERROR TYPES
// ============================================================================

impl From<sqlx::Error> for InvoiceProcessingError {
    fn from(err: sqlx::Error) -> Self {
        InvoiceProcessingError::DatabaseError {
            message: format!("Database error: {}", err),
        }
    }
}

impl From<reqwest::Error> for InvoiceProcessingError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            InvoiceProcessingError::TimeoutError { attempts: 1 }
        } else {
            InvoiceProcessingError::NetworkError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    
    #[test]
    fn test_validation_error_response() {
        let error = InvoiceProcessingError::ValidationError {
            message: "URL inválida".to_string(),
        };
        
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[test]
    fn test_duplicate_invoice_response() {
        let error = InvoiceProcessingError::DuplicateInvoice {
            cufe: "FE012000...".to_string(),
        };
        
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
    
    #[test]
    fn test_scraping_error_response() {
        let error = InvoiceProcessingError::ScrapingError {
            message: "CUFE no encontrado".to_string(),
            error_type: ErrorType::CufeNotFound,
            retry_attempts: 2,
        };
        
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    #[test]
    fn test_success_response_creation() {
        let response = create_success_response(
            "FE012000...".to_string(),
            "001234".to_string(),
            "Test Company".to_string(),
            100.00,
            3,
        );
        
        assert_eq!(response.status, "success");
        assert!(response.message.contains("Test Company"));
        assert!(response.message.contains("$100.00"));
        assert!(response.data.is_some());
    }
}

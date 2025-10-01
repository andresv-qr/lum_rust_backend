use thiserror::Error;
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use crate::api::invoices::models::{ProcessInvoiceResponse, ErrorDetails, ErrorType};

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Error, Debug)]
pub enum InvoiceProcessingError {
    #[error("Validation error: {errors:?}")]
    ValidationError { errors: Vec<String> },
    
    #[error("Invoice already exists: {cufe}")]
    DuplicateInvoice { cufe: String, original_user: Option<String>, processed_date: Option<chrono::DateTime<chrono::Utc>> },
    
    #[error("Scraping error: {message}")]
    ScrapingError { message: String, error_type: ErrorType, retry_attempts: u32 },
    
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    
    #[error("Timeout error after {attempts} attempts")]
    TimeoutError { attempts: u32 },
    
    #[error("Network error: {message}")]
    NetworkError { message: String },
    
    #[error("Internal server error: {message}")]
    InternalError { message: String },
}

// ============================================================================
// HTTP RESPONSE CONVERSION
// ============================================================================

impl IntoResponse for InvoiceProcessingError {
    fn into_response(self) -> Response {
        let (status, response) = match self {
            InvoiceProcessingError::ValidationError { errors } => {
                (
                    StatusCode::BAD_REQUEST,
                    ProcessInvoiceResponse {
                        status: "validation_error".to_string(),
                        message: "Datos de entrada inválidos".to_string(),
                        data: None,
                        error: None,
                        errors: Some(errors),
                    }
                )
            },
            
            InvoiceProcessingError::DuplicateInvoice { cufe, original_user, processed_date } => {
                use crate::api::invoices::models::InvoiceResponseData;
                
                (
                    StatusCode::CONFLICT,
                    ProcessInvoiceResponse {
                        status: "duplicate".to_string(),
                        message: "Esta factura ya fue procesada anteriormente".to_string(),
                        data: Some(InvoiceResponseData {
                            cufe,
                            invoice_number: "".to_string(), // Will be populated if needed
                            issuer_name: "".to_string(),    // Will be populated if needed
                            tot_amount: "".to_string(),     // Will be populated if needed
                            items_count: 0,                 // Will be populated if needed
                            processed_date,
                            original_user,
                        }),
                        error: None,
                        errors: None,
                    }
                )
            },
            
            InvoiceProcessingError::ScrapingError { message, error_type, retry_attempts } => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ProcessInvoiceResponse {
                        status: "processing_error".to_string(),
                        message: "Su factura no pudo ser procesada".to_string(),
                        data: None,
                        error: Some(ErrorDetails {
                            error_type: error_type.as_str().to_string(),
                            details: message,
                            retry_attempts: Some(retry_attempts),
                        }),
                        errors: None,
                    }
                )
            },
            
            InvoiceProcessingError::DatabaseError { message } => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ProcessInvoiceResponse {
                        status: "processing_error".to_string(),
                        message: "Error interno del sistema".to_string(),
                        data: None,
                        error: Some(ErrorDetails {
                            error_type: ErrorType::DbConnectionError.as_str().to_string(),
                            details: message,
                            retry_attempts: None,
                        }),
                        errors: None,
                    }
                )
            },
            
            InvoiceProcessingError::TimeoutError { attempts } => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ProcessInvoiceResponse {
                        status: "processing_error".to_string(),
                        message: "Timeout procesando la factura".to_string(),
                        data: None,
                        error: Some(ErrorDetails {
                            error_type: ErrorType::Timeout.as_str().to_string(),
                            details: format!("Timeout después de {} intentos", attempts),
                            retry_attempts: Some(attempts),
                        }),
                        errors: None,
                    }
                )
            },
            
            InvoiceProcessingError::NetworkError { message } => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ProcessInvoiceResponse {
                        status: "processing_error".to_string(),
                        message: "Error de conexión".to_string(),
                        data: None,
                        error: Some(ErrorDetails {
                            error_type: "NETWORK_ERROR".to_string(),
                            details: message,
                            retry_attempts: None,
                        }),
                        errors: None,
                    }
                )
            },
            
            InvoiceProcessingError::InternalError { message } => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ProcessInvoiceResponse {
                        status: "processing_error".to_string(),
                        message: "Error interno del sistema".to_string(),
                        data: None,
                        error: Some(ErrorDetails {
                            error_type: ErrorType::Unknown.as_str().to_string(),
                            details: message,
                            retry_attempts: None,
                        }),
                        errors: None,
                    }
                )
            },
        };
        
        (status, Json(response)).into_response()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

pub fn create_success_response(
    cufe: String,
    invoice_number: String,
    issuer_name: String,
    tot_amount: String,
    items_count: usize,
) -> ProcessInvoiceResponse {
    use crate::api::invoices::models::InvoiceResponseData;
    
    ProcessInvoiceResponse {
        status: "success".to_string(),
        message: format!(
            "Su factura de {} por valor de ${} fue procesada exitosamente",
            issuer_name, tot_amount
        ),
        data: Some(InvoiceResponseData {
            cufe,
            invoice_number,
            issuer_name,
            tot_amount,
            items_count,
            processed_date: None,
            original_user: None,
        }),
        error: None,
        errors: None,
    }
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
            InvoiceProcessingError::NetworkError {
                message: format!("Network error: {}", err),
            }
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
            errors: vec!["URL inválida".to_string(), "Email requerido".to_string()],
        };
        
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    
    #[test]
    fn test_duplicate_invoice_response() {
        let error = InvoiceProcessingError::DuplicateInvoice {
            cufe: "FE012000...".to_string(),
            original_user: Some("user123".to_string()),
            processed_date: None,
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
            "100.00".to_string(),
            3,
        );
        
        assert_eq!(response.status, "success");
        assert!(response.message.contains("Test Company"));
        assert!(response.message.contains("$100.00"));
        assert!(response.data.is_some());
    }
}

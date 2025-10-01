//! Error handling for all microservices

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Authentication error: {message}")]
    Authentication { message: String },

    #[error("Authorization error: {message}")]
    Authorization { message: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    #[error("Processing error: {message}")]
    Processing { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Timeout error: {operation}")]
    Timeout { operation: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

impl AppError {
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::RateLimit {
            message: message.into(),
        }
    }

    pub fn service_unavailable(service: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
        }
    }

    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    pub fn processing(message: impl Into<String>) -> Self {
        Self::Processing {
            message: message.into(),
        }
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }

    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            AppError::Authorization { .. } => StatusCode::FORBIDDEN,
            AppError::Validation { .. } => StatusCode::BAD_REQUEST,
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Conflict { .. } => StatusCode::CONFLICT,
            AppError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            AppError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ExternalService { .. } => StatusCode::BAD_GATEWAY,
            AppError::Processing { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Configuration { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::HttpClient(_) => StatusCode::BAD_GATEWAY,
            AppError::Jwt(_) => StatusCode::UNAUTHORIZED,
            AppError::Uuid(_) => StatusCode::BAD_REQUEST,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Generic(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Authentication { .. } => "AUTH_ERROR",
            AppError::Authorization { .. } => "AUTHZ_ERROR",
            AppError::Validation { .. } => "VALIDATION_ERROR",
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::Conflict { .. } => "CONFLICT",
            AppError::RateLimit { .. } => "RATE_LIMIT",
            AppError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            AppError::BadRequest { .. } => "BAD_REQUEST",
            AppError::Timeout { .. } => "TIMEOUT",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Redis(_) => "CACHE_ERROR",
            AppError::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            AppError::Processing { .. } => "PROCESSING_ERROR",
            AppError::Configuration { .. } => "CONFIG_ERROR",
            AppError::Internal { .. } => "INTERNAL_ERROR",
            AppError::Serialization(_) => "SERIALIZATION_ERROR",
            AppError::HttpClient(_) => "HTTP_CLIENT_ERROR",
            AppError::Jwt(_) => "JWT_ERROR",
            AppError::Uuid(_) => "UUID_ERROR",
            AppError::Io(_) => "IO_ERROR",
            AppError::Generic(_) => "GENERIC_ERROR",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = ErrorResponse {
            error: self.error_code().to_string(),
            message: self.to_string(),
            code: status.as_u16().to_string(),
            details: None,
        };

        tracing::error!("API Error: {} - {}", self.error_code(), self);

        (status, Json(error_response)).into_response()
    }
}

// Helper macro for creating errors
#[macro_export]
macro_rules! app_error {
    ($variant:ident, $msg:expr) => {
        AppError::$variant {
            message: $msg.to_string(),
        }
    };
    ($variant:ident, $resource:expr) => {
        AppError::$variant {
            resource: $resource.to_string(),
        }
    };
}
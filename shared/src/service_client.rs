//! HTTP client for inter-service communication

use crate::{error::AppError, types::*, Result};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ServiceClient {
    client: Client,
    base_url: String,
    service_name: String,
    timeout: Duration,
}

impl ServiceClient {
    pub fn new(base_url: String, service_name: String, timeout_seconds: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| AppError::configuration(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            base_url,
            service_name,
            timeout: Duration::from_secs(timeout_seconds),
        })
    }

    /// Make a GET request
    pub async fn get<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Make a POST request
    pub async fn post<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Make a PUT request
    pub async fn put<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self
            .client
            .put(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Make a DELETE request
    pub async fn delete<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Make a request with authentication
    pub async fn get_with_auth<T>(&self, endpoint: &str, token: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    pub async fn post_with_auth<T, R>(&self, endpoint: &str, body: &T, token: &str) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Health check endpoint
    pub async fn health_check(&self) -> Result<HealthStatus> {
        self.get("/health").await
    }

    /// Handle HTTP response and deserialize
    async fn handle_response<T>(&self, response: Response) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let status = response.status();
        
        if status.is_success() {
            response
                .json::<T>()
                .await
                .map_err(|e| AppError::external_service(
                    &self.service_name,
                    format!("Failed to deserialize response: {}", e)
                ))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(AppError::external_service(
                &self.service_name,
                format!("HTTP {} - {}", status, error_text)
            ))
        }
    }

    /// Map reqwest errors to AppError
    fn map_reqwest_error(&self, error: reqwest::Error) -> AppError {
        if error.is_timeout() {
            AppError::timeout(format!("Request to {} timed out", self.service_name))
        } else if error.is_connect() {
            AppError::service_unavailable(&self.service_name)
        } else {
            AppError::external_service(&self.service_name, error.to_string())
        }
    }
}

/// Service-specific clients
pub struct QrDetectionClient {
    client: ServiceClient,
}

impl QrDetectionClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = ServiceClient::new(base_url, "qr-detection".to_string(), 30)?;
        Ok(Self { client })
    }

    pub async fn detect_qr(&self, image_data: &[u8]) -> Result<QrDetectionResult> {
        #[derive(Serialize)]
        struct QrRequest {
            image_data: String, // base64 encoded
        }

        use base64::{Engine as _, engine::general_purpose};
        let encoded_image = general_purpose::STANDARD.encode(image_data);
        let request = QrRequest {
            image_data: encoded_image,
        };

        self.client.post("/detect", &request).await
    }

    pub async fn health(&self) -> Result<HealthStatus> {
        self.client.health_check().await
    }
}

pub struct OcrProcessingClient {
    client: ServiceClient,
}

impl OcrProcessingClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = ServiceClient::new(base_url, "ocr-processing".to_string(), 60)?;
        Ok(Self { client })
    }

    pub async fn process_ocr(&self, request: &OcrRequest) -> Result<OcrResult> {
        self.client.post("/process", request).await
    }

    pub async fn health(&self) -> Result<HealthStatus> {
        self.client.health_check().await
    }
}

pub struct RewardsEngineClient {
    client: ServiceClient,
}

impl RewardsEngineClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = ServiceClient::new(base_url, "rewards-engine".to_string(), 30)?;
        Ok(Self { client })
    }

    pub async fn process_accumulation(&self, request: &RewardsAccumulationRequest) -> Result<RewardsAccumulationResult> {
        self.client.post("/accumulate", request).await
    }

    pub async fn process_redemption(&self, request: &RewardsRedemptionRequest) -> Result<RewardsRedemptionResult> {
        self.client.post("/redeem", request).await
    }

    pub async fn get_user_balance(&self, user_id: i32) -> Result<i32> {
        #[derive(Deserialize)]
        struct BalanceResponse {
            balance: i32,
        }

        let response: BalanceResponse = self.client.get(&format!("/balance/{}", user_id)).await?;
        Ok(response.balance)
    }

    pub async fn health(&self) -> Result<HealthStatus> {
        self.client.health_check().await
    }
}

pub struct UserManagementClient {
    client: ServiceClient,
}

impl UserManagementClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = ServiceClient::new(base_url, "user-management".to_string(), 30)?;
        Ok(Self { client })
    }

    pub async fn register_user(&self, request: &UserRegistrationRequest) -> Result<UserInfo> {
        self.client.post("/register", request).await
    }

    pub async fn get_user(&self, user_id: &str, source: &AppSource) -> Result<Option<UserInfo>> {
        #[derive(Serialize)]
        struct UserQuery {
            user_id: String,
            source: AppSource,
        }

        let query = UserQuery {
            user_id: user_id.to_string(),
            source: source.clone(),
        };

        self.client.post("/user/lookup", &query).await
    }

    pub async fn health(&self) -> Result<HealthStatus> {
        self.client.health_check().await
    }
}

pub struct NotificationClient {
    client: ServiceClient,
}

impl NotificationClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = ServiceClient::new(base_url, "notification".to_string(), 30)?;
        Ok(Self { client })
    }

    pub async fn send_notification(&self, request: &NotificationRequest) -> Result<()> {
        #[derive(Deserialize)]
        struct NotificationResponse {
            success: bool,
        }

        let response: NotificationResponse = self.client.post("/send", request).await?;
        
        if response.success {
            Ok(())
        } else {
            Err(AppError::external_service("notification", "Failed to send notification"))
        }
    }

    pub async fn health(&self) -> Result<HealthStatus> {
        self.client.health_check().await
    }
}
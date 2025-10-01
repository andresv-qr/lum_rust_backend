use anyhow::{anyhow, Context, Result};
use image::DynamicImage;
use serde::Deserialize;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Response from Python QReader hybrid fallback service
#[derive(Debug, Deserialize)]
pub struct PythonQrResponse {
    pub success: bool,
    pub qr_data: Option<String>,
    pub detector_model: Option<String>,
    pub pipeline: Option<String>,
    pub methods_tried: Option<Vec<String>>,
    pub message: Option<String>,
}

/// Statistics from Python QReader service
#[derive(Debug, Deserialize)]
pub struct PythonQrStats {
    pub requests_processed: u64,
    pub successful_detections: u64,
    pub failed_detections: u64,
    pub small_model_success: u64,
    pub large_model_success: u64,
    pub total_processing_time: f64,
    pub uptime_seconds: f64,
    pub avg_processing_time: f64,
    pub success_rate: f64,
}

/// Client metrics for monitoring
#[derive(Debug, Default)]
pub struct ClientMetrics {
    pub requests_sent: AtomicU64,
    pub responses_received: AtomicU64,
    pub connection_errors: AtomicU64,
    pub timeout_errors: AtomicU64,
    pub total_latency_ms: AtomicU64,
}

impl ClientMetrics {
    pub fn record_request(&self, latency: Duration, success: bool) {
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.responses_received.fetch_add(1, Ordering::Relaxed);
        }
        
        self.total_latency_ms.fetch_add(
            latency.as_millis() as u64,
            Ordering::Relaxed,
        );
    }
    
    pub fn record_connection_error(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_timeout(&self) {
        self.timeout_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self) -> (u64, u64, u64, u64, f64) {
        let requests = self.requests_sent.load(Ordering::Relaxed);
        let responses = self.responses_received.load(Ordering::Relaxed);
        let conn_errors = self.connection_errors.load(Ordering::Relaxed);
        let timeouts = self.timeout_errors.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);
        
        let avg_latency = if requests > 0 {
            total_latency as f64 / requests as f64
        } else {
            0.0
        };
        
        (requests, responses, conn_errors, timeouts, avg_latency)
    }
}

/// Python QReader client for hybrid fallback QR detection via HTTP
pub struct PythonQReaderClient {
    base_url: String,
    client: reqwest::Client,
    metrics: Arc<ClientMetrics>,
    timeout: Duration,
    max_image_size: usize,
}

impl PythonQReaderClient {
    /// Create a new Python QReader client for HTTP requests
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
            
        info!("🔗 Initializing Python QReader HTTP client for: {}", base_url);
            
        Self {
            base_url,
            client,
            metrics: Arc::new(ClientMetrics::default()),
            timeout: Duration::from_secs(10),
            max_image_size: 10 * 1024 * 1024, // 10MB max image size
        }
    }
    
    /// Set timeout for requests
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set maximum image size
    pub fn with_max_image_size(mut self, max_size: usize) -> Self {
        self.max_image_size = max_size;
        self
    }
    
    /// Get client metrics
    pub fn get_metrics(&self) -> Arc<ClientMetrics> {
        self.metrics.clone()
    }
    
    /// Check if Python service is available via health check
    pub async fn is_available(&self) -> bool {
        let health_url = format!("{}/health", self.base_url);
        
        debug!("🔍 Checking Python service availability at: {}", health_url);
        
        match tokio::time::timeout(
            Duration::from_secs(5),
            self.client.get(&health_url).send()
        ).await {
            Ok(Ok(response)) => {
                let available = response.status().is_success();
                if available {
                    info!("✅ Python fallback service is available at {}", self.base_url);
                } else {
                    warn!("❌ Python fallback service returned error: {}", response.status());
                }
                available
            }
            Ok(Err(e)) => {
                warn!("❌ Python fallback service connection error: {}", e);
                false
            }
            Err(_) => {
                warn!("❌ Python fallback service health check timed out");
                false
            }
        }
    }
    

    
    /// Detect QR code using Python QReader hybrid fallback service
    pub async fn detect_qr(&self, image: &DynamicImage) -> Result<Option<String>> {
        let start_time = Instant::now();
        
        // Convert image to PNG bytes
        let mut image_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut image_bytes);
        
        image.write_to(&mut cursor, image::ImageFormat::Png)
            .context("Failed to encode image as PNG")?;
        
        // Check image size
        if image_bytes.len() > self.max_image_size {
            return Err(anyhow!(
                "Image too large: {} bytes (max: {} bytes)",
                image_bytes.len(),
                self.max_image_size
            ));
        }
        
        info!(
            "🔄 Sending image to Python hybrid fallback ({} bytes) at {}",
            image_bytes.len(),
            self.base_url
        );
        
        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(image_bytes)
                .file_name("qr_image.png")
                .mime_str("image/png")?);
        
        let url = format!("{}/qr/hybrid-fallback", self.base_url);
        
        // Send request with timeout
        let result = tokio::time::timeout(
            self.timeout,
            self.client.post(&url).multipart(form).send()
        ).await;
        
        let response = match result {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                error!("❌ Python hybrid fallback request failed: {}", e);
                self.metrics.record_connection_error();
                self.metrics.record_request(start_time.elapsed(), false);
                return Err(e.into());
            }
            Err(_) => {
                warn!("⏰ Python hybrid fallback request timed out after {:?}", self.timeout);
                self.metrics.record_timeout();
                self.metrics.record_request(start_time.elapsed(), false);
                return Err(anyhow!("Request timed out"));
            }
        };
        
        let latency = start_time.elapsed();
        
        if !response.status().is_success() {
            error!("❌ Python hybrid fallback returned error: {}", response.status());
            self.metrics.record_request(latency, false);
            return Err(anyhow!("HTTP error: {}", response.status()));
        }
        
        let python_response: PythonQrResponse = response.json().await
            .context("Failed to parse Python response as JSON")?;
        
        if python_response.success {
            let content = python_response.qr_data.clone();
            info!(
                "✅ Python hybrid fallback detected QR in {:.2}ms using {}: {}",
                latency.as_millis(),
                python_response.detector_model.as_deref().unwrap_or("unknown"),
                content.as_deref().unwrap_or("None")
            );
            self.metrics.record_request(latency, true);
            Ok(content)
        } else {
            debug!(
                "❌ Python hybrid fallback failed to detect QR: {} (tried: {:?})",
                python_response.message.as_deref().unwrap_or("No message"),
                python_response.methods_tried
            );
            self.metrics.record_request(latency, true); // Request succeeded, just no QR found
            Ok(None)
        }
    }
    
    /// Get statistics from Python service (placeholder - would need dedicated endpoint)
    pub async fn get_python_stats(&self) -> Result<PythonQrStats> {
        // This would need a dedicated stats endpoint in the Python service
        // For now, return dummy stats
        Ok(PythonQrStats {
            requests_processed: 0,
            successful_detections: 0,
            failed_detections: 0,
            small_model_success: 0,
            large_model_success: 0,
            total_processing_time: 0.0,
            uptime_seconds: 0.0,
            avg_processing_time: 0.0,
            success_rate: 0.0,
        })
    }
}

impl Drop for PythonQReaderClient {
    fn drop(&mut self) {
        // Log final metrics
        let (requests, responses, conn_errors, timeouts, avg_latency) = self.metrics.get_stats();
        info!(
            "📊 PythonQReaderClient final stats - Requests: {}, Responses: {}, Errors: {}, Timeouts: {}, Avg Latency: {:.2}ms",
            requests, responses, conn_errors, timeouts, avg_latency
        );
    }
}

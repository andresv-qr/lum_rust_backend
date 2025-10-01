use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error};

use crate::state::AppState;
use crate::services::rewards_service::OfferResult;
use crate::services::whatsapp_service;

#[derive(Debug, Serialize)]
pub struct OfferData {
    pub comercio: String,
    pub producto: String,
    pub codigo: Option<String>,
    pub precio_actual: f64,
    pub precio_anterior: f64,
    pub descuento_porc: Option<f64>,
    pub descuento_valor: f64,
    pub dias: i32,
}

#[derive(Debug, Serialize)]
pub struct DashboardRequest {
    pub offers: Vec<OfferData>,
    pub category: String,
    pub whatsapp_number: String,  // Changed from whatsapp_id to match Python API
    pub user_id: Option<i64>,
    pub callback_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DashboardResponse {
    pub task_id: String,
    pub status: String,
    pub message: String,
    pub estimated_completion: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskStatusResponse {
    pub task_id: String,
    pub status: String,
    pub progress: Option<i32>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    // Removed created_at and completed_at fields that are not in the API response
}

/// Service for generating visual dashboards via Python API
pub struct VisualDashboardService {
    client: Client,
    api_base_url: String,
}

impl VisualDashboardService {
    pub fn new() -> Self {
        let api_base_url = std::env::var("DASHBOARD_API_URL")
            .unwrap_or_else(|_| "http://localhost:8008".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_base_url,
        }
    }

    /// Generate offers dashboard asynchronously
    pub async fn generate_offers_dashboard(
        &self,
        app_state: &Arc<AppState>,
        offers: Vec<OfferResult>, // Use the real offers passed as parameter
        category: &str,
        whatsapp_id: &str,
        user_id: Option<i64>,
    ) -> Result<String> {
        info!("Requesting visual dashboard generation for category '{}', user {}", category, whatsapp_id);

        // Convert OfferResult to OfferData format for dashboard API
        let offer_data: Vec<OfferData> = offers.into_iter().map(|offer| {
            // Extract price from description (format: "Precio: $259.97 (Descuento: -0.16%)")
            let (precio_actual, descuento_porc) = Self::parse_price_info(&offer.offer_description);
            let precio_anterior = if descuento_porc > 0.0 {
                precio_actual / (1.0 - descuento_porc / 100.0)
            } else {
                precio_actual * 1.2
            };
            
            OfferData {
                comercio: "Comercio Genérico".to_string(), // Default since not available in OfferResult
                producto: offer.offer_title.unwrap_or_else(|| "Producto".to_string()),
                codigo: Some("N/A".to_string()), // Default since not available in OfferResult
                precio_actual,
                precio_anterior,
                descuento_porc: Some(descuento_porc.abs()), // Make positive for display
                descuento_valor: precio_anterior - precio_actual,
                dias: 30, // Default since not available in OfferResult
            }
        }).collect();
        
        info!("Using {} real offers for category '{}' (converted from OfferResult)", offer_data.len(), category);

        let request = DashboardRequest {
            offers: offer_data,
            category: category.to_string(),
            whatsapp_number: whatsapp_id.to_string(),  // Fixed: use whatsapp_number field
            user_id,
            callback_url: None, // We'll use polling instead of callbacks for now
        };

        // Submit dashboard generation request
        let url = format!("{}/api/v2/generate_offers_dashboard", self.api_base_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("Dashboard generation request failed: {}", error_text);
            return Err(anyhow::anyhow!("API request failed: {}", error_text));
        }

        let dashboard_response: DashboardResponse = response.json().await?;
        info!("Dashboard generation task submitted: {}", dashboard_response.task_id);

        // Poll for completion
        self.poll_task_completion(app_state, whatsapp_id, &dashboard_response.task_id).await?;

        Ok(dashboard_response.task_id)
    }

    /// Poll task status until completion
    async fn poll_task_completion(&self, app_state: &Arc<AppState>, whatsapp_id: &str, task_id: &str) -> Result<()> {
        info!("Polling task completion for task_id: {}", task_id);
        
        let max_attempts = 60; // 5 minutes with 5-second intervals
        let poll_interval = Duration::from_secs(5);

        for attempt in 1..=max_attempts {
            match self.get_task_status(task_id).await {
                Ok(status) => {
                    info!("Task {} status: {} (attempt {}/{})", task_id, status.status, attempt, max_attempts);
                    
                    match status.status.as_str() {
                        "completed" => {
                            info!("Task {} completed successfully", task_id);
                            
                            // Extract and process the result
                            if let Some(result) = status.result {
                                // Try direct access first (new structure)
                                let image_url = result.get("image_url").and_then(|v| v.as_str())
                                    .or_else(|| {
                                        // Fallback to nested structure (old structure)
                                        result.get("result")
                                            .and_then(|inner| inner.get("image_url"))
                                            .and_then(|v| v.as_str())
                                    });
                                
                                if let Some(image_url) = image_url {
                                    info!("Dashboard image URL: {}", image_url);
                                    info!("Image ready to send to WhatsApp: {}", image_url);
                                    
                                    // Send image to WhatsApp
                                    match whatsapp_service::send_image_message(
                                        app_state,
                                        whatsapp_id,
                                        image_url,
                                        Some("🎯 ¡Tu dashboard de ofertas está listo! Aquí tienes las mejores ofertas disponibles.")
                                    ).await {
                                        Ok(_) => {
                                            info!("Dashboard image sent successfully to WhatsApp user: {}", whatsapp_id);
                                        }
                                        Err(e) => {
                                            error!("Failed to send dashboard image to WhatsApp: {}", e);
                                            // Send fallback text message
                                            if let Err(fallback_err) = whatsapp_service::send_text_message(
                                                app_state,
                                                whatsapp_id,
                                                &format!("🎯 Tu dashboard de ofertas está listo pero hubo un problema al enviarlo. Puedes verlo en: {}", image_url)
                                            ).await {
                                                error!("Failed to send fallback text message: {}", fallback_err);
                                            }
                                        }
                                    }
                                } else {
                                    warn!("No image_url found in task result");
                                }
                            }
                            return Ok(());
                        }
                        "failed" => {
                            let error_msg = status.error.unwrap_or_else(|| "Unknown error".to_string());
                            error!("Task {} failed: {}", task_id, error_msg);
                            return Err(anyhow::anyhow!("Task failed: {}", error_msg));
                        }
                        "pending" | "processing" => {
                            // Continue polling
                            if let Some(progress) = status.progress {
                                info!("Task {} progress: {}%", task_id, progress);
                            }
                        }
                        _ => {
                            warn!("Unknown task status: {}", status.status);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get task status (attempt {}): {}", attempt, e);
                }
            }

            if attempt < max_attempts {
                sleep(poll_interval).await;
            }
        }

        Err(anyhow::anyhow!("Task polling timeout after {} attempts", max_attempts))
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Result<TaskStatusResponse> {
        let url = format!("{}/api/v2/task_status/{}", self.api_base_url, task_id);
        
        let response = timeout(Duration::from_secs(10), self.client.get(&url).send()).await??;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!("Status request failed: {}", error_text));
        }

        let status: TaskStatusResponse = response.json().await?;
        Ok(status)
    }

    /// Check API health
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/v2/health", self.api_base_url);
        
        match timeout(Duration::from_secs(5), self.client.get(&url).send()).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            _ => Ok(false),
        }
    }

    /// Parse price information from offer description
    fn parse_price_info(description: &Option<String>) -> (f64, f64) {
        if let Some(desc) = description {
            // Try to extract price from description like "Precio: $450 (Descuento: 15%)"
            if let Some(price_start) = desc.find("$") {
                if let Some(price_end) = desc[price_start + 1..].find(" ") {
                    if let Ok(price) = desc[price_start + 1..price_start + 1 + price_end].parse::<f64>() {
                        // Try to extract discount percentage
                        if let Some(discount_start) = desc.find("Descuento: ") {
                            if let Some(discount_end) = desc[discount_start + 11..].find("%") {
                                if let Ok(discount) = desc[discount_start + 11..discount_start + 11 + discount_end].parse::<f64>() {
                                    return (price, discount);
                                }
                            }
                        }
                        return (price, 15.0); // Default discount
                    }
                }
            }
        }
        (100.0, 10.0) // Default values
    }
}

/// Generate visual dashboard for offers
pub async fn generate_offers_visual_dashboard(
    app_state: &Arc<AppState>,
    offers: Vec<OfferResult>,
    category: &str,
    whatsapp_id: &str,
    user_id: Option<i64>,
) -> Result<()> {
    // Get API base URL from environment or use default
    let _api_base_url = std::env::var("VISUAL_API_URL")
        .unwrap_or_else(|_| "http://localhost:8001".to_string());

    let dashboard_service = VisualDashboardService::new();

    // Check API health first
    if !dashboard_service.health_check().await.unwrap_or(false) {
        warn!("Visual dashboard API is not healthy, falling back to text format");
        return generate_text_fallback(app_state, offers, category, whatsapp_id).await;
    }

    // Generate visual dashboard
    match dashboard_service
        .generate_offers_dashboard(app_state, offers.clone(), category, whatsapp_id, user_id)
        .await
    {
        Ok(task_id) => {
            info!("Visual dashboard generated successfully for user {}, task_id: {}", whatsapp_id, task_id);
            Ok(())
        }
        Err(e) => {
            error!("Failed to generate visual dashboard: {}", e);
            warn!("Falling back to text format");
            generate_text_fallback(app_state, offers, category, whatsapp_id).await
        }
    }
}

/// Fallback to text format if visual dashboard fails
async fn generate_text_fallback(
    app_state: &Arc<AppState>,
    offers: Vec<OfferResult>,
    category: &str,
    whatsapp_id: &str,
) -> Result<()> {
    use crate::services::whatsapp_service;

    if offers.is_empty() {
        let message = format!(
            "📭 No encontramos ofertas de *{}*\n\n💡 *Tip*: Prueba con un rango más amplio o revisa más tarde",
            category
        );
        whatsapp_service::send_text_message(app_state, whatsapp_id, &message).await?;
        return Ok(());
    }

    // Generate enhanced text format
    let mut message = format!("🎯 *Ofertas de {}*\n\n", category.to_uppercase());

    for (i, offer) in offers.iter().enumerate() {
        if i >= 5 { break; } // Limit to 5 offers

        let title = offer.offer_title.as_ref().map(|s| s.as_str()).unwrap_or("Oferta especial");
        let description = offer.offer_description.as_ref().map(|s| s.as_str()).unwrap_or("Descripción no disponible");

        message.push_str(&format!(
            "{}. *{}*\n📝 {}\n\n",
            i + 1, title, description
        ));
    }

    message.push_str("✨ ¡Aprovecha estas ofertas exclusivas!\n\n");
    message.push_str("💡 *Tip*: Usa más facturas con QR para acceder a más ofertas.");

    whatsapp_service::send_text_message(app_state, whatsapp_id, &message).await?;
    Ok(())
}

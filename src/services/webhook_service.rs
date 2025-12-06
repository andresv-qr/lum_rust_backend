// ============================================================================
// WEBHOOK SERVICE - Notificaciones asíncronas a Merchants
// ============================================================================

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;
use crate::observability::metrics::record_webhook_sent;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub merchant_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MerchantWebhook {
    pub merchant_id: Uuid,
    pub webhook_url: String,
    pub webhook_secret: String,
    pub events: Vec<String>,
    pub is_active: bool,
}

pub struct WebhookService {
    db: PgPool,
    http_client: Client,
}

impl WebhookService {
    pub fn new(db: PgPool) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { db, http_client }
    }

    /// Enviar webhook a merchant
    pub async fn send_webhook(
        &self,
        merchant_id: Uuid,
        event: WebhookEvent,
    ) -> Result<()> {
        // 1. Obtener configuración de webhook del merchant
        let webhook = match self.get_merchant_webhook(merchant_id).await? {
            Some(w) if w.is_active => w,
            Some(_) => {
                info!("Webhook disabled for merchant {}", merchant_id);
                return Ok(());
            }
            None => {
                info!("No webhook configured for merchant {}", merchant_id);
                return Ok(());
            }
        };

        // 2. Verificar que el merchant está suscrito a este evento
        if !webhook.events.contains(&event.event) {
            info!(
                "Merchant {} not subscribed to event {}",
                merchant_id, event.event
            );
            return Ok(());
        }

        // 3. Construir payload
        let payload = serde_json::to_string(&event)?;

        // 4. Generar firma HMAC
        let signature = self.generate_signature(&payload, &webhook.webhook_secret)?;

        // 5. Enviar request
        let max_retries = 3;
        let mut retry_count = 0;

        loop {
            let response = self
                .http_client
                .post(&webhook.webhook_url)
                .header("Content-Type", "application/json")
                .header("X-Webhook-Signature", &signature)
                .header("X-Webhook-Event", &event.event)
                .header("X-Webhook-Timestamp", event.timestamp.to_rfc3339())
                .body(payload.clone())
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    info!(
                        "Webhook sent successfully to merchant {} for event {}",
                        merchant_id, event.event
                    );
                    record_webhook_sent(&event.event, true);

                    // Guardar en log
                    self.log_webhook_sent(merchant_id, &event, true, None)
                        .await?;

                    return Ok(());
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());

                    error!(
                        "Webhook failed (status {}): {}",
                        status, error_body
                    );

                    if retry_count < max_retries {
                        retry_count += 1;
                        let backoff_secs = 2_u64.pow(retry_count);
                        warn!(
                            "Retrying webhook in {} seconds (attempt {}/{})",
                            backoff_secs, retry_count, max_retries
                        );
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        continue;
                    }

                    record_webhook_sent(&event.event, false);
                    self.log_webhook_sent(merchant_id, &event, false, Some(&error_body))
                        .await?;

                    return Err(anyhow::anyhow!("Webhook failed after {} retries", max_retries));
                }
                Err(e) => {
                    error!("Webhook request error: {}", e);

                    if retry_count < max_retries {
                        retry_count += 1;
                        let backoff_secs = 2_u64.pow(retry_count);
                        warn!(
                            "Retrying webhook in {} seconds (attempt {}/{})",
                            backoff_secs, retry_count, max_retries
                        );
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        continue;
                    }

                    record_webhook_sent(&event.event, false);
                    self.log_webhook_sent(merchant_id, &event, false, Some(&e.to_string()))
                        .await?;

                    return Err(anyhow::anyhow!("Webhook failed after {} retries: {}", max_retries, e));
                }
            }
        }
    }

    /// Obtener configuración de webhook del merchant
    async fn get_merchant_webhook(&self, merchant_id: Uuid) -> Result<Option<MerchantWebhook>> {
        let result = sqlx::query_as::<_, MerchantWebhook>(
            r#"
            SELECT 
                merchant_id,
                webhook_url,
                webhook_secret,
                webhook_events as events,
                webhook_enabled as is_active
            FROM rewards.merchants
            WHERE merchant_id = $1
              AND webhook_url IS NOT NULL
            "#,
        )
        .bind(merchant_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(result)
    }

    /// Generar firma HMAC SHA256
    fn generate_signature(&self, payload: &str, secret: &str) -> Result<String> {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .context("Invalid HMAC key")?;
        mac.update(payload.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        Ok(hex::encode(code_bytes))
    }

    /// Guardar log de webhook enviado
    async fn log_webhook_sent(
        &self,
        merchant_id: Uuid,
        event: &WebhookEvent,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO rewards.webhook_logs 
                (merchant_id, event_type, payload, success, error_message, sent_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
        )
        .bind(merchant_id)
        .bind(&event.event)
        .bind(&event.data)
        .bind(success)
        .bind(error_message)
        .execute(&self.db)
        .await
        .context("Failed to log webhook")?;

        Ok(())
    }

    // ========================================================================
    // WEBHOOK EVENTS - Eventos específicos de redenciones
    // ========================================================================

    /// Notificar creación de redención
    pub async fn notify_redemption_created(
        &self,
        merchant_id: Uuid,
        redemption_id: Uuid,
        redemption_code: &str,
        offer_name: &str,
        lumis_spent: i32,
    ) -> Result<()> {
        let event = WebhookEvent {
            event: "redemption.created".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "redemption_id": redemption_id,
                "redemption_code": redemption_code,
                "offer_name": offer_name,
                "lumis_spent": lumis_spent,
            }),
            merchant_id,
        };

        self.send_webhook(merchant_id, event).await
    }

    /// Notificar confirmación de redención
    pub async fn notify_redemption_confirmed(
        &self,
        merchant_id: Uuid,
        redemption_id: Uuid,
        redemption_code: &str,
        offer_name: &str,
        confirmed_by: &str,
    ) -> Result<()> {
        let event = WebhookEvent {
            event: "redemption.confirmed".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "redemption_id": redemption_id,
                "redemption_code": redemption_code,
                "offer_name": offer_name,
                "confirmed_by": confirmed_by,
                "confirmed_at": Utc::now(),
            }),
            merchant_id,
        };

        self.send_webhook(merchant_id, event).await
    }

    /// Notificar expiración de redención
    pub async fn notify_redemption_expired(
        &self,
        merchant_id: Uuid,
        redemption_id: Uuid,
        redemption_code: &str,
        offer_name: &str,
    ) -> Result<()> {
        let event = WebhookEvent {
            event: "redemption.expired".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "redemption_id": redemption_id,
                "redemption_code": redemption_code,
                "offer_name": offer_name,
                "expired_at": Utc::now(),
            }),
            merchant_id,
        };

        self.send_webhook(merchant_id, event).await
    }

    /// Notificar cancelación de redención
    pub async fn notify_redemption_cancelled(
        &self,
        merchant_id: Uuid,
        redemption_id: Uuid,
        redemption_code: &str,
        reason: &str,
    ) -> Result<()> {
        let event = WebhookEvent {
            event: "redemption.cancelled".to_string(),
            timestamp: Utc::now(),
            data: json!({
                "redemption_id": redemption_id,
                "redemption_code": redemption_code,
                "reason": reason,
                "cancelled_at": Utc::now(),
            }),
            merchant_id,
        };

        self.send_webhook(merchant_id, event).await
    }
}

// ============================================================================
// SHARED INSTANCE
// ============================================================================

use std::sync::OnceLock;

static WEBHOOK_SERVICE: OnceLock<Arc<WebhookService>> = OnceLock::new();

pub fn init_webhook_service(db: PgPool) {
    let service = Arc::new(WebhookService::new(db));
    if WEBHOOK_SERVICE.set(service).is_err() {
        warn!("Webhook service already initialized");
    }
}

pub fn get_webhook_service() -> Option<Arc<WebhookService>> {
    WEBHOOK_SERVICE.get().cloned()
}

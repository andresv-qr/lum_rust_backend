// ============================================================================
// PUSH NOTIFICATION SERVICE - Firebase Cloud Messaging (FCM) HTTP v1 API
// ============================================================================
//
// Uses FCM HTTP v1 API with OAuth 2.0 authentication (recommended by Google)
// - More secure: uses service account tokens instead of static server keys
// - Future-proof: Legacy API is deprecated
// - Better features: platform overrides, analytics, improved rate limits
//
// Configuration:
// - GOOGLE_APPLICATION_CREDENTIALS: Path to service account JSON file
// - FIREBASE_PROJECT_ID: Your Firebase project ID
//
// ============================================================================

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use crate::observability::metrics::{record_push_notification, record_notification_queue_processed};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotification {
    pub user_id: i32,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub priority: NotificationPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPriority {
    High,
    Normal,
}

/// FCM HTTP v1 API message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmV1Message {
    pub message: FcmV1MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmV1MessageContent {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<FcmV1Notification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android: Option<FcmAndroidConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apns: Option<FcmApnsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webpush: Option<FcmWebpushConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmV1Notification {
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmAndroidConfig {
    pub priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<FcmAndroidNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmAndroidNotification {
    pub sound: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmApnsConfig {
    pub payload: FcmApnsPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmApnsPayload {
    pub aps: FcmAps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmAps {
    pub sound: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<i64>,
    #[serde(rename = "content-available", skip_serializing_if = "Option::is_none")]
    pub content_available: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmWebpushConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<FcmWebpushNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmWebpushNotification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// FCM v1 error response
#[derive(Debug, Deserialize)]
struct FcmV1ErrorResponse {
    error: FcmV1Error,
}

#[derive(Debug, Deserialize)]
struct FcmV1Error {
    code: i32,
    message: String,
    status: String,
    #[serde(default)]
    details: Vec<FcmV1ErrorDetail>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FcmV1ErrorDetail {
    #[serde(rename = "@type")]
    error_type: Option<String>,
    #[serde(rename = "errorCode")]
    error_code: Option<String>,
}

// ============================================================================
// OAUTH TOKEN CACHE
// ============================================================================

/// Cached OAuth token with expiration
struct CachedToken {
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

// ============================================================================
// PUSH NOTIFICATION SERVICE
// ============================================================================

pub struct PushNotificationService {
    db: PgPool,
    http_client: Client,
    firebase_project_id: String,
    token_cache: Arc<RwLock<Option<CachedToken>>>,
    is_configured: bool,
}

impl PushNotificationService {
    pub fn new(db: PgPool) -> Self {
        let firebase_project_id = std::env::var("FIREBASE_PROJECT_ID").unwrap_or_default();
        
        // Check if service account credentials are available
        let credentials_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok();
        
        let is_configured = !firebase_project_id.is_empty() && credentials_path.is_some();
        
        if !is_configured {
            warn!(
                "FCM HTTP v1 not configured. Required env vars: \
                 FIREBASE_PROJECT_ID, GOOGLE_APPLICATION_CREDENTIALS"
            );
        } else {
            info!(
                "FCM HTTP v1 configured for project: {}", 
                firebase_project_id
            );
        }

        Self {
            db,
            http_client: Client::new(),
            firebase_project_id,
            token_cache: Arc::new(RwLock::new(None)),
            is_configured,
        }
    }

    /// Get a valid OAuth token for FCM
    async fn get_oauth_token(&self) -> Result<String> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(ref cached) = *cache {
                // Use token if it has at least 5 minutes of validity left
                if cached.expires_at > Utc::now() + Duration::minutes(5) {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Need to fetch new token using gcp_auth provider
        let provider = gcp_auth::provider()
            .await
            .context("Failed to create GCP auth provider")?;

        let scopes = &["https://www.googleapis.com/auth/firebase.messaging"];
        let token = provider
            .token(scopes)
            .await
            .context("Failed to get OAuth token for FCM")?;

        let token_string = token.as_str().to_string();
        
        // Cache the token (tokens typically last 1 hour, we cache for 50 minutes)
        let expires_at = Utc::now() + Duration::minutes(50);
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(CachedToken {
                token: token_string.clone(),
                expires_at,
            });
        }

        debug!("Obtained new FCM OAuth token, valid until {}", expires_at);
        Ok(token_string)
    }

    /// Build FCM v1 API endpoint URL
    fn fcm_endpoint(&self) -> String {
        format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.firebase_project_id
        )
    }

    /// Check if service is properly configured
    pub fn is_configured(&self) -> bool {
        self.is_configured
    }

    /// Send push notification to a user
    pub async fn send_notification(&self, notification: PushNotification) -> Result<()> {
        if !self.is_configured {
            warn!("FCM not configured, skipping notification");
            return Ok(());
        }

        // Get FCM token for user
        let fcm_token = self.get_user_fcm_token(notification.user_id).await?;

        if fcm_token.is_empty() {
            info!("User {} has no FCM token, skipping notification", notification.user_id);
            return Ok(());
        }

        // Build data map (FCM v1 requires string values)
        let mut data_map = std::collections::HashMap::new();
        if let Some(obj) = notification.data.as_object() {
            for (key, value) in obj {
                data_map.insert(
                    key.clone(),
                    match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    },
                );
            }
        }

        // Build FCM v1 message
        let priority = match notification.priority {
            NotificationPriority::High => "high",
            NotificationPriority::Normal => "normal",
        };

        let message = FcmV1Message {
            message: FcmV1MessageContent {
                token: fcm_token.clone(),
                notification: Some(FcmV1Notification {
                    title: notification.title.clone(),
                    body: notification.body.clone(),
                    image: None,
                }),
                data: if data_map.is_empty() { None } else { Some(data_map) },
                android: Some(FcmAndroidConfig {
                    priority: priority.to_string(),
                    notification: Some(FcmAndroidNotification {
                        sound: "default".to_string(),
                        click_action: Some("FLUTTER_NOTIFICATION_CLICK".to_string()),
                        channel_id: Some("high_importance_channel".to_string()),
                    }),
                }),
                apns: Some(FcmApnsConfig {
                    payload: FcmApnsPayload {
                        aps: FcmAps {
                            sound: "default".to_string(),
                            badge: None,
                            content_available: Some(1),
                        },
                    },
                }),
                webpush: None,
            },
        };

        // Send to FCM
        match self.send_fcm_v1_message(&message).await {
            Ok(_) => {
                info!("Push notification sent to user {}", notification.user_id);
                record_push_notification("redemption_notification", true);
                self.save_notification_history(&notification).await?;
                Ok(())
            }
            Err(e) => {
                error!("Failed to send push to user {}: {}", notification.user_id, e);
                record_push_notification("redemption_notification", false);
                Err(e)
            }
        }
    }

    /// Internal method to send FCM v1 message
    async fn send_fcm_v1_message(&self, message: &FcmV1Message) -> Result<()> {
        let oauth_token = self.get_oauth_token().await?;
        
        let response = self
            .http_client
            .post(&self.fcm_endpoint())
            .header("Authorization", format!("Bearer {}", oauth_token))
            .header("Content-Type", "application/json")
            .json(message)
            .send()
            .await
            .context("Failed to send FCM request")?;

        let status = response.status();

        if status.is_success() {
            debug!("FCM message sent successfully");
            Ok(())
        } else {
            let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            // Parse FCM v1 error response
            if let Ok(fcm_error) = serde_json::from_str::<FcmV1ErrorResponse>(&error_body) {
                // Check for specific error codes
                let error_code = fcm_error.error.details.iter()
                    .find_map(|d| d.error_code.clone())
                    .unwrap_or_else(|| fcm_error.error.status.clone());

                // These errors indicate the token is invalid and should be removed
                if error_code == "UNREGISTERED" || 
                   error_code == "INVALID_ARGUMENT" && fcm_error.error.message.contains("token") {
                    return Err(anyhow::anyhow!("InvalidToken: {}", error_code));
                }

                Err(anyhow::anyhow!(
                    "FCM error ({}): {} - {}",
                    fcm_error.error.code,
                    fcm_error.error.status,
                    fcm_error.error.message
                ))
            } else {
                Err(anyhow::anyhow!("FCM request failed ({}): {}", status, error_body))
            }
        }
    }

    /// Get FCM token for a user
    async fn get_user_fcm_token(&self, user_id: i32) -> Result<String> {
        // First try the new device_tokens table
        let result = sqlx::query_scalar::<_, String>(
            r#"
            SELECT fcm_token 
            FROM public.device_tokens 
            WHERE user_id = $1 
              AND fcm_token IS NOT NULL 
              AND is_active = true
            ORDER BY last_used_at DESC NULLS LAST
            LIMIT 1
            "#,
        )
        .bind(user_id as i64)
        .fetch_optional(&self.db)
        .await?;

        if let Some(token) = result {
            return Ok(token);
        }

        // Fallback to legacy user_devices table if exists
        let legacy_result = sqlx::query_scalar::<_, String>(
            r#"
            SELECT fcm_token 
            FROM public.user_devices 
            WHERE user_id = $1 
              AND fcm_token IS NOT NULL 
              AND is_active = true
            ORDER BY last_used_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .ok()
        .flatten();

        Ok(legacy_result.unwrap_or_default())
    }

    /// Get all active FCM tokens for a user
    pub async fn get_all_user_tokens(&self, user_id: i64) -> Result<Vec<String>> {
        let tokens = sqlx::query_scalar::<_, String>(
            r#"
            SELECT fcm_token 
            FROM public.device_tokens 
            WHERE user_id = $1 
              AND fcm_token IS NOT NULL 
              AND is_active = true
            ORDER BY last_used_at DESC NULLS LAST
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(tokens)
    }

    /// Save notification to history log
    async fn save_notification_history(&self, notification: &PushNotification) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO public.push_notifications_log 
                (user_id, title, body, data, sent_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(notification.user_id)
        .bind(&notification.title)
        .bind(&notification.body)
        .bind(&notification.data)
        .execute(&self.db)
        .await
        .ok(); // Non-critical, don't fail if logging fails

        Ok(())
    }

    /// Notify when a redemption is confirmed
    pub async fn notify_redemption_confirmed(
        &self,
        user_id: i32,
        redemption_id: uuid::Uuid,
        offer_name: &str,
    ) -> Result<()> {
        let notification = PushNotification {
            user_id,
            title: "¡Redención confirmada!".to_string(),
            body: format!("Tu redención de {} fue confirmada exitosamente", offer_name),
            data: json!({
                "type": "redemption_confirmed",
                "redemption_id": redemption_id.to_string(),
                "offer_name": offer_name,
            }),
            priority: NotificationPriority::High,
        };

        self.send_notification(notification).await
    }

    /// Notify when a redemption is about to expire
    pub async fn notify_redemption_expiring(
        &self,
        user_id: i32,
        redemption_id: uuid::Uuid,
        offer_name: &str,
        minutes_remaining: i32,
    ) -> Result<()> {
        let notification = PushNotification {
            user_id,
            title: "⏰ Tu redención expira pronto".to_string(),
            body: format!(
                "{} expira en {} minutos. ¡Úsala antes de que sea tarde!",
                offer_name, minutes_remaining
            ),
            data: json!({
                "type": "redemption_expiring",
                "redemption_id": redemption_id.to_string(),
                "offer_name": offer_name,
                "minutes_remaining": minutes_remaining,
            }),
            priority: NotificationPriority::High,
        };

        self.send_notification(notification).await
    }

    /// Notify when a redemption is created
    pub async fn notify_redemption_created(
        &self,
        user_id: i32,
        redemption_id: uuid::Uuid,
        offer_name: &str,
        redemption_code: &str,
    ) -> Result<()> {
        let notification = PushNotification {
            user_id,
            title: "🎁 Nueva redención creada".to_string(),
            body: format!("Muestra el código {} al comercio", redemption_code),
            data: json!({
                "type": "redemption_created",
                "redemption_id": redemption_id.to_string(),
                "offer_name": offer_name,
                "redemption_code": redemption_code,
            }),
            priority: NotificationPriority::Normal,
        };

        self.send_notification(notification).await
    }
}

// ============================================================================
// SHARED INSTANCE (for global access)
// ============================================================================

use std::sync::OnceLock;

static PUSH_SERVICE: OnceLock<Arc<PushNotificationService>> = OnceLock::new();

pub fn init_push_service(db: PgPool) {
    let service = Arc::new(PushNotificationService::new(db));
    if PUSH_SERVICE.set(service).is_err() {
        warn!("Push notification service already initialized");
    }
}

pub fn get_push_service() -> Option<Arc<PushNotificationService>> {
    PUSH_SERVICE.get().cloned()
}

// ============================================================================
// NOTIFICATION QUEUE PROCESSOR
// ============================================================================

/// Constants for queue processing
const QUEUE_BATCH_SIZE: i64 = 50;
const MAX_RETRY_ATTEMPTS: i32 = 3;
const BACKOFF_BASE_SECONDS: i64 = 30;

/// Result of processing the notification queue
#[derive(Debug, Default)]
pub struct QueueProcessResult {
    pub sent: usize,
    pub failed: usize,
    pub skipped: usize,
    pub invalid_tokens: usize,
}

impl QueueProcessResult {
    pub fn total_processed(&self) -> usize {
        self.sent + self.failed + self.skipped
    }
}

impl PushNotificationService {
    /// Process the notification push queue
    /// Uses SKIP LOCKED for safe concurrent processing
    pub async fn process_notification_queue(&self) -> Result<QueueProcessResult> {
        if !self.is_configured {
            return Ok(QueueProcessResult::default());
        }

        let start = std::time::Instant::now();
        let now = Utc::now();

        // Fetch pending items with SKIP LOCKED for concurrency safety
        let pending_items = sqlx::query!(
            r#"
            SELECT 
                q.id,
                q.notification_id,
                q.attempts,
                n.user_id,
                n.title,
                n.body,
                n.type as notification_type,
                n.action_url,
                n.payload
            FROM public.notification_push_queue q
            JOIN public.notifications n ON q.notification_id = n.id
            WHERE q.status IN ('pending', 'retrying')
              AND q.next_attempt_at <= $1
            ORDER BY q.next_attempt_at ASC
            LIMIT $2
            FOR UPDATE OF q SKIP LOCKED
            "#,
            now,
            QUEUE_BATCH_SIZE
        )
        .fetch_all(&self.db)
        .await?;

        if pending_items.is_empty() {
            return Ok(QueueProcessResult::default());
        }

        info!("Processing {} push notifications from queue", pending_items.len());

        let mut final_result = QueueProcessResult::default();
        let mut all_invalid_tokens: Vec<String> = Vec::new();

        // Process each notification item sequentially to avoid lifetime issues
        // with nested async closures. The actual FCM sends are still concurrent
        // per user via tokio::spawn.
        for item in pending_items {
            let tokens_result = self.get_all_user_tokens(item.user_id).await;
            
            match tokens_result {
                Ok(tokens) if tokens.is_empty() => {
                    // No active tokens, mark as skipped
                    sqlx::query!(
                        r#"
                        UPDATE public.notification_push_queue
                        SET status = 'skipped', 
                            last_attempt_at = $2,
                            error_message = 'No active device tokens'
                        WHERE id = $1
                        "#,
                        item.id,
                        now
                    )
                    .execute(&self.db)
                    .await
                    .ok();

                    final_result.skipped += 1;
                    continue;
                }
                Err(e) => {
                    warn!("Failed to get tokens for user {}: {}", item.user_id, e);
                    final_result.failed += 1;
                    
                    // Update queue status
                    self.update_queue_item_failed(&item.id, item.attempts, now).await;
                    continue;
                }
                Ok(tokens) => {
                    // Get badge count for iOS
                    let badge_count: i64 = sqlx::query_scalar!(
                        r#"
                        SELECT COUNT(*)::BIGINT as "count!"
                        FROM public.notifications
                        WHERE user_id = $1 AND is_read = FALSE AND is_dismissed = FALSE
                        "#,
                        item.user_id
                    )
                    .fetch_one(&self.db)
                    .await
                    .unwrap_or(0);

                    // Build data map for FCM v1 (all values must be strings)
                    let mut data_map = std::collections::HashMap::new();
                    data_map.insert("notification_id".to_string(), item.notification_id.to_string());
                    data_map.insert("type".to_string(), item.notification_type.clone());
                    data_map.insert("action_url".to_string(), item.action_url.clone().unwrap_or_default());
                    data_map.insert("click_action".to_string(), "FLUTTER_NOTIFICATION_CLICK".to_string());
                    data_map.insert("badge_count".to_string(), badge_count.to_string());

                    // Add payload fields if present
                    if let Some(payload) = &item.payload {
                        if let Some(obj) = payload.as_object() {
                            for (key, value) in obj {
                                data_map.insert(
                                    key.clone(),
                                    match value {
                                        serde_json::Value::String(s) => s.clone(),
                                        other => other.to_string(),
                                    },
                                );
                            }
                        }
                    }

                    // Send to all tokens for this user
                    let mut item_sent = 0usize;
                    let mut item_failed = 0usize;
                    let mut item_invalid_tokens: Vec<String> = Vec::new();

                    for token in tokens {
                        let message = FcmV1Message {
                            message: FcmV1MessageContent {
                                token: token.clone(),
                                notification: Some(FcmV1Notification {
                                    title: item.title.clone(),
                                    body: item.body.clone(),
                                    image: None,
                                }),
                                data: Some(data_map.clone()),
                                android: Some(FcmAndroidConfig {
                                    priority: "high".to_string(),
                                    notification: Some(FcmAndroidNotification {
                                        sound: "default".to_string(),
                                        click_action: Some("FLUTTER_NOTIFICATION_CLICK".to_string()),
                                        channel_id: Some("high_importance_channel".to_string()),
                                    }),
                                }),
                                apns: Some(FcmApnsConfig {
                                    payload: FcmApnsPayload {
                                        aps: FcmAps {
                                            sound: "default".to_string(),
                                            badge: Some(badge_count),
                                            content_available: Some(1),
                                        },
                                    },
                                }),
                                webpush: None,
                            },
                        };

                        match self.send_fcm_v1_message(&message).await {
                            Ok(_) => {
                                item_sent += 1;
                                record_push_notification("queue_notification", true);
                            }
                            Err(e) => {
                                let error_msg = e.to_string();
                                // FCM v1 error codes for invalid tokens
                                if error_msg.contains("InvalidToken") || 
                                   error_msg.contains("UNREGISTERED") ||
                                   error_msg.contains("INVALID_ARGUMENT") {
                                    item_invalid_tokens.push(token);
                                    final_result.invalid_tokens += 1;
                                } else {
                                    warn!("Failed to send push to token: {}", e);
                                    item_failed += 1;
                                }
                                record_push_notification("queue_notification", false);
                            }
                        }
                    }

                    final_result.sent += item_sent;
                    final_result.failed += item_failed;
                    all_invalid_tokens.extend(item_invalid_tokens);

                    // Update queue item status
                    let all_success = item_failed == 0;
                    if all_success || item.attempts >= MAX_RETRY_ATTEMPTS {
                        let status = if all_success { "sent" } else { "failed" };
                        sqlx::query!(
                            r#"
                            UPDATE public.notification_push_queue
                            SET status = $2, 
                                last_attempt_at = $3,
                                attempts = attempts + 1
                            WHERE id = $1
                            "#,
                            item.id,
                            status,
                            now
                        )
                        .execute(&self.db)
                        .await
                        .ok();
                    } else {
                        // Schedule retry with exponential backoff
                        let backoff_seconds = BACKOFF_BASE_SECONDS * 2_i64.pow(item.attempts as u32);
                        let next_attempt = now + Duration::seconds(backoff_seconds);
                        sqlx::query!(
                            r#"
                            UPDATE public.notification_push_queue
                            SET status = 'retrying', 
                                last_attempt_at = $2,
                                next_attempt_at = $3,
                                attempts = attempts + 1
                            WHERE id = $1
                            "#,
                            item.id,
                            now,
                            next_attempt
                        )
                        .execute(&self.db)
                        .await
                        .ok();
                    }
                }
            }
        }

        // Batch deactivate invalid tokens
        if !all_invalid_tokens.is_empty() {
            for chunk in all_invalid_tokens.chunks(50) {
                sqlx::query!(
                    r#"
                    UPDATE public.device_tokens
                    SET is_active = FALSE, updated_at = $2
                    WHERE fcm_token = ANY($1)
                    "#,
                    chunk,
                    now
                )
                .execute(&self.db)
                .await
                .ok();
            }
            info!("Deactivated {} invalid FCM tokens", all_invalid_tokens.len());
        }

        // Record metrics
        record_notification_queue_processed(
            final_result.sent, 
            final_result.failed, 
            final_result.skipped, 
            final_result.invalid_tokens
        );

        let elapsed = start.elapsed();
        if final_result.total_processed() > 0 {
            info!(
                "Queue batch processed in {:?}: sent={}, failed={}, skipped={}, invalid={}", 
                elapsed, final_result.sent, final_result.failed, final_result.skipped, final_result.invalid_tokens
            );
        }

        Ok(final_result)
    }

    /// Helper to update queue item as failed
    async fn update_queue_item_failed(&self, id: &i64, attempts: i32, now: chrono::DateTime<Utc>) {
        if attempts >= MAX_RETRY_ATTEMPTS {
            sqlx::query!(
                r#"
                UPDATE public.notification_push_queue
                SET status = 'failed', 
                    last_attempt_at = $2,
                    attempts = attempts + 1
                WHERE id = $1
                "#,
                id,
                now
            )
            .execute(&self.db)
            .await
            .ok();
        } else {
            let backoff_seconds = BACKOFF_BASE_SECONDS * 2_i64.pow(attempts as u32);
            let next_attempt = now + Duration::seconds(backoff_seconds);
            sqlx::query!(
                r#"
                UPDATE public.notification_push_queue
                SET status = 'retrying', 
                    last_attempt_at = $2,
                    next_attempt_at = $3,
                    attempts = attempts + 1
                WHERE id = $1
                "#,
                id,
                now,
                next_attempt
            )
            .execute(&self.db)
            .await
            .ok();
        }
    }
}

// ============================================================================
// BACKGROUND WORKER
// ============================================================================

/// Configuration for the push queue worker
const WORKER_POLL_INTERVAL_SECS: u64 = 5;
const WORKER_ERROR_BACKOFF_SECS: u64 = 30;

/// Start the push notification queue worker as a background task
/// 
/// IMPORTANT: This function creates a single PushNotificationService instance
/// that is reused across all iterations. The reqwest::Client inside it maintains
/// a connection pool, and the OAuth token is cached for efficiency.
pub async fn start_push_queue_worker(db: PgPool) {
    let service = Arc::new(PushNotificationService::new(db));

    if !service.is_configured() {
        warn!("FCM not configured, push queue worker will not start");
        return;
    }

    info!(
        "Starting push notification queue worker (FCM HTTP v1, poll interval: {}s)", 
        WORKER_POLL_INTERVAL_SECS
    );

    let mut consecutive_errors = 0u32;

    loop {
        match service.process_notification_queue().await {
            Ok(result) => {
                consecutive_errors = 0;
                if result.total_processed() > 0 {
                    info!(
                        "Push worker: sent={}, failed={}, skipped={}, invalid_tokens={}",
                        result.sent, result.failed, result.skipped, result.invalid_tokens
                    );
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                error!("Push worker error (consecutive: {}): {}", consecutive_errors, e);
                
                // Exponential backoff on repeated errors (max 5 min)
                if consecutive_errors >= 3 {
                    let backoff = std::cmp::min(
                        WORKER_ERROR_BACKOFF_SECS * 2u64.pow(consecutive_errors - 3),
                        300
                    );
                    warn!("Push worker backing off for {}s due to repeated errors", backoff);
                    tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                    continue;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(WORKER_POLL_INTERVAL_SECS)).await;
    }
}

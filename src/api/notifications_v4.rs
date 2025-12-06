//! Notifications API v4
//! 
//! Endpoints for in-app notifications and push notification management.
//! 
//! Features:
//! - List notifications with pagination and filtering
//! - Mark notifications as read (single and batch)
//! - Dismiss/delete notifications (soft-delete)
//! - Badge count for unread notifications
//! - FCM token registration and management
//!
//! Security:
//! - All endpoints require JWT authentication
//! - Notifications are scoped to the authenticated user
//! - Rate limiting applied at middleware level

use axum::{
    extract::{Path, Query, State, Extension},
    Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::{
    middleware::CurrentUser,
    api::common::{ApiError, ApiResponse},
    AppState,
};

// Response wrapper for JSON
type ResponseJson<T> = Result<Json<ApiResponse<T>>, ApiError>;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum notifications per request
const MAX_LIMIT: i32 = 100;
/// Default notifications per request
const DEFAULT_LIMIT: i32 = 20;

/// Valid notification types
const VALID_TYPES: &[&str] = &[
    "reward", "achievement", "streak", "invoice",
    "promo", "system", "challenge", "level_up", "reminder"
];

/// Valid platforms for FCM tokens
const VALID_PLATFORMS: &[&str] = &["android", "ios", "web"];

// ============================================================================
// DATABASE MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Notification {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub body: String,
    #[sqlx(rename = "type")]
    pub notification_type: String,
    pub priority: String,
    pub is_read: bool,
    pub is_dismissed: bool,
    pub image_url: Option<String>,
    pub action_url: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DeviceToken {
    pub id: i64,
    pub user_id: i64,
    pub fcm_token: String,
    pub platform: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

/// Query parameters for listing notifications
#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
    #[serde(default)]
    pub unread_only: bool,
    #[serde(rename = "type")]
    pub notification_type: Option<String>,
    /// ISO8601 datetime for incremental sync
    pub since: Option<DateTime<Utc>>,
}

fn default_limit() -> i32 {
    DEFAULT_LIMIT
}

/// Response for listing notifications
#[derive(Debug, Serialize)]
pub struct ListNotificationsResponse {
    pub notifications: Vec<NotificationResponse>,
    pub meta: ListMeta,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub id: i64,
    pub title: String,
    pub body: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub priority: String,
    pub is_read: bool,
    pub image_url: Option<String>,
    pub action_url: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<Notification> for NotificationResponse {
    fn from(n: Notification) -> Self {
        Self {
            id: n.id,
            title: n.title,
            body: n.body,
            notification_type: n.notification_type,
            priority: n.priority,
            is_read: n.is_read,
            image_url: n.image_url,
            action_url: n.action_url,
            payload: n.payload,
            created_at: n.created_at,
            expires_at: n.expires_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListMeta {
    pub total: i64,
    pub unread_count: i64,
    pub limit: i32,
    pub offset: i32,
    pub has_more: bool,
}

/// Response for badge count endpoint
#[derive(Debug, Serialize)]
pub struct BadgeCountResponse {
    pub unread_count: i64,
    pub by_type: serde_json::Value,
    pub has_urgent: bool,
}

/// Response for marking as read
#[derive(Debug, Serialize)]
pub struct MarkReadResponse {
    pub id: i64,
    pub is_read: bool,
    pub read_at: DateTime<Utc>,
}

/// Request for mark all as read
#[derive(Debug, Deserialize)]
pub struct MarkAllReadRequest {
    #[serde(rename = "type")]
    pub notification_type: Option<String>,
    pub before: Option<DateTime<Utc>>,
}

/// Response for mark all as read
#[derive(Debug, Serialize)]
pub struct MarkAllReadResponse {
    pub marked_count: i64,
    pub read_at: DateTime<Utc>,
}

/// Response for dismiss notification
#[derive(Debug, Serialize)]
pub struct DismissResponse {
    pub id: i64,
    pub dismissed: bool,
}

/// Request for registering FCM token
#[derive(Debug, Deserialize)]
pub struct RegisterTokenRequest {
    pub fcm_token: String,
    pub platform: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
}

impl RegisterTokenRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.fcm_token.is_empty() {
            return Err("fcm_token is required".to_string());
        }
        if self.fcm_token.len() > 500 {
            return Err("fcm_token is too long (max 500 chars)".to_string());
        }
        if !VALID_PLATFORMS.contains(&self.platform.as_str()) {
            return Err(format!(
                "Invalid platform '{}'. Valid platforms: {:?}",
                self.platform, VALID_PLATFORMS
            ));
        }
        Ok(())
    }
}

/// Response for registering FCM token
#[derive(Debug, Serialize)]
pub struct RegisterTokenResponse {
    pub registered: bool,
    pub device_id: Option<String>,
    pub is_new: bool,
}

/// Request for removing FCM token
#[derive(Debug, Deserialize)]
pub struct RemoveTokenRequest {
    pub fcm_token: String,
}

/// Response for removing FCM token
#[derive(Debug, Serialize)]
pub struct RemoveTokenResponse {
    pub removed: bool,
}

// ============================================================================
// ROUTER
// ============================================================================

/// Create the notifications v4 router
pub fn create_notifications_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        // Notification endpoints
        .route("/", get(list_notifications))
        .route("/count", get(get_badge_count))
        .route("/:id/read", post(mark_as_read))
        .route("/read-all", post(mark_all_as_read))
        .route("/:id", delete(dismiss_notification))
        // Device token endpoints (under /devices prefix)
        .route("/devices/fcm-token", post(register_fcm_token))
        .route("/devices/fcm-token", delete(remove_fcm_token))
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /api/v4/notifications
/// List notifications for the authenticated user
/// 
/// Performance: Uses a single query with window functions to get both
/// the paginated results AND the total/unread counts in one round-trip.
#[axum::debug_handler]
pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<ListNotificationsQuery>,
) -> ResponseJson<ListNotificationsResponse> {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    let user_id = current_user.user_id as i64;
    
    // Validate and clamp limit
    let limit = params.limit.min(MAX_LIMIT).max(1);
    let offset = params.offset.max(0);
    
    // Validate type if provided
    if let Some(ref t) = params.notification_type {
        if !VALID_TYPES.contains(&t.as_str()) {
            return Err(ApiError {
                code: "INVALID_TYPE".to_string(),
                message: format!("Invalid notification type '{}'. Valid types: {:?}", t, VALID_TYPES),
                details: None,
            });
        }
    }
    
    // Use a single optimized query with window functions for counts
    // This avoids a second round-trip to the database
    let notification_type_filter = params.notification_type.as_deref();
    let since_filter = params.since;
    let unread_only = params.unread_only;
    
    // Build the query dynamically but efficiently
    // Note: We use raw query here to support dynamic filtering with window functions
    let rows = sqlx::query!(
        r#"
        WITH filtered AS (
            SELECT 
                id, user_id, title, body, type, priority,
                is_read, is_dismissed, image_url, action_url, payload,
                created_at, read_at, expires_at,
                -- Window functions for counts (computed once over the full result set)
                COUNT(*) OVER() as total_filtered,
                COUNT(*) FILTER (WHERE is_read = FALSE) OVER() as unread_in_view
            FROM public.notifications
            WHERE user_id = $1 
              AND is_dismissed = FALSE
              AND (expires_at IS NULL OR expires_at > NOW())
              -- Dynamic filters via COALESCE patterns
              AND ($4::BOOLEAN = FALSE OR is_read = FALSE)
              AND ($5::VARCHAR IS NULL OR type = $5)
              AND ($6::TIMESTAMPTZ IS NULL OR created_at > $6)
            ORDER BY created_at DESC
        )
        SELECT 
            id, user_id, title, body, type as "notification_type", priority,
            is_read, is_dismissed, image_url, action_url, payload,
            created_at, read_at, expires_at,
            total_filtered as "total!",
            unread_in_view as "unread_count!"
        FROM filtered
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        limit as i64,
        offset as i64,
        unread_only,
        notification_type_filter,
        since_filter
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to fetch notifications: {}", e),
        details: None,
    })?;
    
    // Extract totals from first row (or default to 0 if empty)
    let (total, unread_count) = rows.first()
        .map(|r| (r.total, r.unread_count))
        .unwrap_or((0, 0));
    
    let has_more = (offset as i64 + rows.len() as i64) < total;
    
    // Transform to response format
    let notifications: Vec<NotificationResponse> = rows.into_iter().map(|row| {
        NotificationResponse {
            id: row.id,
            title: row.title,
            body: row.body,
            notification_type: row.notification_type,
            priority: row.priority,
            is_read: row.is_read,
            image_url: row.image_url,
            action_url: row.action_url,
            payload: row.payload.unwrap_or_default(),
            created_at: row.created_at,
            expires_at: row.expires_at,
        }
    }).collect();
    
    let response = ListNotificationsResponse {
        notifications,
        meta: ListMeta {
            total,
            unread_count,
            limit,
            offset,
            has_more,
        },
    };
    
    let elapsed = start.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(response, request_id, Some(elapsed), false)))
}

/// GET /api/v4/notifications/count
/// Get badge count for unread notifications
#[axum::debug_handler]
pub async fn get_badge_count(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> ResponseJson<BadgeCountResponse> {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    let user_id = current_user.user_id as i64;
    
    // Get counts by type using partial index (super fast)
    let counts = sqlx::query!(
        r#"
        SELECT 
            type,
            COUNT(*) as count,
            BOOL_OR(priority = 'urgent') as has_urgent
        FROM public.notifications
        WHERE user_id = $1 
          AND is_read = FALSE 
          AND is_dismissed = FALSE
          AND (expires_at IS NULL OR expires_at > NOW())
        GROUP BY type
        "#,
        user_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to count notifications: {}", e),
        details: None,
    })?;
    
    let mut by_type = serde_json::Map::new();
    let mut total_unread: i64 = 0;
    let mut has_urgent = false;
    
    for row in counts {
        let t = row.r#type;
        let count = row.count.unwrap_or(0);
        by_type.insert(t, serde_json::Value::Number(count.into()));
        total_unread += count;
        if row.has_urgent.unwrap_or(false) {
            has_urgent = true;
        }
    }
    
    let response = BadgeCountResponse {
        unread_count: total_unread,
        by_type: serde_json::Value::Object(by_type),
        has_urgent,
    };
    
    let elapsed = start.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(response, request_id, Some(elapsed), false)))
}

/// POST /api/v4/notifications/:id/read
/// Mark a single notification as read
#[axum::debug_handler]
pub async fn mark_as_read(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path(notification_id): Path<i64>,
) -> ResponseJson<MarkReadResponse> {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    let user_id = current_user.user_id as i64;
    let now = Utc::now();
    
    // Update with ownership check and idempotent behavior
    let result = sqlx::query!(
        r#"
        UPDATE public.notifications
        SET is_read = TRUE,
            read_at = COALESCE(read_at, $3)
        WHERE id = $1 AND user_id = $2
        RETURNING id, is_read, read_at
        "#,
        notification_id,
        user_id,
        now
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to update notification: {}", e),
        details: None,
    })?;
    
    match result {
        Some(row) => {
            let response = MarkReadResponse {
                id: row.id,
                is_read: row.is_read,
                read_at: row.read_at.unwrap_or(now),
            };
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(Json(ApiResponse::success(response, request_id, Some(elapsed), false)))
        }
        None => Err(ApiError {
            code: "NOTIFICATION_NOT_FOUND".to_string(),
            message: "Notificación no encontrada".to_string(),
            details: None,
        }),
    }
}

/// POST /api/v4/notifications/read-all
/// Mark all notifications as read (optionally filtered by type)
#[axum::debug_handler]
pub async fn mark_all_as_read(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<Option<MarkAllReadRequest>>,
) -> ResponseJson<MarkAllReadResponse> {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    let user_id = current_user.user_id as i64;
    let now = Utc::now();
    
    let payload = payload.unwrap_or(MarkAllReadRequest {
        notification_type: None,
        before: None,
    });
    
    // Build query based on filters
    let marked_count = if let Some(ref t) = payload.notification_type {
        if let Some(ref before) = payload.before {
            sqlx::query_scalar!(
                r#"
                WITH updated AS (
                    UPDATE public.notifications
                    SET is_read = TRUE, read_at = $4
                    WHERE user_id = $1 
                      AND is_read = FALSE 
                      AND is_dismissed = FALSE
                      AND type = $2
                      AND created_at < $3
                    RETURNING 1
                )
                SELECT COUNT(*)::BIGINT as "count!" FROM updated
                "#,
                user_id,
                t,
                before,
                now
            )
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to mark notifications as read: {}", e),
                details: None,
            })?
        } else {
            sqlx::query_scalar!(
                r#"
                WITH updated AS (
                    UPDATE public.notifications
                    SET is_read = TRUE, read_at = $3
                    WHERE user_id = $1 
                      AND is_read = FALSE 
                      AND is_dismissed = FALSE
                      AND type = $2
                    RETURNING 1
                )
                SELECT COUNT(*)::BIGINT as "count!" FROM updated
                "#,
                user_id,
                t,
                now
            )
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: format!("Failed to mark notifications as read: {}", e),
                details: None,
            })?
        }
    } else if let Some(ref before) = payload.before {
        sqlx::query_scalar!(
            r#"
            WITH updated AS (
                UPDATE public.notifications
                SET is_read = TRUE, read_at = $3
                WHERE user_id = $1 
                  AND is_read = FALSE 
                  AND is_dismissed = FALSE
                  AND created_at < $2
                RETURNING 1
            )
            SELECT COUNT(*)::BIGINT as "count!" FROM updated
            "#,
            user_id,
            before,
            now
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError {
            code: "DATABASE_ERROR".to_string(),
            message: format!("Failed to mark notifications as read: {}", e),
            details: None,
        })?
    } else {
        sqlx::query_scalar!(
            r#"
            WITH updated AS (
                UPDATE public.notifications
                SET is_read = TRUE, read_at = $2
                WHERE user_id = $1 
                  AND is_read = FALSE 
                  AND is_dismissed = FALSE
                RETURNING 1
            )
            SELECT COUNT(*)::BIGINT as "count!" FROM updated
            "#,
            user_id,
            now
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| ApiError {
            code: "DATABASE_ERROR".to_string(),
            message: format!("Failed to mark notifications as read: {}", e),
            details: None,
        })?
    };
    
    let response = MarkAllReadResponse {
        marked_count,
        read_at: now,
    };
    
    let elapsed = start.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(response, request_id, Some(elapsed), false)))
}

/// DELETE /api/v4/notifications/:id
/// Dismiss (soft-delete) a notification
#[axum::debug_handler]
pub async fn dismiss_notification(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path(notification_id): Path<i64>,
) -> ResponseJson<DismissResponse> {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    let user_id = current_user.user_id as i64;
    
    // Soft-delete with ownership check
    let result = sqlx::query!(
        r#"
        UPDATE public.notifications
        SET is_dismissed = TRUE
        WHERE id = $1 AND user_id = $2
        RETURNING id
        "#,
        notification_id,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to dismiss notification: {}", e),
        details: None,
    })?;
    
    match result {
        Some(row) => {
            let response = DismissResponse {
                id: row.id,
                dismissed: true,
            };
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(Json(ApiResponse::success(response, request_id, Some(elapsed), false)))
        }
        None => Err(ApiError {
            code: "NOTIFICATION_NOT_FOUND".to_string(),
            message: "Notificación no encontrada".to_string(),
            details: None,
        }),
    }
}

/// POST /api/v4/notifications/devices/fcm-token
/// Register an FCM token for push notifications
#[axum::debug_handler]
pub async fn register_fcm_token(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<RegisterTokenRequest>,
) -> ResponseJson<RegisterTokenResponse> {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    let user_id = current_user.user_id as i64;
    
    // Validate request
    payload.validate().map_err(|e| ApiError {
        code: "INVALID_REQUEST".to_string(),
        message: e,
        details: None,
    })?;
    
    // Use ON CONFLICT to handle race conditions
    // The trigger handles deactivating tokens from other users
    let result = sqlx::query!(
        r#"
        INSERT INTO public.device_tokens (user_id, fcm_token, platform, device_id, device_name, app_version)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (fcm_token) WHERE is_active = TRUE
        DO UPDATE SET 
            user_id = EXCLUDED.user_id,
            platform = EXCLUDED.platform,
            device_id = EXCLUDED.device_id,
            device_name = EXCLUDED.device_name,
            app_version = EXCLUDED.app_version,
            updated_at = NOW(),
            last_used_at = NOW()
        RETURNING id, device_id, (xmax = 0) as "is_new!"
        "#,
        user_id,
        payload.fcm_token,
        payload.platform,
        payload.device_id,
        payload.device_name,
        payload.app_version
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to register token: {}", e),
        details: None,
    })?;
    
    let response = RegisterTokenResponse {
        registered: true,
        device_id: result.device_id,
        is_new: result.is_new,
    };
    
    let elapsed = start.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(response, request_id, Some(elapsed), false)))
}

/// DELETE /api/v4/notifications/devices/fcm-token
/// Remove an FCM token (logout)
#[axum::debug_handler]
pub async fn remove_fcm_token(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<RemoveTokenRequest>,
) -> ResponseJson<RemoveTokenResponse> {
    let start = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    let user_id = current_user.user_id as i64;
    
    // Deactivate token (only for the current user)
    let result = sqlx::query!(
        r#"
        UPDATE public.device_tokens
        SET is_active = FALSE, updated_at = NOW()
        WHERE fcm_token = $1 AND user_id = $2 AND is_active = TRUE
        RETURNING id
        "#,
        payload.fcm_token,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError {
        code: "DATABASE_ERROR".to_string(),
        message: format!("Failed to remove token: {}", e),
        details: None,
    })?;
    
    let response = RemoveTokenResponse {
        removed: result.is_some(),
    };
    
    let elapsed = start.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(response, request_id, Some(elapsed), false)))
}

// ============================================================================
// HELPER FUNCTIONS FOR OTHER MODULES
// ============================================================================

/// Create a notification from Rust code (wrapper for SQL function)
pub async fn create_notification_from_rust(
    pool: &sqlx::PgPool,
    user_id: i64,
    title: &str,
    body: &str,
    notification_type: &str,
    priority: &str,
    action_url: Option<&str>,
    image_url: Option<&str>,
    payload: serde_json::Value,
    idempotency_key: Option<&str>,
    send_push: bool,
) -> Result<Option<i64>, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT public.create_notification(
            $1::BIGINT,
            $2::VARCHAR(200),
            $3::TEXT,
            $4::VARCHAR(50),
            $5::VARCHAR(20),
            $6::VARCHAR(255),
            $7::TEXT,
            $8::JSONB,
            $9::VARCHAR(100),
            NULL::TIMESTAMPTZ,
            $10::BOOLEAN
        ) as "id"
        "#,
        user_id,
        title,
        body,
        notification_type,
        priority,
        action_url,
        image_url,
        payload,
        idempotency_key,
        send_push
    )
    .fetch_one(pool)
    .await?;
    
    Ok(result)
}

/// Notify achievement unlocked (wrapper for SQL function)
pub async fn notify_achievement(
    pool: &sqlx::PgPool,
    user_id: i64,
    achievement_code: &str,
    achievement_name: &str,
    lumis_reward: i32,
) -> Result<Option<i64>, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT public.notify_achievement_unlocked(
            $1::BIGINT,
            $2::VARCHAR(50),
            $3::VARCHAR(200),
            $4::INTEGER
        ) as "id"
        "#,
        user_id,
        achievement_code,
        achievement_name,
        lumis_reward
    )
    .fetch_one(pool)
    .await?;
    
    Ok(result)
}

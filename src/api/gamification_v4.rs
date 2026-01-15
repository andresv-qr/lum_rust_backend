use axum::{
    extract::{Query, State, Extension},
    Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};
use rust_decimal::Decimal;

use crate::{
    middleware::CurrentUser,
    api::common::{ApiError, ApiResponse},
    AppState,
};

// Response wrapper for JSON
type ResponseJson<T> = Result<Json<ApiResponse<T>>, ApiError>;

// ============================================================================
// CONSTANTS FOR VALIDATION
// ============================================================================

/// Valid action types for gamification tracking
const VALID_ACTIONS: &[&str] = &[
    "daily_login",
    "invoice_upload", 
    "survey_complete",
    "referral_complete",
    "profile_complete",
    "first_redemption",
];

/// Valid channel types
const VALID_CHANNELS: &[&str] = &[
    "mobile_app",
    "whatsapp",
    "web_app",
    "telegram",
    "api",
];

/// Maximum metadata JSON size in bytes (10KB)
const MAX_METADATA_SIZE: usize = 10 * 1024;

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TrackActionRequest {
    pub action: String,  // 'daily_login', 'invoice_upload', 'survey_complete'
    #[serde(default = "default_channel")]
    pub channel: String,  // 'mobile_app', 'whatsapp', 'web_app'
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl TrackActionRequest {
    /// Validate the request fields
    pub fn validate(&self) -> Result<(), String> {
        // Validate action
        if !VALID_ACTIONS.contains(&self.action.as_str()) {
            return Err(format!(
                "Invalid action '{}'. Valid actions: {:?}",
                self.action, VALID_ACTIONS
            ));
        }
        
        // Validate channel
        if !VALID_CHANNELS.contains(&self.channel.as_str()) {
            return Err(format!(
                "Invalid channel '{}'. Valid channels: {:?}",
                self.channel, VALID_CHANNELS
            ));
        }
        
        // Validate metadata size (prevent DoS with huge JSON)
        let metadata_str = self.metadata.to_string();
        if metadata_str.len() > MAX_METADATA_SIZE {
            return Err(format!(
                "Metadata too large ({} bytes). Maximum allowed: {} bytes",
                metadata_str.len(), MAX_METADATA_SIZE
            ));
        }
        
        // Validate metadata is object or null (not array, string, etc.)
        if !self.metadata.is_null() && !self.metadata.is_object() {
            return Err("Metadata must be a JSON object or null".to_string());
        }
        
        Ok(())
    }
}

fn default_channel() -> String {
    "mobile_app".to_string()
}

#[derive(Debug, Serialize)]
pub struct GamificationResponse {
    pub lumis_earned: i32,
    pub total_lumis: i32,
    pub xp_earned: i32,
    pub current_level: i32,
    pub level_name: String,
    pub streaks: serde_json::Value,
    pub achievements_unlocked: serde_json::Value,
    pub active_events: serde_json::Value,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserDashboard {
    pub user_id: Option<i32>,
    pub email: Option<String>,
    pub total_lumis: Option<i32>,
    pub current_level: Option<i32>,
    pub level_name: Option<String>,
    pub level_description: Option<String>,
    pub level_color: Option<String>,
    pub level_benefits: Option<serde_json::Value>,
    pub level_min_points: Option<i32>,  // Minimum points for current level
    pub level_max_points: Option<i32>,  // Maximum points for current level
    pub next_level_hint: Option<String>,
    pub lumis_to_next_level: Option<i32>,
    pub next_level_name: Option<String>,
    pub active_streaks: Option<serde_json::Value>,
    pub active_missions_count: Option<i64>,
    pub completed_missions_count: Option<i64>,
    pub total_achievements: Option<i64>,
    pub recent_activity: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Mission {
    pub mission_code: Option<String>,
    pub mission_name: Option<String>,
    pub mission_type: Option<String>,
    pub description: Option<String>,
    pub current_progress: Option<i32>,
    pub target_count: Option<i32>,
    pub reward_lumis: Option<i32>,
    pub due_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub progress_percentage: Option<f64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Event {
    pub event_code: Option<String>,
    pub event_name: Option<String>,
    pub event_type: Option<String>,
    pub starts_in_minutes: Option<f64>,
    pub ends_in_minutes: Option<f64>,
    pub multiplier: Option<Decimal>,
    pub description: Option<String>,
    pub is_active_now: Option<bool>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Achievement {
    pub achievement_code: Option<String>,
    pub achievement_name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub reward_lumis: Option<i32>,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub is_unlocked: Option<bool>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MechanicInfo {
    pub mechanic_code: Option<String>,
    pub mechanic_name: Option<String>,
    pub mechanic_type: Option<String>,
    pub description: Option<String>,
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub how_it_works: Option<serde_json::Value>,
    pub rewards: Option<serde_json::Value>,
    pub tips: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LeaderboardEntry {
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub total_lumis: Option<i64>,
    pub current_level: Option<i32>,
    pub level_name: Option<String>,
    pub rank: Option<i64>,
}

// Helper struct for database queries
#[derive(FromRow)]
struct GamificationResult {
    lumis_earned: Option<i32>,
    xp_earned: Option<i32>,
    streak_info: Option<serde_json::Value>,
    achievements_unlocked: Option<serde_json::Value>,
    active_events: Option<serde_json::Value>,
    message: Option<String>,
}

// ============================================================================
// API HANDLERS
// ============================================================================

/// Main endpoint to track any user action
#[axum::debug_handler]
pub async fn track_action(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<TrackActionRequest>,
) -> ResponseJson<GamificationResponse> {
    let start_time = Utc::now();
    
    // Validate request using the new validation method
    if let Err(validation_error) = request.validate() {
        return Err(ApiError::validation_error(&validation_error));
    }
    
    // Call the database function
    let result = sqlx::query_as!(
        GamificationResult,
        r#"
        SELECT 
            lumis_earned,
            xp_earned,
            streak_info,
            achievements_unlocked,
            active_events,
            message
        FROM gamification.track_user_action($1, $2, $3, $4)
        "#,
        current_user.user_id as i32,
        request.action,
        request.channel,
        request.metadata
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to track action: {:?}", e);
        ApiError::database_error("Failed to track action")
    })?;
    
    // Get user's updated total lumis and level info
    let user_info = sqlx::query!(
        r#"
        SELECT 
            COALESCE(us.total_xp, 0) as total_lumis,
            COALESCE(us.current_level_id, 1) as level,
            COALESCE(l.level_name, 'Chispa Lüm') as name
        FROM public.dim_users u
        LEFT JOIN gamification.user_status us ON u.id = us.user_id
        LEFT JOIN gamification.dim_user_levels l ON us.current_level_id = l.level_id
        WHERE u.id = $1
        "#,
        current_user.user_id as i32
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch user info: {}", e)))?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    let response = GamificationResponse {
        lumis_earned: result.lumis_earned.unwrap_or(0),
        total_lumis: user_info.total_lumis.unwrap_or(0),
        xp_earned: result.xp_earned.unwrap_or(0),
        current_level: user_info.level.unwrap_or(1),
        level_name: user_info.name.unwrap_or("Chispa Lüm".to_string()),
        streaks: result.streak_info.unwrap_or_else(|| serde_json::json!({})),
        achievements_unlocked: result.achievements_unlocked.unwrap_or_else(|| serde_json::json!([])),
        active_events: result.active_events.unwrap_or_else(|| serde_json::json!([])),
        message: result.message,
    };
    
    Ok(Json(ApiResponse::success(response, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

/// Get complete gamification dashboard for user
#[axum::debug_handler]
pub async fn get_dashboard(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> ResponseJson<UserDashboard> {
    let start_time = Utc::now();

    // Refrescar rachas de forma best-effort para que el frontend vea datos actuales
    // aunque el batch/cron no esté corriendo o el FE no llame explícitamente /track.
    //
    // - daily_login: se actualiza 1 vez por día (idempotente si se llama varias veces)
    // - consistent_month: recalcula basado en facturas
    let user_id = current_user.user_id as i32;

    if let Err(e) = sqlx::query("SELECT gamification.update_daily_login_streak($1)")
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
    {
        tracing::warn!("Failed to refresh daily_login streak for user {}: {}", user_id, e);
    }

    if let Err(e) = sqlx::query("SELECT gamification.update_user_streaks($1)")
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
    {
        tracing::warn!("Failed to refresh consistent_month streak for user {}: {}", user_id, e);
    }
    
    // Usar la vista materializada que tiene los datos correctos
    let dashboard = sqlx::query_as!(
        UserDashboard,
        r#"
        SELECT 
            user_id::int4,
            email,
            total_invoices::int4 as total_lumis,
            current_level::int4,
            level_name,
            CONCAT(level_name, ' - Basado en ', total_invoices, ' facturas') as level_description,
            level_color,
            level_benefits,
            level_min_points::int4,
            level_max_points::int4,
            CONCAT('Faltan ', invoices_to_next_level, ' facturas para ', next_level_name) as next_level_hint,
            invoices_to_next_level::int4 as lumis_to_next_level,
            next_level_name,
            active_streaks,
            active_mechanics_count::int8 as active_missions_count,
            0::int8 as completed_missions_count,
            0::int8 as total_achievements,
            '[]'::jsonb as recent_activity
        FROM gamification.v_user_dashboard
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch dashboard: {}", e)))?;
    
    let dashboard = dashboard.ok_or_else(|| {
        ApiError::not_found("User not found or inactive")
    })?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(dashboard, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

/// Get user's active and completed missions
#[axum::debug_handler]
pub async fn get_missions(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> ResponseJson<Vec<Mission>> {
    let start_time = Utc::now();
    
    let missions = sqlx::query_as!(
        Mission,
        r#"
        SELECT 
            m.mechanic_code as mission_code,
            m.mechanic_name as mission_name,
            'mission' as mission_type,
            m.description,
            (um.progress->>'current')::int4 as current_progress,
            (m.config->>'target_count')::int4 as target_count,
            m.reward_lumis,
            m.end_date::date as due_date,
            um.status,
            CASE 
                WHEN (m.config->>'target_count')::int4 > 0 THEN 
                    ((um.progress->>'current')::float / (m.config->>'target_count')::float * 100)::float
                ELSE 0.0
            END as progress_percentage
        FROM gamification.user_mechanics um
        JOIN gamification.dim_mechanics m ON um.mechanic_id = m.mechanic_id
        WHERE um.user_id = $1 AND m.mechanic_type = 'mission'
        ORDER BY 
            CASE um.status 
                WHEN 'active' THEN 1 
                WHEN 'completed' THEN 2 
                ELSE 3 
            END,
            m.end_date ASC NULLS LAST,
            um.started_at DESC
        "#,
        current_user.user_id as i32
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch missions: {}", e)))?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(missions, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

/// Get current and upcoming events
#[axum::debug_handler]
pub async fn get_events(
    State(state): State<Arc<AppState>>,
    Extension(_current_user): Extension<CurrentUser>,
) -> ResponseJson<Vec<Event>> {
    let start_time = Utc::now();
    
    let events = sqlx::query_as!(
        Event,
        r#"
        SELECT 
            mechanic_code as event_code,
            mechanic_name as event_name,
            (config->>'event_type') as event_type,
            (EXTRACT(EPOCH FROM (start_date - NOW())) / 60)::float8 as starts_in_minutes,
            (EXTRACT(EPOCH FROM (end_date - NOW())) / 60)::float8 as ends_in_minutes,
            (config->>'multiplier')::numeric as multiplier,
            description,
            (NOW() BETWEEN start_date AND end_date) as is_active_now
        FROM gamification.dim_mechanics
        WHERE is_active = true
        AND mechanic_type = 'event'
        AND (
            (NOW() BETWEEN start_date AND end_date) OR -- Currently active
            (start_date > NOW() AND start_date < NOW() + INTERVAL '24 hours') -- Starting within 24h
        )
        ORDER BY 
            CASE WHEN NOW() BETWEEN start_date AND end_date THEN 1 ELSE 2 END,
            start_date ASC
        LIMIT 10
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch events: {}", e)))?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(events, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

/// Get user's achievements (unlocked and available)
#[axum::debug_handler]
pub async fn get_achievements(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> ResponseJson<Vec<Achievement>> {
    let start_time = Utc::now();
    
    let achievements = sqlx::query_as!(
        Achievement,
        r#"
        SELECT 
            m.mechanic_code as achievement_code,
            m.mechanic_name as achievement_name,
            m.description,
            (m.config->>'category') as category,
            m.difficulty,
            m.reward_lumis,
            um.completed_at as unlocked_at,
            (um.user_id IS NOT NULL AND um.status IN ('completed', 'claimed')) as is_unlocked
        FROM gamification.dim_mechanics m
        LEFT JOIN gamification.user_mechanics um 
            ON m.mechanic_id = um.mechanic_id 
            AND um.user_id = $1
        WHERE m.is_active = true
        AND m.mechanic_type = 'achievement'
        ORDER BY 
            is_unlocked DESC,
            CASE m.difficulty 
                WHEN 'bronze' THEN 1 
                WHEN 'silver' THEN 2 
                WHEN 'gold' THEN 3 
                WHEN 'platinum' THEN 4 
                ELSE 5 
            END,
            m.mechanic_name
        "#,
        current_user.user_id as i32
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch achievements: {}", e)))?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(achievements, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

/// Get explanations for all game mechanics
#[axum::debug_handler]
pub async fn get_mechanics_info(
    State(state): State<Arc<AppState>>,
    Extension(_current_user): Extension<CurrentUser>,
) -> ResponseJson<Vec<MechanicInfo>> {
    let start_time = Utc::now();
    
    let mechanics = sqlx::query_as!(
        MechanicInfo,
        r#"
        SELECT 
            mechanic_code,
            mechanic_name,
            mechanic_type,
            description,
            mechanic_name as display_name,
            description as short_description,
            description as long_description,
            config as how_it_works,
            jsonb_build_object('lumis', reward_lumis) as rewards,
            '{}'::jsonb as tips
        FROM gamification.dim_mechanics
        WHERE is_active = true
        ORDER BY 
            CASE mechanic_type 
                WHEN 'streak' THEN 1 
                WHEN 'mission' THEN 2 
                WHEN 'achievement' THEN 3 
                ELSE 4 
            END,
            mechanic_name
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch mechanics: {}", e)))?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(mechanics, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

/// Get leaderboard (top users by lumis)
#[axum::debug_handler]
pub async fn get_leaderboard(
    State(state): State<Arc<AppState>>,
    Extension(_current_user): Extension<CurrentUser>,
    Query(params): Query<LeaderboardQuery>,
) -> ResponseJson<Vec<LeaderboardEntry>> {
    let start_time = Utc::now();
    
    let limit = params.limit.unwrap_or(50).min(100); // Max 100 entries
    let offset = params.offset.unwrap_or(0);
    
    let leaderboard = sqlx::query_as!(
        LeaderboardEntry,
        r#"
        SELECT 
            u.id::int4 as user_id,
            COALESCE(u.name, u.email) as username,
            COALESCE(us.total_xp, 0)::int8 as total_lumis,
            COALESCE(us.current_level_id, 1)::int4 as current_level,
            COALESCE(l.level_name, 'Chispa Lüm') as level_name,
            ROW_NUMBER() OVER (ORDER BY COALESCE(us.total_xp, 0) DESC) as rank
        FROM public.dim_users u
        LEFT JOIN gamification.user_status us ON u.id = us.user_id
        LEFT JOIN gamification.dim_user_levels l ON us.current_level_id = l.level_id
        WHERE u.is_active = true
        ORDER BY total_lumis DESC, u.id ASC
        LIMIT $1 OFFSET $2
        "#,
        limit as i64,
        offset as i64
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch leaderboard: {}", e)))?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(leaderboard, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

// ============================================================================
// QUERY PARAMETERS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// ROUTER CREATION
// ============================================================================

/// Create router for gamification endpoints
pub fn create_gamification_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/gamification/track", post(track_action))
        .route("/api/v4/gamification/dashboard", get(get_dashboard))
        .route("/api/v4/gamification/missions", get(get_missions))
        .route("/api/v4/gamification/events", get(get_events))
        .route("/api/v4/gamification/achievements", get(get_achievements))
        .route("/api/v4/gamification/mechanics", get(get_mechanics_info))
        .route("/api/v4/gamification/leaderboard", get(get_leaderboard))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_track_action_request_deserialize() {
        let json = json!({
            "action": "daily_login",
            "channel": "mobile_app",
            "metadata": {"test": true}
        });
        
        let request: TrackActionRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.action, "daily_login");
        assert_eq!(request.channel, "mobile_app");
    }
    
    #[test]
    fn test_track_action_request_defaults() {
        let json = json!({
            "action": "survey_complete"
        });
        
        let request: TrackActionRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.action, "survey_complete");
        assert_eq!(request.channel, "mobile_app");
        assert_eq!(request.metadata, serde_json::Value::Null);
    }
}

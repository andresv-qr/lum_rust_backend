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
    
    // Validate action type
    if !["daily_login", "invoice_upload", "survey_complete"].contains(&request.action.as_str()) {
        return Err(ApiError::validation_error("Invalid action type"));
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
            COALESCE(p.total_xp, 0) as total_lumis,
            COALESCE(p.current_level, 1) as level,
            COALESCE(l.level_name, 'Chispa Lüm') as name
        FROM public.dim_users u
        LEFT JOIN gamification.fact_user_progression p ON u.id = p.user_id
        LEFT JOIN gamification.dim_user_levels l ON p.current_level = l.level_id
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
    
    // Usar la vista materializada que tiene los datos correctos
    let dashboard = sqlx::query_as!(
        UserDashboard,
        r#"
        SELECT 
            lum.user_id::int4 as user_id,
            u.email,
            lum.total_invoices::int4 as total_lumis,  -- Representa facturas, no balance
            lum.current_level::int4 as current_level,
            lum.level_name,
            CONCAT(lum.level_name, ' - Basado en ', lum.total_invoices, ' facturas') as level_description,
            lum.level_color,
            lum.level_benefits,
            CONCAT('Faltan ', lum.lumis_to_next_level, ' facturas para ', lum.next_level_name) as next_level_hint,
            lum.lumis_to_next_level::int4 as lumis_to_next_level,
            lum.next_level_name,
            -- Obtener streaks activas reales
            COALESCE(
                (SELECT jsonb_agg(
                    jsonb_build_object(
                        'type', streak_type,
                        'current', current_count,
                        'max', max_count,
                        'last_date', last_activity_date
                    )
                ) FROM gamification.fact_user_streaks 
                WHERE user_id = lum.user_id AND is_active = true), 
                '[]'::jsonb
            ) as active_streaks,
            -- Misiones (mantener compatibilidad)
            0::int8 as active_missions_count,
            0::int8 as completed_missions_count,
            0::int8 as total_achievements,
            '[]'::jsonb as recent_activity
        FROM gamification.vw_user_lum_levels lum
        JOIN public.dim_users u ON lum.user_id = u.id
        WHERE lum.user_id = $1 AND u.is_active = true
        "#,
        current_user.user_id as i32
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
            m.mission_code,
            m.mission_name,
            m.mission_type,
            COALESCE(r.requirements_json->>'description', m.mission_name) as description,
            m.current_progress,
            m.target_count,
            m.reward_lumis,
            m.due_date,
            m.status,
            CASE 
                WHEN m.target_count > 0 THEN (m.current_progress::float / m.target_count::float * 100)::float
                ELSE 0.0
            END as progress_percentage
        FROM gamification.fact_user_missions m
        LEFT JOIN gamification.dim_rewards_config r ON m.mission_code = r.reward_code
        WHERE m.user_id = $1
        ORDER BY 
            CASE m.status 
                WHEN 'active' THEN 1 
                WHEN 'completed' THEN 2 
                ELSE 3 
            END,
            m.due_date ASC NULLS LAST,
            m.created_at DESC
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
            event_code,
            event_name,
            event_type,
            (EXTRACT(EPOCH FROM (start_date - NOW())) / 60)::float8 as starts_in_minutes,
            (EXTRACT(EPOCH FROM (end_date - NOW())) / 60)::float8 as ends_in_minutes,
            multiplier,
            COALESCE(config_json->>'description', event_name) as description,
            (NOW() BETWEEN start_date AND end_date) as is_active_now
        FROM gamification.dim_events
        WHERE is_active = true
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
            a.achievement_code,
            a.achievement_name,
            a.description,
            a.category,
            a.difficulty,
            a.reward_lumis,
            ua.unlocked_at,
            (ua.user_id IS NOT NULL) as is_unlocked
        FROM gamification.dim_achievements a
        LEFT JOIN gamification.fact_user_achievements ua 
            ON a.achievement_id = ua.achievement_id 
            AND ua.user_id = $1
        WHERE a.is_active = true
        ORDER BY 
            is_unlocked DESC,
            CASE a.difficulty 
                WHEN 'bronze' THEN 1 
                WHEN 'silver' THEN 2 
                WHEN 'gold' THEN 3 
                WHEN 'platinum' THEN 4 
                ELSE 5 
            END,
            a.achievement_name
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
            display_name,
            short_description,
            long_description,
            how_it_works,
            rewards,
            tips
        FROM gamification.v_mechanics_info
        ORDER BY 
            CASE mechanic_type 
                WHEN 'streak' THEN 1 
                WHEN 'milestone' THEN 2 
                WHEN 'mission' THEN 3 
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
            COALESCE(p.total_xp, 0)::int8 as total_lumis,
            COALESCE(p.current_level, 1)::int4 as current_level,
            COALESCE(l.level_name, 'Chispa Lüm') as level_name,
            ROW_NUMBER() OVER (ORDER BY COALESCE(p.total_xp, 0) DESC) as rank
        FROM public.dim_users u
        LEFT JOIN gamification.fact_user_progression p ON u.id = p.user_id
        LEFT JOIN gamification.dim_user_levels l ON p.current_level = l.level_id
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

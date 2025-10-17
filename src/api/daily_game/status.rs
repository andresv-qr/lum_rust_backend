use axum::{
    extract::{State, Extension},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use chrono_tz::America::Panama;
use tracing::{info, error};

use std::sync::Arc;
use crate::{
    api::daily_game::templates::{DailyGameStatusResponse, DailyGameStats},
    api::common::SimpleApiResponse,
    state::AppState,
    middleware::CurrentUser,
};

/// GET /v4/daily-game/status
/// 
/// Obtiene el estado actual del juego diario para el usuario:
/// - Si puede jugar hoy
/// - Última fecha de juego
/// - Recompensa de hoy (si ya jugó)
/// - Estadísticas básicas
pub async fn handle_status(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<SimpleApiResponse<DailyGameStatusResponse>>, (StatusCode, Json<SimpleApiResponse<()>>)> {
    
    let user_id = current_user.user_id;
    info!("📊 Daily game status request from user {}", user_id);
    
    // Obtener fecha actual en zona horaria de Panamá
    let today = Utc::now().with_timezone(&Panama).date_naive();
    
    // Query optimizada: obtiene toda la info en una sola consulta usando CTEs
    let result = sqlx::query!(
        r#"
        WITH today_play AS (
            SELECT lumis_won
            FROM rewards.fact_daily_game_plays
            WHERE user_id = $1 AND play_date = $2
        ),
        stats AS (
            SELECT 
                COUNT(*) as total_plays,
                COALESCE(SUM(lumis_won), 0) as total_lumis,
                COUNT(*) FILTER (WHERE lumis_won = 5) as golden_stars
            FROM rewards.fact_daily_game_plays
            WHERE user_id = $1
        ),
        last_play AS (
            SELECT play_date, lumis_won
            FROM rewards.fact_daily_game_plays
            WHERE user_id = $1 AND play_date < $2
            ORDER BY play_date DESC
            LIMIT 1
        )
        SELECT 
            (SELECT lumis_won FROM today_play) as "todays_reward?: i16",
            (SELECT total_plays FROM stats) as "total_plays!: i64",
            (SELECT total_lumis FROM stats) as "total_lumis!: i64",
            (SELECT golden_stars FROM stats) as "golden_stars!: i64",
            (SELECT play_date FROM last_play) as last_played,
            (SELECT lumis_won FROM last_play) as "last_reward?: i16"
        "#,
        user_id,
        today
    )
    .fetch_one(&state.db_pool)
    .await;
    
    let query_result = match result {
        Ok(r) => r,
        Err(e) => {
            error!("❌ Failed to fetch daily game status: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SimpleApiResponse::<()>::error("Failed to fetch status")),
            ));
        }
    };
    
    // Determinar si puede jugar
    let can_play = query_result.todays_reward.is_none();
    let has_played_today = !can_play;
    
    let todays_reward = query_result.todays_reward.map(|r| r as i32);
    
    // Construir estadísticas
    let stats = if query_result.total_plays > 0 {
        Some(DailyGameStats {
            total_plays: query_result.total_plays as i32,
            total_lumis_won: query_result.total_lumis as i32,
            golden_stars_captured: query_result.golden_stars as i32,
        })
    } else {
        None
    };
    
    info!(
        "📊 User {} status: can_play={}, has_played={}, last_played={:?}, stats={:?}",
        user_id, can_play, has_played_today, query_result.last_played, stats
    );
    
    Ok(Json(SimpleApiResponse::success(DailyGameStatusResponse {
        can_play_today: can_play,
        has_played_today,
        last_played_date: query_result.last_played,
        todays_reward,
        stats,
    })))
}

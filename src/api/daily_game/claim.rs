use axum::{
    extract::{State, Extension},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use chrono_tz::America::Panama;
use tracing::{info, warn, error};
// use rust_decimal::Decimal; // Unused - comentado

use std::sync::Arc;
use crate::{
    api::daily_game::templates::{DailyGameClaimRequest, DailyGameClaimResponse},
    api::common::SimpleApiResponse,
    state::AppState,
    middleware::CurrentUser,
};

/// POST /v4/daily-game/claim
/// 
/// Reclama la recompensa diaria después de que el usuario seleccione una estrella.
/// 
/// Validaciones:
/// - lumis_won debe ser 0, 1, o 5
/// - star_id debe ser star_0 a star_8
/// - Usuario no debe haber jugado hoy (garantizado por UNIQUE constraint)
pub async fn handle_claim(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<DailyGameClaimRequest>,
) -> Result<Json<SimpleApiResponse<DailyGameClaimResponse>>, (StatusCode, Json<SimpleApiResponse<()>>)> {
    
    let user_id = current_user.user_id;
    info!("🎮 Daily game claim request from user {}: star_id={}, lumis_won={}", 
          user_id, request.star_id, request.lumis_won);
    
    // 1. Validar request
    if let Err(e) = request.validate() {
        warn!("❌ Validation failed for user {}: {}", user_id, e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(SimpleApiResponse::<()>::error(&e)),
        ));
    }
    
    // 2. Obtener fecha/hora actual en zona horaria de Panamá
    let now_panama = Utc::now().with_timezone(&Panama);
    let today = now_panama.date_naive();
    let play_time = now_panama.time();
    
    info!("📅 Play date: {}, time: {}", today, play_time);
    
    // 3. Iniciar transacción
    let mut tx = match state.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("❌ Failed to start transaction: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SimpleApiResponse::<()>::error("Database error")),
            ));
        }
    };
    
    // 4. Insertar en fact_daily_game_plays
    // El constraint UNIQUE valida automáticamente "ya jugó hoy"
    let play_result = sqlx::query!(
        r#"
        INSERT INTO rewards.fact_daily_game_plays
        (user_id, play_date, play_time, star_id, lumis_won)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
        user_id,
        today,
        play_time,
        request.star_id,
        request.lumis_won as i16
    )
    .fetch_one(&mut *tx)
    .await;
    
    let play_id = match play_result {
        Ok(record) => {
            info!("✅ Inserted daily game play with id: {}", record.id);
            record.id
        },
        Err(e) => {
            // Detectar violación de UNIQUE constraint
            let error_msg = e.to_string();
            if error_msg.contains("unique_user_daily_play") {
                warn!("⚠️ User {} already played today", user_id);
                return Err((
                    StatusCode::CONFLICT,
                    Json(SimpleApiResponse::<()>::error_with_code(
                        "ALREADY_PLAYED_TODAY",
                        "Ya jugaste hoy. Vuelve mañana a las 00:00."
                    )),
                ));
            }
            
            error!("❌ Database error inserting play: {}", error_msg);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SimpleApiResponse::<()>::error("Failed to record play")),
            ));
        }
    };
    
    // 5. Registrar en fact_accumulations (solo si ganó Lümis)
    if request.lumis_won > 0 {
        let accum_key = format!("daily_game_{}_{}", user_id, today);
        
        let accum_result = sqlx::query!(
            r#"
            INSERT INTO rewards.fact_accumulations 
            (user_id, accum_type, accum_key, dtype, quantity, date, accum_id)
            VALUES ($1, 'daily_game', $2, 'points', $3, NOW(), 10)
            "#,
            user_id as i32,
            accum_key,
            rust_decimal::Decimal::from(request.lumis_won)
        )
        .execute(&mut *tx)
        .await;
        
        match accum_result {
            Ok(_) => {
                info!("✅ Recorded accumulation: {} Lümis for user {}", request.lumis_won, user_id);
            },
            Err(e) => {
                error!("❌ Failed to record accumulation: {}", e);
                // Rollback será automático al salir del scope
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SimpleApiResponse::<()>::error("Failed to credit Lümis")),
                ));
            }
        }
    } else {
        info!("ℹ️ No Lümis to accumulate (empty star)");
    }
    
    // 6. Commit transacción
    if let Err(e) = tx.commit().await {
        error!("❌ Failed to commit transaction: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SimpleApiResponse::<()>::error("Transaction failed")),
        ));
    }
    
    // 7. Consultar balance actualizado (trigger ya lo actualizó)
    let new_balance = match crate::api::gamification_service::get_user_balance(&state.db_pool, user_id).await {
        Ok(balance) => balance,
        Err(e) => {
            warn!("⚠️ Failed to get updated balance: {}", e);
            0 // Fallback, pero la jugada ya se registró
        }
    };
    
    info!("💰 User {} new balance: {} Lümis (added: {})", user_id, new_balance, request.lumis_won);
    
    // 8. Construir respuesta
    let message = if request.lumis_won == 0 {
        "¡Ups! Estrella vacía. Mejor suerte mañana. 🌟".to_string()
    } else if request.lumis_won == 5 {
        format!("¡Increíble! 🌟✨ ¡Encontraste la estrella dorada! +{} Lümis", request.lumis_won)
    } else {
        format!("¡Genial! +{} Lümi ganado. 🌟", request.lumis_won)
    };
    
    Ok(Json(SimpleApiResponse::success_with_message(
        DailyGameClaimResponse {
            lumis_added: request.lumis_won,
            new_balance,
            play_id,
        },
        message,
    )))
}

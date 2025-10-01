use axum::{
    extract::{Query, State, Extension},
    response::Json,
    routing::get,
    Router,
    http::StatusCode,
};
use serde_json::json;
use crate::api::common::ApiResponse;
use crate::middleware::auth::CurrentUser;
use crate::domains::rewards::service::UserSummaryService;
use crate::models::rewards::{UserSummaryQuery, UserSummaryResponse};
use crate::AppState;
use std::sync::Arc;
use tracing::{info, error};
use uuid::Uuid;

pub fn create_rewards_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/summary", get(get_user_summary))
        .route("/balance", get(get_user_balance))
}

/// GET /api/v4/rewards/summary - Obtener resumen completo del usuario
/// 
/// **Headers:** `Authorization: Bearer <jwt_token>` **REQUERIDO**
/// 
/// **Query Parameters:**
/// - `include_trends`: Incluir análisis de tendencias (default: true)

#[axum::debug_handler]
async fn get_user_summary(
    State(app_state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<UserSummaryQuery>,
) -> Result<Json<ApiResponse<UserSummaryResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    info!("Getting user summary for user_id: {} with query: {:?}", current_user.user_id, query);
    let summary_service = UserSummaryService::new(app_state.db_pool.clone());
    let user_id_i32 = current_user.user_id as i32;
    match summary_service.get_user_summary(user_id_i32, Some(query)).await {
        Ok(summary_response) => {
            let elapsed = start_time.elapsed();
            info!("User summary retrieved successfully for user {} in {:?}ms", current_user.user_id, elapsed.as_millis());
            Ok(Json(ApiResponse::success(
                summary_response,
                request_id,
                Some(elapsed.as_millis() as u64),
                false
            )))
        },
        Err(e) => {
            error!("Error fetching user summary for user {}: {:?}", current_user.user_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[axum::debug_handler]
async fn get_user_balance(
    State(app_state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    info!("Getting user balance for user_id: {}", current_user.user_id);
    match crate::domains::rewards::get_user_balance(&app_state.db_pool, current_user.user_id as i64).await {
        Ok(balance) => {
            let elapsed = start_time.elapsed();
            info!("User balance retrieved successfully for user {} in {:?}ms: {} Lümis", current_user.user_id, elapsed.as_millis(), balance);
            Ok(Json(ApiResponse::success(
                json!({
                    "balance": balance,
                    "currency": "Lümis",
                    "user_id": current_user.user_id
                }),
                request_id,
                Some(elapsed.as_millis() as u64),
                false
            )))
        },
        Err(e) => {
            error!("Error fetching user balance for user {}: {}", current_user.user_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ...existing code...
// ...existing code...

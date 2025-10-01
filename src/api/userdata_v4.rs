use axum::{
    extract::{State, Extension},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use chrono::{Utc, DateTime};
use uuid::Uuid;

use axum::middleware::from_fn;
use crate::{
    state::AppState,
    middleware::{CurrentUser, extract_current_user},
    api::common::ApiResponse,
};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserData {
    pub name: Option<String>,
    pub email: Option<String>,
    pub date_of_birth: Option<String>, // VARCHAR in DB
    pub country_origin: Option<String>,
    pub country_residence: Option<String>,
    pub segment_activity: Option<String>,
    pub genre: Option<String>,
    pub ws_id: Option<String>,
    pub updated_at: Option<DateTime<Utc>>, // timestamp with time zone in DB
}

/// GET /api/v4/userdata - Obtener datos del usuario desde public.dim_users
pub async fn get_user_data(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<UserData>>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    info!("Fetching user data for user_id: {}", current_user.user_id);

    let query = r#"
        SELECT 
            name,
            email,
            date_of_birth,
            country_origin,
            country_residence,
            segment_activity,
            genre,
            ws_id,
            updated_at
        FROM public.dim_users
        WHERE id = $1
    "#;    let result = sqlx::query_as::<_, UserData>(query)
        .bind(current_user.user_id) // Use i64 directly, no cast needed
        .fetch_optional(&state.db_pool)
        .await;

    match result {
        Ok(Some(user_data)) => {
            let elapsed = start_time.elapsed();
            info!(
                "User data retrieved in {:?}ms for user_id: {} - email: {:?}", 
                elapsed.as_millis(), 
                current_user.user_id,
                user_data.email
            );

            let response = ApiResponse {
                success: true,
                data: Some(user_data),
                error: None,
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: Some(elapsed.as_millis() as u64),
                cached: false,
            };

            Ok(Json(response))
        },
        Ok(None) => {
            info!("No user data found for user_id: {}", current_user.user_id);
            
            // Crear datos vacíos para usuario no encontrado
            let empty_data = UserData {
                name: None,
                email: None,
                date_of_birth: None,
                country_origin: None,
                country_residence: None,
                segment_activity: None,
                genre: None,
                ws_id: None,
                updated_at: None,
            };

            let response = ApiResponse {
                success: true,
                data: Some(empty_data),
                error: None,
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: Some(start_time.elapsed().as_millis() as u64),
                cached: false,
            };

            Ok(Json(response))
        },
        Err(e) => {
            error!("Database error fetching user data for user_id {}: {}", current_user.user_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Router para userdata v4
pub fn create_userdata_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/userdata", get(get_user_data))
        .route_layer(from_fn(extract_current_user))
}

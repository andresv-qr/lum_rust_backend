use axum::{
    extract::{State, Extension},
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use chrono::{Utc, DateTime, FixedOffset};
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};

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

/// Estructura para actualización parcial de datos de usuario
#[derive(Debug, Deserialize)]
pub struct UpdateUserData {
    pub name: Option<String>,
    pub date_of_birth: Option<String>,
    pub country_origin: Option<String>,
    pub country_residence: Option<String>,
    pub segment_activity: Option<String>,
    pub genre: Option<String>,
    pub ws_id: Option<String>,
}

/// PUT /api/v4/userdata - Actualizar datos del usuario en public.dim_users
pub async fn update_user_data(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<UpdateUserData>,
) -> Result<Json<ApiResponse<UserData>>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    info!("Updating user data for user_id: {} with payload: {:?}", current_user.user_id, payload);

    // Crear timestamp con timezone GMT-5
    let gmt_minus_5 = FixedOffset::west_opt(5 * 3600).unwrap();
    let now_gmt_minus_5 = Utc::now().with_timezone(&gmt_minus_5);

    // Construir query dinámico solo con campos que no son None
    let mut set_clauses = Vec::new();
    let mut params: Vec<String> = Vec::new();
    let mut param_count = 1;

    if payload.name.is_some() {
        set_clauses.push(format!("name = ${}", param_count));
        params.push(payload.name.clone().unwrap());
        param_count += 1;
    }
    if payload.date_of_birth.is_some() {
        set_clauses.push(format!("date_of_birth = ${}", param_count));
        params.push(payload.date_of_birth.clone().unwrap());
        param_count += 1;
    }
    if payload.country_origin.is_some() {
        set_clauses.push(format!("country_origin = ${}", param_count));
        params.push(payload.country_origin.clone().unwrap());
        param_count += 1;
    }
    if payload.country_residence.is_some() {
        set_clauses.push(format!("country_residence = ${}", param_count));
        params.push(payload.country_residence.clone().unwrap());
        param_count += 1;
    }
    if payload.segment_activity.is_some() {
        set_clauses.push(format!("segment_activity = ${}", param_count));
        params.push(payload.segment_activity.clone().unwrap());
        param_count += 1;
    }
    if payload.genre.is_some() {
        set_clauses.push(format!("genre = ${}", param_count));
        params.push(payload.genre.clone().unwrap());
        param_count += 1;
    }
    if payload.ws_id.is_some() {
        set_clauses.push(format!("ws_id = ${}", param_count));
        params.push(payload.ws_id.clone().unwrap());
        param_count += 1;
    }

    // Si no hay campos para actualizar, retornar error
    if set_clauses.is_empty() {
        error!("No fields provided to update for user_id: {}", current_user.user_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Agregar updated_at
    set_clauses.push(format!("updated_at = ${}", param_count));
    
    // Construir query completo
    let query = format!(
        r#"
        UPDATE public.dim_users
        SET {}
        WHERE id = ${}
        RETURNING name, email, date_of_birth, country_origin, country_residence, 
                  segment_activity, genre, ws_id, updated_at
        "#,
        set_clauses.join(", "),
        param_count + 1
    );

    info!("Generated UPDATE query: {}", query);

    // Ejecutar query con bind dinámico
    let mut query_builder = sqlx::query_as::<_, UserData>(&query);
    
    for param in params {
        query_builder = query_builder.bind(param);
    }
    
    query_builder = query_builder.bind(now_gmt_minus_5);
    query_builder = query_builder.bind(current_user.user_id);

    let result = query_builder
        .fetch_optional(&state.db_pool)
        .await;

    match result {
        Ok(Some(updated_data)) => {
            let elapsed = start_time.elapsed();
            info!(
                "User data updated in {:?}ms for user_id: {} - email: {:?}", 
                elapsed.as_millis(), 
                current_user.user_id,
                updated_data.email
            );

            let response = ApiResponse {
                success: true,
                data: Some(updated_data),
                error: None,
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: Some(elapsed.as_millis() as u64),
                cached: false,
            };

            Ok(Json(response))
        },
        Ok(None) => {
            error!("User not found for user_id: {}", current_user.user_id);
            Err(StatusCode::NOT_FOUND)
        },
        Err(e) => {
            error!("Database error updating user data for user_id {}: {}", current_user.user_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Estructura para cambio de contraseña
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
    pub confirmation_password: String,
}

/// Respuesta de cambio de contraseña
#[derive(Debug, Serialize)]
pub struct PasswordChangeResponse {
    pub user_id: i64,
    pub email: String,
    pub password_updated_at: String,
    pub message: String,
}

/// Validar fortaleza de contraseña
fn validate_password_strength(password: &str) -> Result<(), StatusCode> {
    // Longitud mínima
    if password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Longitud máxima
    if password.len() > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Al menos una mayúscula
    if !password.chars().any(|c| c.is_uppercase()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Al menos una minúscula
    if !password.chars().any(|c| c.is_lowercase()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Al menos un número
    if !password.chars().any(|c| c.is_numeric()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Al menos un carácter especial
    let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
    if !password.chars().any(|c| special_chars.contains(c)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    Ok(())
}

/// PUT /api/v4/userdata/password - Cambiar contraseña con autenticación JWT + contraseña actual
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<PasswordChangeResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    
    info!(
        request_id = %request_id,
        user_id = current_user.user_id,
        email = %current_user.email,
        "Password change request initiated"
    );

    // 1. Validar que las contraseñas nuevas coincidan
    if payload.new_password != payload.confirmation_password {
        warn!(
            request_id = %request_id,
            user_id = current_user.user_id,
            "Password confirmation mismatch"
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // 2. Validar formato de nueva contraseña
    if let Err(_) = validate_password_strength(&payload.new_password) {
        warn!(
            request_id = %request_id,
            user_id = current_user.user_id,
            "Password does not meet strength requirements"
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // 3. Obtener hash de contraseña actual del usuario
    let user_result = sqlx::query!(
        r#"
        SELECT password_hash 
        FROM public.dim_users 
        WHERE id = $1
        "#,
        current_user.user_id
    )
    .fetch_optional(&state.db_pool)
    .await;

    let user = match user_result {
        Ok(Some(u)) => u,
        Ok(None) => {
            error!(
                request_id = %request_id,
                user_id = current_user.user_id,
                "User not found in database"
            );
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                user_id = current_user.user_id,
                error = %e,
                "Database error fetching user"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // 4. Verificar que el usuario tiene contraseña establecida
    let password_hash = match user.password_hash {
        Some(hash) => hash,
        None => {
            warn!(
                request_id = %request_id,
                user_id = current_user.user_id,
                "User does not have password set (OAuth user)"
            );
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 5. Verificar contraseña actual
    let is_valid = match verify(&payload.current_password, &password_hash) {
        Ok(valid) => valid,
        Err(e) => {
            error!(
                request_id = %request_id,
                user_id = current_user.user_id,
                error = %e,
                "Error verifying password"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if !is_valid {
        warn!(
            request_id = %request_id,
            user_id = current_user.user_id,
            "Current password is incorrect"
        );
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 6. Verificar que nueva contraseña sea diferente de la actual
    let same_password = verify(&payload.new_password, &password_hash).unwrap_or(false);
    if same_password {
        warn!(
            request_id = %request_id,
            user_id = current_user.user_id,
            "New password is the same as current password"
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // 7. Hash nueva contraseña
    let new_hash = match hash(&payload.new_password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            error!(
                request_id = %request_id,
                user_id = current_user.user_id,
                error = %e,
                "Error hashing new password"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // 8. Actualizar contraseña con timestamp GMT-5
    let gmt_minus_5 = FixedOffset::west_opt(5 * 3600).unwrap();
    let now_gmt_minus_5 = Utc::now().with_timezone(&gmt_minus_5);

    let update_result = sqlx::query!(
        r#"
        UPDATE public.dim_users
        SET password_hash = $1,
            updated_at = $2
        WHERE id = $3
        "#,
        new_hash,
        now_gmt_minus_5,
        current_user.user_id
    )
    .execute(&state.db_pool)
    .await;

    if let Err(e) = update_result {
        error!(
            request_id = %request_id,
            user_id = current_user.user_id,
            error = %e,
            "Database error updating password"
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 9. Retornar respuesta exitosa
    let elapsed = start_time.elapsed();
    
    info!(
        request_id = %request_id,
        user_id = current_user.user_id,
        email = %current_user.email,
        execution_time_ms = elapsed.as_millis(),
        "Password changed successfully"
    );

    let response_data = PasswordChangeResponse {
        user_id: current_user.user_id,
        email: current_user.email.clone(),
        password_updated_at: now_gmt_minus_5.to_rfc3339(),
        message: "Contraseña actualizada exitosamente".to_string(),
    };

    let response = ApiResponse {
        success: true,
        data: Some(response_data),
        error: None,
        request_id,
        timestamp: Utc::now(),
        execution_time_ms: Some(elapsed.as_millis() as u64),
        cached: false,
    };

    Ok(Json(response))
}

/// Router para userdata v4
pub fn create_userdata_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/userdata", get(get_user_data).put(update_user_data))
        .route("/api/v4/userdata/password", put(change_password))
        .route_layer(from_fn(extract_current_user))
}

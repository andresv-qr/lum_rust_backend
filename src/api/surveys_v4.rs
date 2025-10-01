// ============================================
// SURVEY APIs v4 - Implementación Rust/Axum
// ============================================
// Fecha: 2025-08-26
// Descripción: APIs para manejo de encuestas con autenticación JWT
// Esquema: survey

use axum::{
    extract::{Json, Path, Query, State, Extension},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, patch},
    Router,
};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    state::AppState,
    middleware::CurrentUser,
    api::common::{ApiResponse, ApiError},
};

// ============================================
// ESTRUCTURAS DE DATOS
// ============================================

// Estructura para la vista v_user_surveys
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSurvey {
    pub status_id: i32,
    pub user_id: i32,
    pub survey_id: i32,
    pub campaign_id: i32,
    pub survey_title: String,
    pub survey_description: Option<String>,
    pub instructions: Option<String>,
    pub total_questions: i32,
    pub max_attempts: i32,
    pub time_limit_minutes: Option<i32>,
    pub points_per_question: i32,
    pub difficulty: String,
    pub campaign_name: String,
    pub status: Option<String>,
    pub attempts_made: Option<i32>,
    pub total_score: Option<i32>,
    pub correct_answers: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_percentage: Option<f32>,
    pub due_date: Option<DateTime<Utc>>,
    pub assigned_date: Option<DateTime<Utc>>,
}

// Query parameters para listar encuestas
#[derive(Debug, Deserialize)]
pub struct GetUserSurveysQuery {
    pub status: Option<String>,    // pending, in_progress, completed, expired
    pub limit: Option<i32>,        // Default 50
    pub offset: Option<i32>,       // Default 0
}

// Estructura para detalle de encuesta
#[derive(Debug, Serialize, Deserialize)]
pub struct SurveyDetail {
    pub survey_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub total_questions: i32,
    pub time_limit_minutes: Option<i32>,
    pub difficulty: String,
    pub questions: serde_json::Value, // JSONB content
    pub user_status: Option<UserSurveyStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSurveyStatus {
    pub status_id: i32,
    pub status: String,
    pub attempts_made: i32,
    pub total_score: Option<i32>,
    pub correct_answers: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_percentage: Option<f32>,
}

// Estructura para guardar respuestas
#[derive(Debug, Deserialize)]
pub struct SaveSurveyResponseRequest {
    pub survey_id: i32,
    pub responses: serde_json::Value, // JSONB content
    pub is_completed: bool,
    pub total_time_minutes: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SaveSurveyResponseData {
    pub status_id: i32,
    pub survey_id: i32,
    pub status: String,
    pub total_score: Option<i32>,
    pub correct_answers: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================
// API 1: LISTAR ENCUESTAS DEL USUARIO
// ============================================

/// GET /api/v4/surveys
/// Lista todas las encuestas del usuario autenticado
/// Order: completadas primero, luego por due_date
pub async fn get_user_surveys(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<GetUserSurveysQuery>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, StatusCode> {
    let start_time = Utc::now();
    
    // Usar la función de la base de datos para obtener encuestas
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    
    let result = sqlx::query_scalar!(
        r#"
        SELECT survey.api_get_user_surveys($1, $2, NULL, $3, $4)
        "#,
        current_user.user_id as i32,
        params.status,
        limit,
        offset
    )
    .fetch_one(&state.db_pool)
    .await;
    
    match result {
        Ok(surveys_json) => {
            let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
            
            Ok(ResponseJson(ApiResponse {
                success: true,
                data: surveys_json,
                error: None,
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: Some(execution_time.try_into().unwrap()),
                cached: false,
            }))
        }
        Err(e) => {
            eprintln!("Error getting user surveys: {:?}", e);
            Ok(ResponseJson(ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: "Error al obtener encuestas del usuario".to_string(),
                    details: Some(format!("Error: {}", e).into()),
                }),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: None,
                cached: false,
            }))
        }
    }
}

// ============================================
// API 2: DETALLE DE ENCUESTA
// ============================================

/// GET /api/v4/surveys/{survey_id}
/// Obtiene el detalle completo de una encuesta específica
pub async fn get_survey_detail(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Path(survey_id): Path<i32>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, StatusCode> {
    let start_time = Utc::now();
    
    // Usar la función de la base de datos para obtener detalle de encuesta
    let result = sqlx::query_scalar!(
        r#"
        SELECT survey.api_get_survey_details($1, $2)
        "#,
        survey_id,
        current_user.user_id as i32
    )
    .fetch_one(&state.db_pool)
    .await;
    
    match result {
        Ok(Some(survey_json)) => {
            let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
            
            Ok(ResponseJson(ApiResponse {
                success: true,
                data: Some(survey_json),
                error: None,
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: Some(execution_time.try_into().unwrap()),
                cached: false,
            }))
        }
        Ok(None) => {
            Ok(ResponseJson(ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "SURVEY_NOT_FOUND".to_string(),
                    message: "Encuesta no encontrada o inactiva".to_string(),
                    details: None,
                }),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: None,
                cached: false,
            }))
        }
        Err(e) => {
            eprintln!("Error getting survey detail: {:?}", e);
            Ok(ResponseJson(ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: "Error al obtener detalle de encuesta".to_string(),
                    details: Some(format!("Error: {}", e).into()),
                }),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: None,
                cached: false,
            }))
        }
    }
}

// ============================================
// API 3: GUARDAR RESPUESTAS
// ============================================

/// PATCH /api/v4/surveys/responses
/// Guarda respuestas de encuesta (parciales o completas)
pub async fn save_survey_responses(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<SaveSurveyResponseRequest>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, StatusCode> {
    let start_time = Utc::now();
    
    // Solo procesar respuestas completadas por ahora
    if !request.is_completed {
        return Ok(ResponseJson(ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "PARTIAL_RESPONSES_NOT_SUPPORTED".to_string(),
                message: "El guardado de respuestas parciales no está soportado aún".to_string(),
                details: None,
            }),
            request_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            execution_time_ms: None,
            cached: false,
        }));
    }
    
    // Usar la función de la base de datos para guardar respuestas
    let result = sqlx::query_scalar!(
        r#"
        SELECT survey.api_submit_survey_responses($1, $2, $3, $4)
        "#,
        current_user.user_id as i32,
        request.survey_id,
        request.responses,
        request.total_time_minutes
    )
    .fetch_one(&state.db_pool)
    .await;
    
    match result {
        Ok(Some(response_json)) => {
            let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
            
            // Verificar si la función retornó success: true
            if let Some(success) = response_json.get("success").and_then(|v| v.as_bool()) {
                if success {
                    Ok(ResponseJson(ApiResponse {
                        success: true,
                        data: Some(response_json),
                        error: None,
                        request_id: Uuid::new_v4().to_string(),
                        timestamp: Utc::now(),
                        execution_time_ms: Some(execution_time.try_into().unwrap()),
                        cached: false,
                    }))
                } else {
                    // Error de negocio (ej: max attempts reached)
                    let error_code = response_json.get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("BUSINESS_ERROR");
                    let error_message = response_json.get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Error en el procesamiento de la encuesta");
                    
                    Ok(ResponseJson(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(ApiError {
                            code: error_code.to_string(),
                            message: error_message.to_string(),
                            details: Some(response_json),
                        }),
                        request_id: Uuid::new_v4().to_string(),
                        timestamp: Utc::now(),
                        execution_time_ms: Some(execution_time.try_into().unwrap()),
                        cached: false,
                    }))
                }
            } else {
                Ok(ResponseJson(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(ApiError {
                        code: "INVALID_RESPONSE".to_string(),
                        message: "Respuesta inválida de la función de base de datos".to_string(),
                        details: Some(response_json),
                    }),
                    request_id: Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    execution_time_ms: None,
                    cached: false,
                }))
            }
        }
        Ok(None) => {
            Ok(ResponseJson(ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "NULL_RESPONSE".to_string(),
                    message: "La función de base de datos retornó NULL".to_string(),
                    details: None,
                }),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: None,
                cached: false,
            }))
        }
        Err(e) => {
            eprintln!("Error saving survey responses: {:?}", e);
            Ok(ResponseJson(ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: "Error al guardar respuestas de encuesta".to_string(),
                    details: Some(format!("Error: {}", e).into()),
                }),
                request_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                execution_time_ms: None,
                cached: false,
            }))
        }
    }
}

// ============================================
// ROUTER CONFIGURATION
// ============================================

/// Creates the surveys v4 router with all endpoints
pub fn create_surveys_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/surveys", get(get_user_surveys))
        .route("/api/v4/surveys/:survey_id", get(get_survey_detail))
        .route("/api/v4/surveys/responses", patch(save_survey_responses))
}
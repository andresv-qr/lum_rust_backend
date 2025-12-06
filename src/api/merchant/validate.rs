// ============================================================================
// MERCHANT VALIDATION - Validar y confirmar redenciones
// ============================================================================

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    middleware::auth::MerchantClaims,
    state::AppState,
    observability::metrics::{record_merchant_validation, record_redemption_confirmed},
    services::get_push_service,
};

/// Request body for validating a redemption
#[derive(Debug, Deserialize)]
pub struct ValidateRedemptionRequest {
    /// Can be either redemption_code (e.g., "LUMS-A1B2-C3D4-E5F6") or redemption_id (UUID)
    pub code: String,
}

/// Response for validation
#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub success: bool,
    pub valid: bool,
    pub redemption: Option<RedemptionDetails>,
    pub message: String,
}

/// Redemption details for merchant
#[derive(Debug, Serialize)]
pub struct RedemptionDetails {
    pub redemption_id: String,
    pub redemption_code: String,
    pub offer_name: String,
    pub lumis_spent: i32,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub can_confirm: bool,
}

/// Internal struct for query results
#[derive(Debug)]
struct RedemptionQueryResult {
    redemption_id: String,
    redemption_code: String,
    redemption_status: String,
    lumis_spent: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    code_expires_at: chrono::DateTime<chrono::Utc>,
    offer_name: Option<String>,
}

/// Response for confirmation
#[derive(Debug, Serialize)]
pub struct ConfirmationResponse {
    pub success: bool,
    pub message: String,
    pub redemption_id: String,
    pub confirmed_at: String,
}

/// Validate a redemption code
/// 
/// # Endpoint
/// POST /api/v1/merchant/validate
/// 
/// # Authentication
/// Requires merchant JWT token
/// 
/// # Request Body
/// ```json
/// {
///   "code": "LUMS-A1B2-C3D4-E5F6"
/// }
/// ```
/// 
/// # Returns
/// - 200 OK: Validation result (valid or invalid with reason)
/// - 401 Unauthorized: Invalid merchant token
/// - 500 Internal Server Error: Database error
pub async fn validate_redemption(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Json(payload): Json<ValidateRedemptionRequest>,
) -> Result<Json<ValidationResponse>, ApiError> {
    info!("Merchant {} validating redemption code: {}", 
          merchant.merchant_name, payload.code);
    
    // Query the redemption - check if it's a UUID or a code string
    let redemption_opt: Option<RedemptionQueryResult> = match Uuid::parse_str(&payload.code) {
        Ok(uuid) => {
            // Query by redemption_id (UUID)
            let result = sqlx::query!(
                r#"
                SELECT 
                    ur.redemption_id::text,
                    ur.redemption_code,
                    ur.redemption_status,
                    ur.lumis_spent,
                    ur.created_at,
                    ur.code_expires_at,
                    ro.name_friendly as offer_name
                FROM rewards.user_redemptions ur
                INNER JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
                WHERE ur.redemption_id = $1
                "#,
                uuid
            )
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| {
                error!("Database error validating redemption by UUID: {}", e);
                ApiError::InternalError("Error al validar código".to_string())
            })?;
            
            result.map(|r| RedemptionQueryResult {
                redemption_id: r.redemption_id.unwrap_or_default(),
                redemption_code: r.redemption_code,
                redemption_status: r.redemption_status,
                lumis_spent: r.lumis_spent,
                created_at: r.created_at.unwrap_or_else(|| chrono::Utc::now()),
                code_expires_at: r.code_expires_at,
                offer_name: r.offer_name,
            })
        }
        Err(_) => {
            // Query by redemption_code (string)
            let result = sqlx::query!(
                r#"
                SELECT 
                    ur.redemption_id::text,
                    ur.redemption_code,
                    ur.redemption_status,
                    ur.lumis_spent,
                    ur.created_at,
                    ur.code_expires_at,
                    ro.name_friendly as offer_name
                FROM rewards.user_redemptions ur
                INNER JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
                WHERE ur.redemption_code = $1
                "#,
                payload.code
            )
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| {
                error!("Database error validating redemption by code: {}", e);
                ApiError::InternalError("Error al validar código".to_string())
            })?;
            
            result.map(|r| RedemptionQueryResult {
                redemption_id: r.redemption_id.unwrap_or_default(),
                redemption_code: r.redemption_code,
                redemption_status: r.redemption_status,
                lumis_spent: r.lumis_spent,
                created_at: r.created_at.unwrap_or_else(|| chrono::Utc::now()),
                code_expires_at: r.code_expires_at,
                offer_name: r.offer_name,
            })
        }
    };
    
    let redemption = match redemption_opt {
        Some(r) => r,
        None => {
            info!("Redemption not found: {}", payload.code);
            return Ok(Json(ValidationResponse {
                success: true,
                valid: false,
                redemption: None,
                message: "Código de redención no encontrado".to_string(),
            }));
        }
    };
    
    // Check if already used
    if redemption.redemption_status == "confirmed" {
        return Ok(Json(ValidationResponse {
            success: true,
            valid: false,
            redemption: None,
            message: "Este código ya fue utilizado".to_string(),
        }));
    }
    
    // Check if cancelled
    if redemption.redemption_status == "cancelled" {
        return Ok(Json(ValidationResponse {
            success: true,
            valid: false,
            redemption: None,
            message: "Este código fue cancelado".to_string(),
        }));
    }
    
    // Check if expired
    let now = chrono::Utc::now();
    if redemption.code_expires_at < now {
        return Ok(Json(ValidationResponse {
            success: true,
            valid: false,
            redemption: None,
            message: "Este código expiró".to_string(),
        }));
    }
    
    // Valid redemption
    let can_confirm = redemption.redemption_status == "pending";
    
    info!("Redemption validated successfully: {}", redemption.redemption_code);
    
    // Registrar métrica de validación exitosa
    record_merchant_validation(&merchant.sub, true);
    
    Ok(Json(ValidationResponse {
        success: true,
        valid: true,
        redemption: Some(RedemptionDetails {
            redemption_id: redemption.redemption_id.clone(),
            redemption_code: redemption.redemption_code.clone(),
            offer_name: redemption.offer_name.clone().unwrap_or_else(|| "N/A".to_string()),
            lumis_spent: redemption.lumis_spent,
            status: redemption.redemption_status.clone(),
            created_at: redemption.created_at.to_rfc3339(),
            expires_at: redemption.code_expires_at.to_rfc3339(),
            can_confirm,
        }),
        message: if can_confirm {
            "Código válido. Puedes confirmar la redención.".to_string()
        } else {
            format!("Código encontrado pero no se puede confirmar (estado: {})", redemption.redemption_status)
        },
    }))
}

/// Confirm a redemption (mark as used)
/// 
/// # Endpoint
/// POST /api/v1/merchant/confirm/:id
/// 
/// # Authentication
/// Requires merchant JWT token
/// 
/// # Path Parameters
/// - id: UUID of the redemption to confirm
/// 
/// # Returns
/// - 200 OK: Redemption confirmed successfully
/// - 400 Bad Request: Cannot confirm (already used, expired, etc.)
/// - 401 Unauthorized: Invalid merchant token
/// - 404 Not Found: Redemption not found
/// - 500 Internal Server Error: Database error
pub async fn confirm_redemption(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Path(redemption_id): Path<Uuid>,
) -> Result<Json<ConfirmationResponse>, ApiError> {
    info!("Merchant {} confirming redemption: {}", 
          merchant.merchant_name, redemption_id);
    
    // Start transaction
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        ApiError::InternalError("Error al iniciar transacción".to_string())
    })?;
    
    // Get redemption with lock
    let redemption = sqlx::query!(
        r#"
        SELECT 
            redemption_id,
            redemption_code,
            redemption_status,
            code_expires_at
        FROM rewards.user_redemptions
        WHERE redemption_id = $1
        FOR UPDATE
        "#,
        redemption_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        error!("Database error fetching redemption: {}", e);
        ApiError::InternalError("Error al consultar redención".to_string())
    })?
    .ok_or_else(|| {
        error!("Redemption not found: {}", redemption_id);
        ApiError::NotFound("Redención no encontrada".to_string())
    })?;
    
    // Validate status
    if redemption.redemption_status != "pending" {
        return Err(ApiError::BadRequest(format!(
            "No se puede confirmar redención con estado: {}",
            redemption.redemption_status
        )));
    }
    
    // Validate expiration
    let now = chrono::Utc::now();
    if redemption.code_expires_at < now {
        return Err(ApiError::BadRequest("Código expirado".to_string()));
    }
    
    // Update status to confirmed
    sqlx::query!(
        r#"
        UPDATE rewards.user_redemptions
        SET 
            redemption_status = 'confirmed',
            validated_at = NOW()
        WHERE redemption_id = $1
        "#,
        redemption_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error!("Failed to update redemption: {}", e);
        ApiError::InternalError("Error al confirmar redención".to_string())
    })?;
    
    // Commit transaction
    tx.commit().await.map_err(|e| {
        error!("Failed to commit transaction: {}", e);
        ApiError::InternalError("Error al guardar confirmación".to_string())
    })?;
    
    info!("Redemption confirmed successfully: {}", redemption.redemption_code);
    
    // Registrar métrica de confirmación
    record_redemption_confirmed(&merchant.sub, "standard");
    
    // Obtener datos adicionales para notificaciones
    let redemption_data = sqlx::query!(
        r#"
        SELECT 
            ur.user_id,
            ro.name_friendly as offer_name,
            ro.merchant_id
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ur.redemption_id = $1
        "#,
        redemption_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();
    
    // Extraer datos para uso en spawns
    let (user_id_opt, offer_name_opt, _merchant_id_opt) = redemption_data
        .map(|d| (Some(d.user_id), d.offer_name.clone(), Some(d.merchant_id)))
        .unwrap_or((None, None, None));
    
    // Enviar push notification al usuario (asíncrono)
    if let (Some(user_id), Some(offer_name)) = (user_id_opt, offer_name_opt.as_ref()) {
        if let Some(push_service) = get_push_service() {
            let offer_name = offer_name.clone();
            
            tokio::spawn(async move {
                if let Err(e) = push_service.notify_redemption_confirmed(
                    user_id,
                    redemption_id,
                    &offer_name,
                ).await {
                    error!("Failed to send confirmation push notification: {}", e);
                }
            });
        }
    }
    
    // TODO: Enviar webhook al merchant (temporalmente deshabilitado por bug de compilación)
    // Bug: Rust no infiere correctamente el tipo de merchant_id_opt dentro del closure async
    // Solución temporal: Comentado para permitir compilación
    // Solución definitiva: Investigar ownership en closures async con Uuid
    // Ver: ULTIMO_ERROR_COMPILACION.md para detalles
    
    /*
    match (merchant_id_opt, offer_name_opt) {
        (Some(mid), Some(oname)) if get_webhook_service().is_some() => {
            let webhook_service = get_webhook_service().unwrap();
            let merchant_id_copy: uuid::Uuid = mid;
            let offer_name_copy = oname.clone();
            let code = redemption.redemption_code.clone();
            let confirmed_by = merchant.merchant_name.clone();
            
            tokio::spawn(async move {
                if let Err(e) = webhook_service.notify_redemption_confirmed(
                    merchant_id_copy,
                    redemption_id,
                    &code,
                    &offer_name_copy,
                    &confirmed_by,
                ).await {
                    error!("Failed to send confirmation webhook: {}", e);
                }
            });
        }
        _ => {}
    }
    */
    
    Ok(Json(ConfirmationResponse {
        success: true,
        message: "Redención confirmada exitosamente".to_string(),
        redemption_id: redemption_id.to_string(),
        confirmed_at: now.to_rfc3339(),
    }))
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        
        let body = Json(serde_json::json!({
            "success": false,
            "error": message,
        }));
        
        (status, body).into_response()
    }
}

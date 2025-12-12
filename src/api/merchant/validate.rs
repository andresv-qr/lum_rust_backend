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
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    middleware::auth::MerchantClaims,
    state::AppState,
    observability::metrics::{record_merchant_validation, record_redemption_confirmed},
    services::get_push_service,
    domains::rewards::qr_generator::QrGenerator,
};

/// Request body for validating a redemption
#[derive(Debug, Deserialize)]
pub struct ValidateRedemptionRequest {
    /// Can be either redemption_code (e.g., "LUMS-A1B2-C3D4-E5F6") or redemption_id (UUID)
    pub code: String,
    /// Optional: JWT token from QR for extra security verification
    pub token: Option<String>,
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

/// Request body for confirmation (optional, enhances security)
#[derive(Debug, Deserialize, Default)]
pub struct ConfirmRedemptionRequest {
    /// Optional: JWT token from QR for jti verification
    pub token: Option<String>,
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
///   "code": "LUMS-A1B2-C3D4-E5F6",
///   "token": "optional_jwt_from_qr"
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
    
    // 🔒 Rate limiting para prevenir ataques de fuerza bruta
    check_merchant_validation_rate_limit(&state, &merchant.sub).await?;
    
    // Verificar token JWT si se proporciona (seguridad extra)
    if let Some(ref token) = payload.token {
        let qr_config = crate::domains::rewards::qr_generator::QrConfig::default();
        let qr_generator = QrGenerator::new(qr_config);
        
        match qr_generator.verify_validation_token(token) {
            Ok(claims) => {
                // Verificar que el código coincide
                if claims.redemption_code != payload.code {
                    warn!("Token mismatch: token code {} != request code {}", 
                          claims.redemption_code, payload.code);
                    return Ok(Json(ValidationResponse {
                        success: true,
                        valid: false,
                        redemption: None,
                        message: "Token no coincide con el código proporcionado".to_string(),
                    }));
                }
                
                // Verificar si el jti ya fue usado
                let jti_used: Option<bool> = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM rewards.used_validation_tokens WHERE jti = $1)"
                )
                .bind(&claims.jti)
                .fetch_optional(&state.db_pool)
                .await
                .ok()
                .flatten();
                
                if jti_used == Some(true) {
                    warn!("Token jti already used: {}", claims.jti);
                    return Ok(Json(ValidationResponse {
                        success: true,
                        valid: false,
                        redemption: None,
                        message: "Este código QR ya fue escaneado. Solicite uno nuevo.".to_string(),
                    }));
                }
                
                info!("Token validated successfully for code {} with jti {}", payload.code, claims.jti);
            }
            Err(e) => {
                warn!("Invalid token provided: {}", e);
                // No bloqueamos, pero avisamos
                // En producción podrías hacer esto más estricto
            }
        }
    }
    
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
            // Query by redemption_code (string) - SOLO búsqueda exacta por seguridad
            // Códigos parciales deshabilitados para prevenir ataques de fuerza bruta
            let search_code = payload.code.trim().to_uppercase();
            
            // Validar formato mínimo del código
            if search_code.len() < 10 {
                return Ok(Json(ValidationResponse {
                    success: true,
                    valid: false,
                    redemption: None,
                    message: "Código de redención inválido. Escanea el QR completo.".to_string(),
                }));
            }

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
                LIMIT 1
                "#,
                search_code
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
/// # Request Body (optional)
/// ```json
/// {
///   "token": "jwt_from_qr_for_jti_verification"
/// }
/// ```
/// 
/// # Returns
/// - 200 OK: Redemption confirmed successfully
/// - 400 Bad Request: Cannot confirm (already used, expired, etc.)
/// - 401 Unauthorized: Invalid merchant token
/// - 403 Forbidden: Merchant not authorized for this offer
/// - 404 Not Found: Redemption not found
/// - 500 Internal Server Error: Database error
pub async fn confirm_redemption(
    State(state): State<Arc<AppState>>,
    Extension(merchant): Extension<MerchantClaims>,
    Path(redemption_id): Path<Uuid>,
    body: Option<Json<ConfirmRedemptionRequest>>,
) -> Result<Json<ConfirmationResponse>, ApiError> {
    info!("Merchant {} (id: {:?}) confirming redemption: {}", 
          merchant.merchant_name, merchant.get_merchant_id(), redemption_id);
    
    let request = body.map(|b| b.0).unwrap_or_default();
    
    // Verificar jti si se proporciona token
    let mut token_jti: Option<String> = None;
    if let Some(ref token) = request.token {
        let qr_config = crate::domains::rewards::qr_generator::QrConfig::default();
        let qr_generator = QrGenerator::new(qr_config);
        
        match qr_generator.verify_validation_token(token) {
            Ok(claims) => {
                // Verificar si el jti ya fue usado
                let jti_used: Option<bool> = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM rewards.used_validation_tokens WHERE jti = $1)"
                )
                .bind(&claims.jti)
                .fetch_optional(&state.db_pool)
                .await
                .ok()
                .flatten();
                
                if jti_used == Some(true) {
                    warn!("Token jti already used in confirm: {}", claims.jti);
                    return Err(ApiError::BadRequest(
                        "Este código QR ya fue utilizado. Solicite uno nuevo.".to_string()
                    ));
                }
                
                token_jti = Some(claims.jti);
            }
            Err(e) => {
                warn!("Invalid token in confirm: {}", e);
                // Token inválido o expirado
                return Err(ApiError::BadRequest(
                    "Token de validación inválido o expirado".to_string()
                ));
            }
        }
    }
    
    // Start transaction
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        ApiError::InternalError("Error al iniciar transacción".to_string())
    })?;
    
    // Get redemption with lock - including merchant validation
    let redemption = sqlx::query!(
        r#"
        SELECT 
            ur.redemption_id,
            ur.redemption_code,
            ur.redemption_status,
            ur.code_expires_at,
            ro.merchant_id as offer_merchant_id
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ur.redemption_id = $1
        FOR UPDATE OF ur
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
    
    // Validar que el merchant está autorizado para esta oferta
    if let Some(offer_merchant_id) = redemption.offer_merchant_id {
        if let Some(claiming_merchant_id) = merchant.get_merchant_id() {
            if offer_merchant_id != claiming_merchant_id {
                warn!("Merchant {} attempted to confirm redemption for merchant {}", 
                      claiming_merchant_id, offer_merchant_id);
                return Err(ApiError::Forbidden(
                    "No estás autorizado para confirmar esta redención".to_string()
                ));
            }
        }
        // Si el merchant no tiene ID en el token, permitimos (backward compatibility)
    }
    
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
    
    // Si hay un jti, guardarlo como usado ANTES de confirmar
    if let Some(ref jti) = token_jti {
        sqlx::query(
            r#"
            INSERT INTO rewards.used_validation_tokens (jti, redemption_id, used_by_merchant_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (jti) DO NOTHING
            "#
        )
        .bind(jti)
        .bind(redemption_id)
        .bind(merchant.get_merchant_id())
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Failed to save used token: {}", e);
            ApiError::InternalError("Error al registrar token usado".to_string())
        })?;
    }
    
    // Update status to confirmed with merchant info
    sqlx::query(
        r#"
        UPDATE rewards.user_redemptions
        SET 
            redemption_status = 'confirmed',
            validated_at = NOW(),
            validated_by_merchant_id = $2
        WHERE redemption_id = $1
        "#
    )
    .bind(redemption_id)
    .bind(merchant.get_merchant_id())
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
    
    // Extraer datos para uso en spawns - fix: clone explícito antes de mover
    let user_id_opt = redemption_data.as_ref().map(|d| d.user_id);
    let offer_name_opt = redemption_data.as_ref().and_then(|d| d.offer_name.clone());
    let merchant_id_opt = redemption_data.as_ref().and_then(|d| d.merchant_id);
    
    // Enviar push notification al usuario (asíncrono)
    if let (Some(user_id), Some(ref offer_name)) = (user_id_opt, &offer_name_opt) {
        if let Some(push_service) = get_push_service() {
            let offer_name = offer_name.clone();
            let user_id = user_id;
            
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
    
    // Enviar webhook al merchant (asíncrono)
    if let Some(merchant_id) = merchant_id_opt {
        if let Some(webhook_service) = crate::services::get_webhook_service() {
            let offer_name = offer_name_opt.clone().unwrap_or_default();
            let code = redemption.redemption_code.clone();
            let confirmed_by = merchant.merchant_name.clone();
            
            tokio::spawn(async move {
                if let Err(e) = webhook_service.notify_redemption_confirmed(
                    merchant_id,
                    redemption_id,
                    &code,
                    &offer_name,
                    &confirmed_by,
                ).await {
                    error!("Failed to send confirmation webhook: {}", e);
                }
            });
        }
    }
    
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
    Forbidden(String),
    NotFound(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
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

/// Rate limiting para validación de merchants (previene fuerza bruta)
const VALIDATIONS_PER_MINUTE: i64 = 30;
const VALIDATIONS_PER_HOUR: i64 = 300;

async fn check_merchant_validation_rate_limit(state: &AppState, merchant_id: &str) -> Result<(), ApiError> {
    let mut conn = match state.redis_pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Redis unavailable for rate limit, allowing request: {}", e);
            return Ok(()); // Fail open si Redis no está disponible
        }
    };
    
    let now = chrono::Utc::now();
    let minute_key = format!("merchant_validate:min:{}:{}", merchant_id, now.format("%Y%m%d%H%M"));
    let hour_key = format!("merchant_validate:hour:{}:{}", merchant_id, now.format("%Y%m%d%H"));
    
    // Verificar límite por minuto (atómico)
    let minute_count: i64 = redis::cmd("INCR")
        .arg(&minute_key)
        .query_async(&mut *conn)
        .await
        .unwrap_or(1);
    
    if minute_count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&minute_key)
            .arg(60)
            .query_async(&mut *conn)
            .await
            .unwrap_or(());
    }
    
    if minute_count > VALIDATIONS_PER_MINUTE {
        return Err(ApiError::BadRequest(
            "Demasiados intentos de validación. Espera un momento.".to_string()
        ));
    }
    
    // Verificar límite por hora
    let hour_count: i64 = redis::cmd("INCR")
        .arg(&hour_key)
        .query_async(&mut *conn)
        .await
        .unwrap_or(1);
    
    if hour_count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&hour_key)
            .arg(3600)
            .query_async(&mut *conn)
            .await
            .unwrap_or(());
    }
    
    if hour_count > VALIDATIONS_PER_HOUR {
        return Err(ApiError::BadRequest(
            "Límite de validaciones por hora excedido.".to_string()
        ));
    }
    
    Ok(())
}

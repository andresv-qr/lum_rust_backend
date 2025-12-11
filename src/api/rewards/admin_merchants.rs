// ============================================================================
// ADMIN MERCHANTS - CRUD completo para gestión de comercios
// ============================================================================

use axum::{
    extract::{Path, Query, State},
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
    middleware::auth::JwtClaims,
    state::AppState,
};

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListMerchantsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub status: Option<String>,  // active, inactive, all
    pub search: Option<String>,  // Buscar por nombre
}

#[derive(Debug, Serialize)]
pub struct MerchantListResponse {
    pub success: bool,
    pub merchants: Vec<MerchantItem>,
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MerchantItem {
    pub merchant_id: Uuid,
    pub merchant_name: String,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub is_active: bool,
    pub webhook_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub offers_count: i64,
    pub redemptions_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateMerchantRequest {
    pub merchant_name: String,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub webhook_url: Option<String>,
    pub api_key: Option<String>,  // Si no se proporciona, se genera automáticamente
}

#[derive(Debug, Deserialize)]
pub struct UpdateMerchantRequest {
    pub merchant_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub webhook_url: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MerchantDetailResponse {
    pub success: bool,
    pub merchant: MerchantDetail,
}

#[derive(Debug, Serialize)]
pub struct MerchantDetail {
    pub merchant_id: Uuid,
    pub merchant_name: String,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub is_active: bool,
    pub webhook_url: Option<String>,
    pub api_key_preview: Option<String>,  // Solo últimos 4 caracteres
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub statistics: MerchantStats,
}

#[derive(Debug, Serialize, Default)]
pub struct MerchantStats {
    pub total_offers: i64,
    pub active_offers: i64,
    pub total_redemptions: i64,
    pub confirmed_redemptions: i64,
    pub total_lumis_redeemed: i64,
}

#[derive(Debug, Serialize)]
pub struct CreateMerchantResponse {
    pub success: bool,
    pub merchant_id: Uuid,
    pub api_key: String,  // Solo se muestra una vez al crear
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RegenerateApiKeyResponse {
    pub success: bool,
    pub api_key: String,
    pub message: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn is_admin(user_id: i32) -> bool {
    let admin_ids: Vec<i32> = std::env::var("ADMIN_USER_IDS")
        .unwrap_or_else(|_| "1,2,3".to_string())
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    admin_ids.contains(&user_id)
}

fn generate_api_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let key: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect();
    format!("lum_mk_{}", key)
}

fn hash_api_key(key: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ============================================================================
// Endpoints
// ============================================================================

/// Listar todos los merchants (admin)
/// GET /api/v1/admin/merchants
pub async fn list_merchants(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Query(params): Query<ListMerchantsQuery>,
) -> Result<Json<MerchantListResponse>, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let status_filter = params.status.as_deref().unwrap_or("all");

    // Query para merchants con estadísticas
    let mut query = String::from(
        r#"
        SELECT 
            m.merchant_id,
            m.merchant_name,
            m.contact_email,
            m.contact_phone,
            m.is_active,
            m.webhook_url,
            m.created_at,
            COALESCE(COUNT(DISTINCT ro.offer_id), 0) as offers_count,
            COALESCE(COUNT(DISTINCT ur.redemption_id), 0) as redemptions_count
        FROM rewards.merchants m
        LEFT JOIN rewards.redemption_offers ro ON m.merchant_id = ro.merchant_id
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
        WHERE 1=1
        "#
    );

    if status_filter == "active" {
        query.push_str(" AND m.is_active = true");
    } else if status_filter == "inactive" {
        query.push_str(" AND m.is_active = false");
    }

    if params.search.is_some() {
        query.push_str(" AND m.merchant_name ILIKE $3");
    }

    query.push_str(
        r#"
        GROUP BY m.merchant_id, m.merchant_name, m.contact_email, m.contact_phone, 
                 m.is_active, m.webhook_url, m.created_at
        ORDER BY m.created_at DESC
        LIMIT $1 OFFSET $2
        "#
    );

    let merchants: Vec<MerchantItem> = if let Some(ref search) = params.search {
        let search_pattern = format!("%{}%", search);
        sqlx::query_as(&query)
            .bind(limit as i64)
            .bind(offset as i64)
            .bind(&search_pattern)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                error!("Failed to list merchants: {}", e);
                ApiError::InternalError("Error al listar comercios".to_string())
            })?
    } else {
        sqlx::query_as(&query)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                error!("Failed to list merchants: {}", e);
                ApiError::InternalError("Error al listar comercios".to_string())
            })?
    };

    // Contar total
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rewards.merchants"
    )
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    Ok(Json(MerchantListResponse {
        success: true,
        merchants,
        total,
        limit,
        offset,
    }))
}

/// Obtener detalle de un merchant
/// GET /api/v1/admin/merchants/:id
pub async fn get_merchant(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Path(merchant_id): Path<Uuid>,
) -> Result<Json<MerchantDetailResponse>, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    // Obtener merchant básico
    let merchant = sqlx::query!(
        r#"
        SELECT 
            merchant_id,
            merchant_name,
            contact_email,
            contact_phone,
            is_active,
            webhook_url,
            api_key_hash,
            created_at,
            updated_at
        FROM rewards.merchants
        WHERE merchant_id = $1
        "#,
        merchant_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to get merchant: {}", e);
        ApiError::InternalError("Error al obtener comercio".to_string())
    })?
    .ok_or_else(|| ApiError::NotFound("Comercio no encontrado".to_string()))?;

    // Obtener estadísticas
    let stats = sqlx::query!(
        r#"
        SELECT 
            COUNT(DISTINCT ro.offer_id) as total_offers,
            COUNT(DISTINCT ro.offer_id) FILTER (WHERE ro.is_active = true) as active_offers,
            COUNT(DISTINCT ur.redemption_id) as total_redemptions,
            COUNT(DISTINCT ur.redemption_id) FILTER (WHERE ur.redemption_status = 'confirmed') as confirmed_redemptions,
            COALESCE(SUM(ur.lumis_spent) FILTER (WHERE ur.redemption_status = 'confirmed'), 0) as total_lumis_redeemed
        FROM rewards.merchants m
        LEFT JOIN rewards.redemption_offers ro ON m.merchant_id = ro.merchant_id
        LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id
        WHERE m.merchant_id = $1
        GROUP BY m.merchant_id
        "#,
        merchant_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .ok()
    .flatten();

    let statistics = stats.map(|s| MerchantStats {
        total_offers: s.total_offers.unwrap_or(0),
        active_offers: s.active_offers.unwrap_or(0),
        total_redemptions: s.total_redemptions.unwrap_or(0),
        confirmed_redemptions: s.confirmed_redemptions.unwrap_or(0),
        total_lumis_redeemed: s.total_lumis_redeemed.unwrap_or(0),
    }).unwrap_or_default();

    // Preview del API key (últimos 4 caracteres del hash)
    let api_key_preview: Option<String> = Some({
        let h = &merchant.api_key_hash;
        format!("...{}", &h[h.len().saturating_sub(4)..])
    });

    Ok(Json(MerchantDetailResponse {
        success: true,
        merchant: MerchantDetail {
            merchant_id: merchant.merchant_id,
            merchant_name: merchant.merchant_name,
            contact_email: merchant.contact_email,
            contact_phone: merchant.contact_phone,
            is_active: merchant.is_active.unwrap_or(true),
            webhook_url: merchant.webhook_url,
            api_key_preview,
            created_at: merchant.created_at.unwrap_or_else(|| chrono::Utc::now()),
            updated_at: merchant.updated_at,
            statistics,
        },
    }))
}

/// Crear un nuevo merchant
/// POST /api/v1/admin/merchants
pub async fn create_merchant(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Json(payload): Json<CreateMerchantRequest>,
) -> Result<Json<CreateMerchantResponse>, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    // Validar nombre
    if payload.merchant_name.trim().is_empty() {
        return Err(ApiError::BadRequest("El nombre del comercio es requerido".to_string()));
    }

    // Generar API key
    let api_key = payload.api_key.unwrap_or_else(generate_api_key);
    let api_key_hash = hash_api_key(&api_key);

    let merchant_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO rewards.merchants (
            merchant_id, merchant_name, contact_email, contact_phone,
            webhook_url, api_key_hash, is_active, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, true, NOW())
        "#,
        merchant_id,
        payload.merchant_name.trim(),
        payload.contact_email,
        payload.contact_phone,
        payload.webhook_url,
        api_key_hash,
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to create merchant: {}", e);
        if e.to_string().contains("duplicate") {
            ApiError::BadRequest("Ya existe un comercio con ese nombre".to_string())
        } else {
            ApiError::InternalError("Error al crear comercio".to_string())
        }
    })?;

    info!("Admin {} created merchant {}: {}", user_id, merchant_id, payload.merchant_name);

    Ok(Json(CreateMerchantResponse {
        success: true,
        merchant_id,
        api_key,
        message: "Comercio creado exitosamente. Guarda el API key, no se mostrará de nuevo.".to_string(),
    }))
}

/// Actualizar un merchant
/// PUT /api/v1/admin/merchants/:id
pub async fn update_merchant(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Path(merchant_id): Path<Uuid>,
    Json(payload): Json<UpdateMerchantRequest>,
) -> Result<Json<SuccessResponse>, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    // Verificar que existe
    let exists: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM rewards.merchants WHERE merchant_id = $1)"
    )
    .bind(merchant_id)
    .fetch_one(&state.db_pool)
    .await
    .ok();

    if exists != Some(true) {
        return Err(ApiError::NotFound("Comercio no encontrado".to_string()));
    }

    // Usar query simple para actualizar todos los campos opcionalmente
    sqlx::query(
        r#"
        UPDATE rewards.merchants
        SET 
            merchant_name = COALESCE($2, merchant_name),
            contact_email = COALESCE($3, contact_email),
            contact_phone = COALESCE($4, contact_phone),
            webhook_url = COALESCE($5, webhook_url),
            is_active = COALESCE($6, is_active),
            updated_at = NOW()
        WHERE merchant_id = $1
        "#
    )
    .bind(merchant_id)
    .bind(&payload.merchant_name)
    .bind(&payload.contact_email)
    .bind(&payload.contact_phone)
    .bind(&payload.webhook_url)
    .bind(payload.is_active)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to update merchant: {}", e);
        ApiError::InternalError("Error al actualizar comercio".to_string())
    })?;

    info!("Admin {} updated merchant {}", user_id, merchant_id);

    Ok(Json(SuccessResponse {
        success: true,
        message: "Comercio actualizado exitosamente".to_string(),
    }))
}

/// Eliminar (desactivar) un merchant
/// DELETE /api/v1/admin/merchants/:id
pub async fn delete_merchant(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Path(merchant_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    // Soft delete - solo desactivar
    let result = sqlx::query!(
        r#"
        UPDATE rewards.merchants
        SET is_active = false, updated_at = NOW()
        WHERE merchant_id = $1
        "#,
        merchant_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to delete merchant: {}", e);
        ApiError::InternalError("Error al eliminar comercio".to_string())
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Comercio no encontrado".to_string()));
    }

    // También desactivar ofertas del merchant
    sqlx::query!(
        r#"
        UPDATE rewards.redemption_offers
        SET is_active = false
        WHERE merchant_id = $1
        "#,
        merchant_id
    )
    .execute(&state.db_pool)
    .await
    .ok();

    warn!("Admin {} deleted (deactivated) merchant {}", user_id, merchant_id);

    Ok(Json(SuccessResponse {
        success: true,
        message: "Comercio desactivado exitosamente".to_string(),
    }))
}

/// Regenerar API key de un merchant
/// POST /api/v1/admin/merchants/:id/regenerate-key
pub async fn regenerate_api_key(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Path(merchant_id): Path<Uuid>,
) -> Result<Json<RegenerateApiKeyResponse>, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    // Generar nuevo API key
    let new_api_key = generate_api_key();
    let api_key_hash = hash_api_key(&new_api_key);

    let result = sqlx::query!(
        r#"
        UPDATE rewards.merchants
        SET api_key_hash = $2, updated_at = NOW()
        WHERE merchant_id = $1
        "#,
        merchant_id,
        api_key_hash
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to regenerate API key: {}", e);
        ApiError::InternalError("Error al regenerar API key".to_string())
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Comercio no encontrado".to_string()));
    }

    warn!("Admin {} regenerated API key for merchant {}", user_id, merchant_id);

    Ok(Json(RegenerateApiKeyResponse {
        success: true,
        api_key: new_api_key,
        message: "API key regenerado. Guárdalo, no se mostrará de nuevo.".to_string(),
    }))
}

/// Activar un merchant
/// POST /api/v1/admin/merchants/:id/activate
pub async fn activate_merchant(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<JwtClaims>,
    Path(merchant_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, ApiError> {
    let user_id = claims.user_id().map_err(|e| ApiError::Unauthorized(e))?;
    
    if !is_admin(user_id) {
        return Err(ApiError::Forbidden("Acceso no autorizado".to_string()));
    }

    let result = sqlx::query!(
        r#"
        UPDATE rewards.merchants
        SET is_active = true, updated_at = NOW()
        WHERE merchant_id = $1
        "#,
        merchant_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to activate merchant: {}", e);
        ApiError::InternalError("Error al activar comercio".to_string())
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Comercio no encontrado".to_string()));
    }

    info!("Admin {} activated merchant {}", user_id, merchant_id);

    Ok(Json(SuccessResponse {
        success: true,
        message: "Comercio activado exitosamente".to_string(),
    }))
}

// ============================================================================
// Error Types
// ============================================================================

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

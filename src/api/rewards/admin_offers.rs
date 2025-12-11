//! Admin Offers API - CRUD para gestión de ofertas de redención
//!
//! Endpoints:
//! - GET    /api/v1/rewards/admin/offers           - Listar ofertas
//! - GET    /api/v1/rewards/admin/offers/:offer_id - Obtener detalle
//! - POST   /api/v1/rewards/admin/offers           - Crear oferta
//! - PUT    /api/v1/rewards/admin/offers/:offer_id - Actualizar oferta
//! - DELETE /api/v1/rewards/admin/offers/:offer_id - Eliminar oferta
//! - POST   /api/v1/rewards/admin/offers/:offer_id/activate   - Activar
//! - POST   /api/v1/rewards/admin/offers/:offer_id/deactivate - Desactivar

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::api::common::{ApiError, ApiResponse};
use crate::middleware::auth::CurrentUser;
use crate::state::AppState;
use axum::Extension;

// Helper para crear respuestas
fn ok_response<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse::success(data, Uuid::new_v4().to_string(), None, false))
}

// ============================================================================
// MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateOfferRequest {
    pub name: String,
    pub name_friendly: String,
    pub description_friendly: Option<String>,
    pub lumis_cost: i32,
    pub offer_category: Option<String>,
    pub merchant_id: Option<Uuid>,
    pub merchant_name: Option<String>,
    pub stock_quantity: Option<i32>,
    #[serde(default = "default_max_redemptions")]
    pub max_redemptions_per_user: i32,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub img: Option<String>,
    pub terms_and_conditions: Option<String>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
}

fn default_max_redemptions() -> i32 { 5 }
fn default_is_active() -> bool { true }

#[derive(Debug, Deserialize)]
pub struct UpdateOfferRequest {
    pub name: Option<String>,
    pub name_friendly: Option<String>,
    pub description_friendly: Option<String>,
    pub lumis_cost: Option<i32>,
    pub offer_category: Option<String>,
    pub merchant_id: Option<Uuid>,
    pub merchant_name: Option<String>,
    pub stock_quantity: Option<i32>,
    pub max_redemptions_per_user: Option<i32>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub img: Option<String>,
    pub terms_and_conditions: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AdminOfferFilters {
    pub category: Option<String>,
    pub merchant_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 20 }

#[derive(Debug, Serialize)]
pub struct AdminOfferResponse {
    pub offer_id: Uuid,
    pub name: String,
    pub name_friendly: String,
    pub description_friendly: Option<String>,
    pub lumis_cost: i32,
    pub offer_category: Option<String>,
    pub merchant_id: Option<Uuid>,
    pub merchant_name: Option<String>,
    pub stock_quantity: Option<i32>,
    pub max_redemptions_per_user: i32,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub img: Option<String>,
    pub terms_and_conditions: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_redemptions: i64,
    pub pending_redemptions: i64,
    pub used_redemptions: i64,
    pub total_lumis_redeemed: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminOffersListResponse {
    pub offers: Vec<AdminOfferResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct OfferWithStatsRow {
    offer_id: Uuid,
    name: String,
    name_friendly: String,
    description_friendly: Option<String>,
    lumis_cost: i32,
    offer_category: Option<String>,
    merchant_id: Option<Uuid>,
    merchant_name: Option<String>,
    stock_quantity: Option<i32>,
    max_redemptions_per_user: i32,
    valid_from: Option<DateTime<Utc>>,
    valid_to: Option<DateTime<Utc>>,
    is_active: bool,
    img: Option<String>,
    terms_and_conditions: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    total_redemptions: i64,
    pending_redemptions: i64,
    used_redemptions: i64,
    total_lumis_redeemed: i64,
}

// ============================================================================
// ADMIN VALIDATION
// ============================================================================

fn get_admin_user_ids() -> Vec<i64> {
    std::env::var("ADMIN_USER_IDS")
        .map(|s| s.split(',').filter_map(|id| id.trim().parse().ok()).collect())
        .unwrap_or_else(|_| vec![1, 2, 3])
}

fn verify_admin(user_id: i64) -> Result<(), ApiError> {
    if get_admin_user_ids().contains(&user_id) {
        Ok(())
    } else {
        warn!("Non-admin user {} attempted admin action", user_id);
        Err(ApiError::new("FORBIDDEN", "No tienes permisos de administrador"))
    }
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /api/v1/rewards/admin/offers - List all offers
pub async fn list_offers(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<CurrentUser>,
    Query(filters): Query<AdminOfferFilters>,
) -> Result<Json<ApiResponse<AdminOffersListResponse>>, ApiError> {
    verify_admin(user.user_id)?;
    
    let pool = &state.db_pool;
    
    // Build query with stats
    let rows = sqlx::query_as::<_, OfferWithStatsRow>(r#"
        SELECT 
            o.offer_id,
            COALESCE(o.name, o.name_friendly) as name,
            o.name_friendly,
            o.description_friendly,
            COALESCE(o.lumis_cost, o.points, 0) as lumis_cost,
            o.offer_category,
            o.merchant_id,
            o.merchant_name,
            o.stock_quantity,
            COALESCE(o.max_redemptions_per_user, 5) as max_redemptions_per_user,
            o.valid_from,
            o.valid_to,
            COALESCE(o.is_active, true) as is_active,
            o.img,
            o.terms_and_conditions,
            COALESCE(o.created_at, NOW()) as created_at,
            COALESCE(o.updated_at, NOW()) as updated_at,
            COALESCE(stats.total_redemptions, 0) as total_redemptions,
            COALESCE(stats.pending_redemptions, 0) as pending_redemptions,
            COALESCE(stats.used_redemptions, 0) as used_redemptions,
            COALESCE(stats.total_lumis, 0) as total_lumis_redeemed
        FROM rewards.redemption_offers o
        LEFT JOIN LATERAL (
            SELECT 
                COUNT(*) as total_redemptions,
                COUNT(*) FILTER (WHERE status = 'pending') as pending_redemptions,
                COUNT(*) FILTER (WHERE status = 'used') as used_redemptions,
                COALESCE(SUM(lumis_cost), 0) as total_lumis
            FROM rewards.user_redemptions
            WHERE offer_id = o.offer_id
        ) stats ON true
        ORDER BY o.created_at DESC
        LIMIT $1 OFFSET $2
    "#)
    .bind(filters.limit)
    .bind(filters.offset)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!("Error fetching offers: {}", e);
        ApiError::database_error(&format!("Error obteniendo ofertas: {}", e))
    })?;
    
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rewards.redemption_offers")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::database_error(&format!("Error contando: {}", e)))?;
    
    let offers: Vec<AdminOfferResponse> = rows.into_iter().map(|r| AdminOfferResponse {
        offer_id: r.offer_id,
        name: r.name,
        name_friendly: r.name_friendly,
        description_friendly: r.description_friendly,
        lumis_cost: r.lumis_cost,
        offer_category: r.offer_category,
        merchant_id: r.merchant_id,
        merchant_name: r.merchant_name,
        stock_quantity: r.stock_quantity,
        max_redemptions_per_user: r.max_redemptions_per_user,
        valid_from: r.valid_from,
        valid_to: r.valid_to,
        is_active: r.is_active,
        img: r.img,
        terms_and_conditions: r.terms_and_conditions,
        created_at: r.created_at,
        updated_at: r.updated_at,
        total_redemptions: r.total_redemptions,
        pending_redemptions: r.pending_redemptions,
        used_redemptions: r.used_redemptions,
        total_lumis_redeemed: r.total_lumis_redeemed,
    }).collect();
    
    let has_more = (filters.offset + filters.limit) < total.0;
    
    info!("Admin {} listed {} offers", user.user_id, offers.len());
    
    Ok(ok_response(AdminOffersListResponse {
        offers,
        total: total.0,
        limit: filters.limit,
        offset: filters.offset,
        has_more,
    }))
}

/// GET /api/v1/rewards/admin/offers/:offer_id
pub async fn get_offer(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<CurrentUser>,
    Path(offer_id): Path<Uuid>,
) -> Result<Json<ApiResponse<AdminOfferResponse>>, ApiError> {
    verify_admin(user.user_id)?;
    
    let pool = &state.db_pool;
    
    let row = sqlx::query_as::<_, OfferWithStatsRow>(r#"
        SELECT 
            o.offer_id,
            COALESCE(o.name, o.name_friendly) as name,
            o.name_friendly,
            o.description_friendly,
            COALESCE(o.lumis_cost, o.points, 0) as lumis_cost,
            o.offer_category,
            o.merchant_id,
            o.merchant_name,
            o.stock_quantity,
            COALESCE(o.max_redemptions_per_user, 5) as max_redemptions_per_user,
            o.valid_from,
            o.valid_to,
            COALESCE(o.is_active, true) as is_active,
            o.img,
            o.terms_and_conditions,
            COALESCE(o.created_at, NOW()) as created_at,
            COALESCE(o.updated_at, NOW()) as updated_at,
            COALESCE(stats.total_redemptions, 0) as total_redemptions,
            COALESCE(stats.pending_redemptions, 0) as pending_redemptions,
            COALESCE(stats.used_redemptions, 0) as used_redemptions,
            COALESCE(stats.total_lumis, 0) as total_lumis_redeemed
        FROM rewards.redemption_offers o
        LEFT JOIN LATERAL (
            SELECT 
                COUNT(*) as total_redemptions,
                COUNT(*) FILTER (WHERE status = 'pending') as pending_redemptions,
                COUNT(*) FILTER (WHERE status = 'used') as used_redemptions,
                COALESCE(SUM(lumis_cost), 0) as total_lumis
            FROM rewards.user_redemptions
            WHERE offer_id = o.offer_id
        ) stats ON true
        WHERE o.offer_id = $1
    "#)
    .bind(offer_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("Oferta"))?;
    
    Ok(ok_response(AdminOfferResponse {
        offer_id: row.offer_id,
        name: row.name,
        name_friendly: row.name_friendly,
        description_friendly: row.description_friendly,
        lumis_cost: row.lumis_cost,
        offer_category: row.offer_category,
        merchant_id: row.merchant_id,
        merchant_name: row.merchant_name,
        stock_quantity: row.stock_quantity,
        max_redemptions_per_user: row.max_redemptions_per_user,
        valid_from: row.valid_from,
        valid_to: row.valid_to,
        is_active: row.is_active,
        img: row.img,
        terms_and_conditions: row.terms_and_conditions,
        created_at: row.created_at,
        updated_at: row.updated_at,
        total_redemptions: row.total_redemptions,
        pending_redemptions: row.pending_redemptions,
        used_redemptions: row.used_redemptions,
        total_lumis_redeemed: row.total_lumis_redeemed,
    }))
}

/// POST /api/v1/rewards/admin/offers
pub async fn create_offer(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<CurrentUser>,
    Json(req): Json<CreateOfferRequest>,
) -> Result<Json<ApiResponse<AdminOfferResponse>>, ApiError> {
    verify_admin(user.user_id)?;
    
    if req.name_friendly.trim().is_empty() {
        return Err(ApiError::bad_request("El nombre es requerido"));
    }
    if req.lumis_cost < 0 {
        return Err(ApiError::bad_request("El costo debe ser positivo"));
    }
    
    let pool = &state.db_pool;
    let offer_id = Uuid::new_v4();
    let now = Utc::now();
    
    sqlx::query(r#"
        INSERT INTO rewards.redemption_offers (
            offer_id, name, name_friendly, description_friendly,
            lumis_cost, points, offer_category, merchant_id, merchant_name,
            stock_quantity, max_redemptions_per_user,
            valid_from, valid_to, img, terms_and_conditions,
            is_active, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $16)
    "#)
    .bind(offer_id)
    .bind(&req.name)
    .bind(&req.name_friendly)
    .bind(&req.description_friendly)
    .bind(req.lumis_cost)
    .bind(&req.offer_category)
    .bind(req.merchant_id)
    .bind(&req.merchant_name)
    .bind(req.stock_quantity)
    .bind(req.max_redemptions_per_user)
    .bind(req.valid_from.unwrap_or(now))
    .bind(req.valid_to)
    .bind(&req.img)
    .bind(&req.terms_and_conditions)
    .bind(req.is_active)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error creando: {}", e)))?;
    
    info!("Admin {} created offer {} ({})", user.user_id, offer_id, req.name_friendly);
    
    get_offer(State(state), Extension(user), Path(offer_id)).await
}

/// PUT /api/v1/rewards/admin/offers/:offer_id
pub async fn update_offer(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<CurrentUser>,
    Path(offer_id): Path<Uuid>,
    Json(req): Json<UpdateOfferRequest>,
) -> Result<Json<ApiResponse<AdminOfferResponse>>, ApiError> {
    verify_admin(user.user_id)?;
    
    let pool = &state.db_pool;
    
    // Check exists
    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT id FROM rewards.redemption_offers WHERE offer_id = $1"
    )
    .bind(offer_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error: {}", e)))?;
    
    if exists.is_none() {
        return Err(ApiError::not_found("Oferta"));
    }
    
    // Update with provided fields
    sqlx::query(r#"
        UPDATE rewards.redemption_offers SET
            name = COALESCE($2, name),
            name_friendly = COALESCE($3, name_friendly),
            description_friendly = COALESCE($4, description_friendly),
            lumis_cost = COALESCE($5, lumis_cost),
            points = COALESCE($5, points),
            offer_category = COALESCE($6, offer_category),
            merchant_id = COALESCE($7, merchant_id),
            merchant_name = COALESCE($8, merchant_name),
            stock_quantity = COALESCE($9, stock_quantity),
            max_redemptions_per_user = COALESCE($10, max_redemptions_per_user),
            valid_from = COALESCE($11, valid_from),
            valid_to = COALESCE($12, valid_to),
            img = COALESCE($13, img),
            terms_and_conditions = COALESCE($14, terms_and_conditions),
            is_active = COALESCE($15, is_active),
            updated_at = NOW()
        WHERE offer_id = $1
    "#)
    .bind(offer_id)
    .bind(&req.name)
    .bind(&req.name_friendly)
    .bind(&req.description_friendly)
    .bind(req.lumis_cost)
    .bind(&req.offer_category)
    .bind(req.merchant_id)
    .bind(&req.merchant_name)
    .bind(req.stock_quantity)
    .bind(req.max_redemptions_per_user)
    .bind(req.valid_from)
    .bind(req.valid_to)
    .bind(&req.img)
    .bind(&req.terms_and_conditions)
    .bind(req.is_active)
    .execute(pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error actualizando: {}", e)))?;
    
    info!("Admin {} updated offer {}", user.user_id, offer_id);
    
    get_offer(State(state), Extension(user), Path(offer_id)).await
}

/// DELETE /api/v1/rewards/admin/offers/:offer_id (soft delete)
pub async fn delete_offer(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<CurrentUser>,
    Path(offer_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    verify_admin(user.user_id)?;
    
    let pool = &state.db_pool;
    
    // Check for pending redemptions
    let pending: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rewards.user_redemptions WHERE offer_id = $1 AND status = 'pending'"
    )
    .bind(offer_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error: {}", e)))?;
    
    if pending.0 > 0 {
        return Err(ApiError::bad_request(&format!(
            "No se puede eliminar: {} redenciones pendientes", pending.0
        )));
    }
    
    let result = sqlx::query(
        "UPDATE rewards.redemption_offers SET is_active = false, updated_at = NOW() WHERE offer_id = $1"
    )
    .bind(offer_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error: {}", e)))?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Oferta"));
    }
    
    info!("Admin {} soft-deleted offer {}", user.user_id, offer_id);
    
    Ok(ok_response(serde_json::json!({
        "message": "Oferta eliminada",
        "offer_id": offer_id
    })))
}

/// POST /api/v1/rewards/admin/offers/:offer_id/activate
pub async fn activate_offer(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<CurrentUser>,
    Path(offer_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    verify_admin(user.user_id)?;
    
    let result = sqlx::query(
        "UPDATE rewards.redemption_offers SET is_active = true, updated_at = NOW() WHERE offer_id = $1"
    )
    .bind(offer_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error: {}", e)))?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Oferta"));
    }
    
    info!("Admin {} activated offer {}", user.user_id, offer_id);
    
    Ok(ok_response(serde_json::json!({
        "message": "Oferta activada",
        "offer_id": offer_id,
        "is_active": true
    })))
}

/// POST /api/v1/rewards/admin/offers/:offer_id/deactivate
pub async fn deactivate_offer(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<CurrentUser>,
    Path(offer_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    verify_admin(user.user_id)?;
    
    let result = sqlx::query(
        "UPDATE rewards.redemption_offers SET is_active = false, updated_at = NOW() WHERE offer_id = $1"
    )
    .bind(offer_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Error: {}", e)))?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Oferta"));
    }
    
    info!("Admin {} deactivated offer {}", user.user_id, offer_id);
    
    Ok(ok_response(serde_json::json!({
        "message": "Oferta desactivada",
        "offer_id": offer_id,
        "is_active": false
    })))
}

// ============================================================================
// ROUTER
// ============================================================================

pub fn admin_offers_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_offers).post(create_offer))
        .route("/{offer_id}", get(get_offer).put(update_offer).delete(delete_offer))
        .route("/{offer_id}/activate", post(activate_offer))
        .route("/{offer_id}/deactivate", post(deactivate_offer))
}

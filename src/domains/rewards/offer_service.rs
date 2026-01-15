use super::models::{OfferFilters, OfferListItem, RedemptionError, RedemptionOffer};
use sqlx::PgPool;
use uuid::Uuid;

/// Servicio para gestionar el catálogo de ofertas
pub struct OfferService {
    db: PgPool,
}

impl OfferService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Listar ofertas con filtros
    pub async fn list_offers(
        &self,
        user_id: i32,
        filters: OfferFilters,
    ) -> Result<Vec<OfferListItem>, RedemptionError> {
        let limit = filters.limit.unwrap_or(20);
        let offset = filters.offset.unwrap_or(0);
        
        let sort_clause = match filters.sort.as_deref() {
            Some("price_asc") | Some("cost_asc") => "ORDER BY COALESCE(ro.lumis_cost, ro.points) ASC",
            Some("price_desc") | Some("cost_desc") => "ORDER BY COALESCE(ro.lumis_cost, ro.points) DESC",
            Some("newest") => "ORDER BY ro.created_at DESC",
            _ => "ORDER BY COALESCE(ro.lumis_cost, ro.points) ASC",
        };

        let query = format!(
            r#"
            SELECT 
                ro.offer_id,
                COALESCE(ro.name_friendly, ro.name) as name_friendly,
                COALESCE(ro.description_friendly, '') as description_friendly,
                COALESCE(ro.lumis_cost, ro.points) as lumis_cost,
                COALESCE(ro.offer_category, 'general') as category,
                COALESCE(ro.merchant_name, 'Comercio Aliado') as merchant_name,
                ro.img as image_url,
                ro.stock_quantity,
                COALESCE(ro.max_redemptions_per_user, 5) as max_redemptions_per_user,
                ro.valid_to as expires_at,
                COUNT(ur.redemption_id) as user_redemptions_count
            FROM rewards.redemption_offers ro
            LEFT JOIN rewards.user_redemptions ur ON ro.offer_id = ur.offer_id 
                AND ur.user_id = $1 
                AND ur.redemption_status != 'cancelled'
            WHERE ro.is_active = true
                AND (ro.valid_to IS NULL OR ro.valid_to > NOW())
                {}
                {}
                {}
                {}
            GROUP BY ro.offer_id, ro.name_friendly, ro.name, ro.description_friendly, 
                     ro.lumis_cost, ro.points, ro.offer_category, ro.merchant_name, 
                     ro.img, ro.stock_quantity, ro.max_redemptions_per_user, ro.valid_to
            {}
            LIMIT $2 OFFSET $3
            "#,
            if filters.category.is_some() { "AND ro.offer_category = $4" } else { "" },
            if filters.min_cost.is_some() { "AND COALESCE(ro.lumis_cost, ro.points) >= $5" } else { "" },
            if filters.max_cost.is_some() { "AND COALESCE(ro.lumis_cost, ro.points) <= $6" } else { "" },
            if filters.merchant_id.is_some() { "AND ro.merchant_id = $7" } else { "" },
            sort_clause
        );

        let mut query_builder = sqlx::query_as::<_, OfferRow>(&query)
            .bind(user_id)
            .bind(limit)
            .bind(offset);

        if let Some(cat) = filters.category {
            query_builder = query_builder.bind(cat);
        }
        if let Some(min) = filters.min_cost {
            query_builder = query_builder.bind(min);
        }
        if let Some(max) = filters.max_cost {
            query_builder = query_builder.bind(max);
        }
        if let Some(merchant) = filters.merchant_id {
            query_builder = query_builder.bind(merchant);
        }

        let rows = query_builder.fetch_all(&self.db).await?;

        let user_balance = self.get_user_balance(user_id).await?;

        let offers = rows
            .into_iter()
            .map(|row| {
                let has_stock = row.stock_quantity.map_or(true, |s| s > 0);
                let can_redeem = row.user_redemptions_count < row.max_redemptions_per_user as i64
                    && has_stock
                    && user_balance >= row.lumis_cost as i64;

                OfferListItem {
                    offer_id: row.offer_id,
                    name_friendly: row.name_friendly,
                    description_friendly: row.description_friendly,
                    lumis_cost: row.lumis_cost,
                    category: row.category,
                    merchant_name: row.merchant_name,
                    image_url: row.image_url,
                    is_available: can_redeem,
                    stock_remaining: row.stock_quantity,
                    max_redemptions_per_user: row.max_redemptions_per_user,
                    user_redemptions_count: row.user_redemptions_count,
                    expires_at: row.expires_at,
                }
            })
            .collect();

        Ok(offers)
    }

    /// Obtener detalles de una oferta
    pub async fn get_offer_details(
        &self,
        offer_id: Uuid,
        _user_id: i32, // Prefixed with _ as unused (for future filtering)
    ) -> Result<RedemptionOffer, RedemptionError> {
        let offer = sqlx::query_as::<_, RedemptionOffer>(
            r#"
            SELECT 
                id, offer_id, name, name_friendly, description_friendly,
                points, lumis_cost, offer_category, merchant_id, merchant_name,
                valid_from, valid_to, is_active, stock_quantity, 
                max_redemptions_per_user, img, NULL::text as terms_and_conditions,
                created_at
            FROM rewards.redemption_offers
            WHERE offer_id = $1 AND is_active = true
            "#,
        )
        .bind(offer_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or(RedemptionError::OfferNotFound)?;

        Ok(offer)
    }

    /// Obtener balance de Lümis del usuario
    pub async fn get_user_balance(&self, user_id: i32) -> Result<i64, RedemptionError> {
        let balance: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(balance::bigint, 0)
            FROM rewards.fact_balance_points
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(balance.unwrap_or(0))
    }

    /// Listar ofertas del usuario con estado enriquecido (my-offers)
    pub async fn get_user_offers(
        &self,
        user_id: i32,
        filters: super::models::MyOffersFilters,
    ) -> Result<Vec<super::models::MyOfferItem>, RedemptionError> {
        let limit = filters.limit.unwrap_or(50);
        let offset = filters.offset.unwrap_or(0);
        let status_filter = filters.status.as_deref().unwrap_or("all");

        // Query que obtiene ofertas con stats de redenciones del usuario
        let rows = sqlx::query_as::<_, MyOfferRow>(
            r#"
            WITH user_offer_stats AS (
                SELECT 
                    ur.offer_id,
                    COUNT(*) as total_redemptions,
                    COUNT(*) FILTER (WHERE ur.redemption_status = 'pending' AND ur.code_expires_at > NOW()) as pending,
                    COUNT(*) FILTER (WHERE ur.redemption_status = 'confirmed') as confirmed,
                    COUNT(*) FILTER (WHERE ur.redemption_status = 'cancelled') as cancelled,
                    COUNT(*) FILTER (WHERE ur.redemption_status = 'expired' OR (ur.redemption_status = 'pending' AND ur.code_expires_at <= NOW())) as expired,
                    MAX(ur.created_at) as last_redeemed_at
                FROM rewards.user_redemptions ur
                WHERE ur.user_id = $1
                GROUP BY ur.offer_id
            ),
            global_offer_stats AS (
                SELECT 
                    ur.offer_id,
                    COUNT(*) FILTER (WHERE ur.redemption_status != 'cancelled') as total_global_redemptions
                FROM rewards.user_redemptions ur
                GROUP BY ur.offer_id
            )
            SELECT 
                ro.offer_id,
                COALESCE(ro.name_friendly, ro.name) as name_friendly,
                COALESCE(ro.description_friendly, '') as description_friendly,
                COALESCE(ro.lumis_cost, ro.points, 0) as lumis_cost,
                COALESCE(ro.offer_category, 'general') as category,
                COALESCE(ro.merchant_name, 'Comercio Aliado') as merchant_name,
                ro.img as image_url,
                ro.is_active,
                ro.valid_to as offer_expires_at,
                COALESCE(ro.max_redemptions_per_user, 5) as max_redemptions_per_user,
                COALESCE(uos.total_redemptions, 0)::int as user_redemptions_count,
                COALESCE(uos.pending, 0)::int as pending,
                COALESCE(uos.confirmed, 0)::int as confirmed,
                COALESCE(uos.cancelled, 0)::int as cancelled,
                COALESCE(uos.expired, 0)::int as expired,
                uos.last_redeemed_at,
                -- Calcular status de la oferta para el usuario
                CASE 
                    WHEN uos.total_redemptions > 0 THEN 'redeemed'
                    WHEN ro.valid_to IS NOT NULL AND ro.valid_to < NOW() THEN 'expired'
                    WHEN ro.is_active = false THEN 'expired'
                    ELSE 'active'
                END as offer_status,
                -- Nuevos campos: stock y redenciones globales
                ro.stock_quantity as stock_initial,
                GREATEST(0, COALESCE(ro.stock_quantity, 0) - COALESCE(gos.total_global_redemptions, 0)::int) as stock_remaining,
                COALESCE(gos.total_global_redemptions, 0)::int as total_redemptions_count
            FROM rewards.redemption_offers ro
            LEFT JOIN user_offer_stats uos ON ro.offer_id = uos.offer_id
            LEFT JOIN global_offer_stats gos ON ro.offer_id = gos.offer_id
            WHERE 
                -- Filtro por status
                CASE $2
                    WHEN 'active' THEN 
                        uos.total_redemptions IS NULL 
                        AND ro.is_active = true 
                        AND (ro.valid_to IS NULL OR ro.valid_to > NOW())
                    WHEN 'redeemed' THEN 
                        uos.total_redemptions > 0
                    WHEN 'expired' THEN 
                        (ro.valid_to IS NOT NULL AND ro.valid_to < NOW()) 
                        OR ro.is_active = false
                    ELSE true  -- 'all'
                END
                -- Filtro por categoría opcional
                AND ($3::text IS NULL OR ro.offer_category = $3)
            ORDER BY 
                CASE WHEN uos.last_redeemed_at IS NOT NULL THEN 0 ELSE 1 END,
                uos.last_redeemed_at DESC NULLS LAST,
                ro.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(user_id)
        .bind(status_filter)
        .bind(&filters.category)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        let offers = rows
            .into_iter()
            .map(|row| {
                let is_still_available = row.is_active.unwrap_or(false)
                    && row.offer_expires_at.map_or(true, |exp| exp > chrono::Utc::now());

                // Calcular porcentaje de uso solo si hay stock inicial
                let usage_percentage = row.stock_initial.map(|initial| {
                    if initial > 0 {
                        (row.total_redemptions_count as f32 / initial as f32 * 100.0).min(100.0)
                    } else {
                        0.0
                    }
                });

                super::models::MyOfferItem {
                    offer_id: row.offer_id,
                    name_friendly: row.name_friendly,
                    description_friendly: row.description_friendly,
                    lumis_cost: row.lumis_cost,
                    category: row.category,
                    merchant_name: row.merchant_name,
                    image_url: row.image_url,
                    status: row.offer_status,
                    is_still_available,
                    user_redemptions_count: row.user_redemptions_count,
                    max_redemptions_per_user: row.max_redemptions_per_user,
                    last_redeemed_at: row.last_redeemed_at,
                    offer_expires_at: row.offer_expires_at,
                    redemptions_summary: super::models::RedemptionsSummary {
                        total: row.user_redemptions_count,
                        pending: row.pending,
                        confirmed: row.confirmed,
                        cancelled: row.cancelled,
                        expired: row.expired,
                    },
                    stock_initial: row.stock_initial,
                    stock_remaining: row.stock_remaining,
                    total_redemptions_count: row.total_redemptions_count,
                    usage_percentage,
                }
            })
            .collect();

        Ok(offers)
    }
}

// Struct auxiliar para query
#[derive(sqlx::FromRow)]
struct OfferRow {
    offer_id: Uuid,
    name_friendly: String,
    description_friendly: String,
    lumis_cost: i32,
    category: String,
    merchant_name: String,
    image_url: Option<String>,
    stock_quantity: Option<i32>,
    max_redemptions_per_user: i32,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    user_redemptions_count: i64,
}

// Struct auxiliar para my-offers query
#[derive(sqlx::FromRow)]
struct MyOfferRow {
    offer_id: Uuid,
    name_friendly: String,
    description_friendly: String,
    lumis_cost: i32,
    category: String,
    merchant_name: String,
    image_url: Option<String>,
    is_active: Option<bool>,
    offer_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    max_redemptions_per_user: i32,
    user_redemptions_count: i32,
    pending: i32,
    confirmed: i32,
    cancelled: i32,
    expired: i32,
    last_redeemed_at: Option<chrono::DateTime<chrono::Utc>>,
    offer_status: String,
    // Nuevos campos para stock y redenciones globales
    stock_initial: Option<i32>,
    stock_remaining: Option<i32>,
    total_redemptions_count: i32,
}

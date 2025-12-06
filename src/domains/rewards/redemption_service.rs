use super::models::{
    CreateRedemptionRequest, RedemptionCreatedResponse, RedemptionError, UserRedemption, UserRedemptionItem, UserRedemptionStats, CancellationResponse, 
    // AuditActionType, // Unused - para uso futuro
};
use super::offer_service::OfferService;
use super::qr_generator::QrGenerator;
use chrono::{Duration, Utc};
use sqlx::PgPool; // Removed unused Postgres, Transaction
use std::sync::Arc;
use uuid::Uuid;
use crate::observability::metrics::{
    record_redemption_created, record_qr_generated, REDEMPTION_PROCESSING_DURATION,
};
use crate::services::{get_push_service, get_webhook_service};

/// Servicio para gestionar redenciones de usuarios
pub struct RedemptionService {
    db: PgPool,
    offer_service: Arc<OfferService>,
    qr_generator: Arc<QrGenerator>,
}

impl RedemptionService {
    pub fn new(db: PgPool, offer_service: Arc<OfferService>, qr_generator: Arc<QrGenerator>) -> Self {
        Self {
            db,
            offer_service,
            qr_generator,
        }
    }

    /// Crear nueva redención con QR
    pub async fn create_redemption(
        &self,
        request: CreateRedemptionRequest,
        _ip_address: Option<String>, // Prefixed with _ as unused
    ) -> Result<RedemptionCreatedResponse, RedemptionError> {
        let start_time = std::time::Instant::now();
        let user_id = request.user_id;
        
        // 1. Validar oferta
        let offer = self
            .offer_service
            .get_offer_details(request.offer_id, user_id)
            .await?;

        if !offer.is_currently_valid() {
            return Err(RedemptionError::OfferInactive);
        }

        if !offer.has_stock() {
            return Err(RedemptionError::OutOfStock);
        }

        let lumis_cost = offer.get_cost();

        // 2. Verificar balance del usuario
        let user_balance = self.offer_service.get_user_balance(user_id).await?;
        if user_balance < lumis_cost as i64 {
            return Err(RedemptionError::InsufficientBalance {
                current: user_balance,
                required: lumis_cost,
            });
        }

        // 3. Generar código y QR
        let redemption_code = self.qr_generator.generate_redemption_code();
        let code_expires_at = Utc::now() + Duration::minutes(15);
        let landing_url = format!(
            "https://app.lumis.pa/redeem/{}",
            redemption_code
        );
        let qr_image_url = format!(
            "https://cdn.lumis.pa/qr/{}.png",
            redemption_code
        );

        // 4. Iniciar transacción
        let mut tx = self.db.begin().await?;

        let redemption_id = Uuid::new_v4();

        // 5. Insertar redención
        sqlx::query(
            r#"
            INSERT INTO rewards.user_redemptions (
                redemption_id, user_id, offer_id, lumis_spent,
                redemption_code, code_expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(redemption_id)
        .bind(user_id)
        .bind(request.offer_id)
        .bind(lumis_cost)
        .bind(&redemption_code)
        .bind(code_expires_at)
        .execute(&mut *tx)
        .await?;

        // 6. Insertar transacción negativa en fact_accumulations
        sqlx::query(
            r#"
            INSERT INTO rewards.fact_accumulations (
                user_id, accum_type, dtype, quantity, balance, date, redemption_id
            )
            SELECT 
                $1, 'spend', 'points', $2, 
                COALESCE(fbp.balance, 0) - $2,
                NOW(), $3
            FROM rewards.fact_balance_points fbp
            WHERE fbp.user_id = $1
            "#,
        )
        .bind(user_id)
        .bind(lumis_cost)
        .bind(redemption_id)
        .execute(&mut *tx)
        .await?;

        // 7. Actualizar balance
        sqlx::query(
            r#"
            UPDATE rewards.fact_balance_points
            SET balance = balance - $1, latest_update = NOW()
            WHERE user_id = $2
            "#,
        )
        .bind(lumis_cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // 8. Commit
        tx.commit().await?;

        // 9. Obtener nuevo balance
        let new_balance = self.offer_service.get_user_balance(user_id).await?;

        // 10. Registrar métricas
        record_redemption_created("standard", true, lumis_cost as f64);
        record_qr_generated("png");
        REDEMPTION_PROCESSING_DURATION
            .with_label_values(&["create_redemption"])
            .observe(start_time.elapsed().as_secs_f64());

        // ✨ OPTIMIZATION: Calculate offer_name once to avoid multiple clones
        let offer_name = offer.name_friendly.unwrap_or(offer.name);

        // 11. Enviar push notification (asíncrono, no bloqueante)
        if let Some(push_service) = get_push_service() {
            let push_user_id = user_id;
            let push_redemption_id = redemption_id;
            let push_offer_name = offer_name.clone();
            let push_code = redemption_code.clone();
            
            tokio::spawn(async move {
                if let Err(e) = push_service.notify_redemption_created(
                    push_user_id,
                    push_redemption_id,
                    &push_offer_name,
                    &push_code,
                ).await {
                    tracing::error!("Failed to send push notification: {}", e);
                }
            });
        }

        // 12. Enviar webhook si merchant lo tiene configurado
        if let Some(merchant_id) = offer.merchant_id {
            if let Some(webhook_service) = get_webhook_service() {
                let webhook_merchant_id = merchant_id;
                let webhook_redemption_id = redemption_id;
                let webhook_code = redemption_code.clone();
                let webhook_offer_name = offer_name.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = webhook_service.notify_redemption_created(
                        webhook_merchant_id,
                        webhook_redemption_id,
                        &webhook_code,
                        &webhook_offer_name,
                        lumis_cost,
                    ).await {
                        tracing::error!("Failed to send webhook: {}", e);
                    }
                });
            }
        }

        // 13. Retornar respuesta
        Ok(RedemptionCreatedResponse {
            redemption_id,
            redemption_code,
            offer_name,
            lumis_spent: lumis_cost,
            qr_landing_url: landing_url,
            qr_image_url: Some(qr_image_url),
            code_expires_at,
            expires_at: code_expires_at,
            status: "pending".to_string(),
            merchant_name: offer.merchant_name.unwrap_or_default(),
            message: "¡Redención creada! Presenta este código en el comercio.".to_string(),
            new_balance,
        })
    }

    /// Listar redenciones del usuario
    pub async fn get_user_redemptions(
        &self,
        user_id: i32,
        status_filter: Option<String>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<UserRedemptionItem>, RedemptionError> {
        let mut query = String::from(
            r#"
            SELECT 
                ur.redemption_id,
                ur.redemption_code,
                ur.lumis_spent,
                ur.redemption_status,
                ur.code_expires_at,
                ur.qr_landing_url,
                ur.created_at,
                ur.validated_at,
                ro.name_friendly as offer_name,
                COALESCE(ro.merchant_name, 'Comercio Aliado') as merchant_name
            FROM rewards.user_redemptions ur
            INNER JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
            WHERE ur.user_id = $1
            "#,
        );

        let mut param_count = 1;

        if let Some(_) = &status_filter {
            param_count += 1;
            query.push_str(&format!(" AND ur.redemption_status = ${}", param_count));
        }

        query.push_str(" ORDER BY ur.created_at DESC");

        param_count += 1;
        query.push_str(&format!(" LIMIT ${}", param_count));
        param_count += 1;
        query.push_str(&format!(" OFFSET ${}", param_count));

        let mut sql_query = sqlx::query_as::<_, RedemptionRow>(&query).bind(user_id);

        if let Some(ref status) = status_filter {
            sql_query = sql_query.bind(status);
        }

        sql_query = sql_query.bind(limit as i64).bind(offset as i64);

        let rows = sql_query.fetch_all(&self.db).await?;

        let items = rows
            .into_iter()
            .map(|row| UserRedemptionItem {
                redemption_id: row.redemption_id,
                offer_name: row.offer_name,
                merchant_name: Some(row.merchant_name),
                lumis_spent: row.lumis_spent,
                redemption_code: row.redemption_code,
                redemption_status: row.redemption_status,
                code_expires_at: row.code_expires_at,
                qr_landing_url: row.qr_landing_url.unwrap_or_default(),
                created_at: row.created_at,
                validated_at: row.validated_at,
            })
            .collect();

        Ok(items)
    }

    /// Cancelar redención
    pub async fn cancel_redemption(
        &self,
        redemption_id: Uuid,
        user_id: i32,
    ) -> Result<CancellationResponse, RedemptionError> {
        let reason = "Cancelled by user".to_string();
        let mut tx = self.db.begin().await?;

        // 1. Obtener redención
        let redemption = sqlx::query_as::<_, UserRedemption>(
            r#"
            SELECT * FROM rewards.user_redemptions
            WHERE redemption_id = $1 AND user_id = $2
            FOR UPDATE
            "#,
        )
        .bind(redemption_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RedemptionError::RedemptionNotFound)?;

        // 2. Validar que se puede cancelar
        if !redemption.can_be_cancelled() {
            return Err(RedemptionError::CannotCancel {
                status: redemption.redemption_status.clone(),
            });
        }

        // 3. Actualizar estado
        sqlx::query(
            r#"
            UPDATE rewards.user_redemptions
            SET redemption_status = 'cancelled',
                cancelled_at = NOW(),
                cancellation_reason = $1
            WHERE redemption_id = $2
            "#,
        )
        .bind(&reason)
        .bind(redemption_id)
        .execute(&mut *tx)
        .await?;

        // 4. Devolver Lümis
        sqlx::query(
            r#"
            INSERT INTO rewards.fact_accumulations (
                user_id, accum_type, dtype, quantity, balance, date, redemption_id
            )
            SELECT 
                $1, 'earn', 'refund', $2,
                COALESCE(fbp.balance, 0) + $2,
                NOW(), $3
            FROM rewards.fact_balance_points fbp
            WHERE fbp.user_id = $1
            "#,
        )
        .bind(user_id)
        .bind(redemption.lumis_spent)
        .bind(redemption_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE rewards.fact_balance_points
            SET balance = balance + $1, latest_update = NOW()
            WHERE user_id = $2
            "#,
        )
        .bind(redemption.lumis_spent)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let new_balance = self.offer_service.get_user_balance(user_id).await?;

        Ok(CancellationResponse {
            redemption_id,
            success: true,
            lumis_refunded: redemption.lumis_spent,
            new_balance,
            message: "Redención cancelada y Lümis devueltos exitosamente".to_string(),
        })
    }

    /// Obtener estadísticas de usuario
    pub async fn get_user_redemption_stats(&self, user_id: i32) -> Result<UserRedemptionStats, RedemptionError> {
        let stats = sqlx::query_as::<_, UserRedemptionStats>(
            r#"
            SELECT 
                COUNT(*) as total_redemptions,
                COUNT(*) FILTER (WHERE redemption_status = 'pending') as pending,
                COUNT(*) FILTER (WHERE redemption_status = 'confirmed') as confirmed,
                COUNT(*) FILTER (WHERE redemption_status = 'cancelled') as cancelled,
                COUNT(*) FILTER (WHERE redemption_status = 'expired') as expired,
                COALESCE(SUM(lumis_spent) FILTER (WHERE redemption_status = 'confirmed'), 0) as total_lumis_spent
            FROM rewards.user_redemptions
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(stats)
    }
    
    /// Obtener una redención específica por ID y verificar pertenencia
    pub async fn get_redemption_by_id(
        &self,
        redemption_id: Uuid,
        user_id: i32,
    ) -> Result<UserRedemptionItem, RedemptionError> {
        let row = sqlx::query_as::<_, RedemptionRow>(
            r#"
            SELECT 
                ur.redemption_id,
                ur.redemption_code,
                ur.lumis_spent,
                ur.redemption_status,
                ur.code_expires_at,
                ur.qr_landing_url,
                ur.created_at,
                ur.validated_at,
                ro.name_friendly as offer_name,
                COALESCE(ro.merchant_name, 'Comercio Aliado') as merchant_name
            FROM rewards.user_redemptions ur
            INNER JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
            WHERE ur.redemption_id = $1 AND ur.user_id = $2
            "#,
        )
        .bind(redemption_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or(RedemptionError::RedemptionNotFound)?;

        Ok(UserRedemptionItem {
            redemption_id: row.redemption_id,
            offer_name: row.offer_name,
            merchant_name: Some(row.merchant_name),
            lumis_spent: row.lumis_spent,
            redemption_code: row.redemption_code,
            redemption_status: row.redemption_status,
            code_expires_at: row.code_expires_at,
            qr_landing_url: row.qr_landing_url.unwrap_or_default(),
            created_at: row.created_at,
            validated_at: row.validated_at,
        })
    }
}

// Struct auxiliar para query
#[derive(sqlx::FromRow)]
struct RedemptionRow {
    redemption_id: Uuid,
    redemption_code: String,
    lumis_spent: i32,
    redemption_status: String,
    code_expires_at: chrono::DateTime<chrono::Utc>,
    qr_landing_url: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    validated_at: Option<chrono::DateTime<chrono::Utc>>,
    offer_name: String,
    merchant_name: String,
}

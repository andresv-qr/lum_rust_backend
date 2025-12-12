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
        
        // 1. Validar oferta (lectura inicial sin lock)
        let offer = self
            .offer_service
            .get_offer_details(request.offer_id, user_id)
            .await?;

        if !offer.is_currently_valid() {
            return Err(RedemptionError::OfferInactive);
        }

        // Verificación inicial de stock (sin lock, para fail-fast)
        if !offer.has_stock() {
            return Err(RedemptionError::OutOfStock);
        }

        let lumis_cost = offer.get_cost();

        // 2. Verificar balance del usuario (lectura inicial)
        let user_balance = self.offer_service.get_user_balance(user_id).await?;
        if user_balance < lumis_cost as i64 {
            return Err(RedemptionError::InsufficientBalance {
                current: user_balance,
                required: lumis_cost,
            });
        }

        // 3. Generar código único
        let redemption_code = self.qr_generator.generate_redemption_code();
        let code_expires_at = Utc::now() + Duration::minutes(15);
        let redemption_id = Uuid::new_v4();

        // 4. Generar token de validación JWT (para QR seguro)
        let validation_token = self.qr_generator
            .generate_validation_token(&redemption_code, user_id, &redemption_id)
            .map_err(|e| RedemptionError::QRGenerationFailed(e.to_string()))?;
        
        // 5. Generar hash del token para almacenar en DB
        let token_hash = super::qr_generator::QrGenerator::hash_token(&validation_token);

        // 6. Generar QR con logo
        let qr_image_bytes = match self.qr_generator.generate_qr_with_logo(&redemption_code, &validation_token).await {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!("Failed to generate QR with logo, using simple QR: {}", e);
                // Fallback a QR simple sin logo
                self.qr_generator.generate_qr_simple(&redemption_code).ok()
            }
        };

        // 7. Guardar QR en storage local (o subir a CDN en producción)
        let qr_image_url = if let Some(ref bytes) = qr_image_bytes {
            // Por ahora guardamos en sistema de archivos local
            // TODO: Integrar con S3 o CDN en producción
            let qr_filename = format!("{}.png", redemption_code);
            let qr_path = format!("assets/qr/{}", qr_filename);
            
            // Crear directorio si no existe
            let _ = std::fs::create_dir_all("assets/qr");
            
            if std::fs::write(&qr_path, bytes).is_ok() {
                Some(format!("https://api.lumis.pa/static/qr/{}", qr_filename))
            } else {
                tracing::warn!("Failed to save QR image to disk");
                None
            }
        } else {
            None
        };

        // Landing URL para el QR
        let landing_url = self.qr_generator.generate_landing_url(&redemption_code, Some(&validation_token));

        // 8. Iniciar transacción
        let mut tx = self.db.begin().await?;

        // 8.1. CRITICAL: Re-verificar stock con SELECT FOR UPDATE para evitar race condition
        let current_stock: Option<Option<i32>> = sqlx::query_scalar(
            r#"
            SELECT stock_quantity 
            FROM rewards.redemption_offers 
            WHERE offer_id = $1 
            FOR UPDATE
            "#
        )
        .bind(request.offer_id)
        .fetch_optional(&mut *tx)
        .await?;
        
        // Verificar que hay stock disponible (NULL = ilimitado, Some(n) = n disponibles)
        if let Some(Some(stock)) = current_stock {
            if stock <= 0 {
                // Rollback implícito al dropear tx
                return Err(RedemptionError::OutOfStock);
            }
            
            // Decrementar stock
            sqlx::query(
                r#"
                UPDATE rewards.redemption_offers 
                SET stock_quantity = stock_quantity - 1 
                WHERE offer_id = $1
                "#
            )
            .bind(request.offer_id)
            .execute(&mut *tx)
            .await?;
        }

        // 8.2. Verificar balance con lock para evitar race condition en balance
        let locked_balance: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(balance::bigint, 0)
            FROM rewards.fact_balance_points
            WHERE user_id = $1
            FOR UPDATE
            "#
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        
        if locked_balance.unwrap_or(0) < lumis_cost as i64 {
            return Err(RedemptionError::InsufficientBalance {
                current: locked_balance.unwrap_or(0),
                required: lumis_cost,
            });
        }

        // 9. Insertar redención con token hash
        sqlx::query(
            r#"
            INSERT INTO rewards.user_redemptions (
                redemption_id, user_id, offer_id, lumis_spent,
                redemption_code, code_expires_at, qr_landing_url, 
                qr_image_url, validation_token_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(redemption_id)
        .bind(user_id)
        .bind(request.offer_id)
        .bind(lumis_cost)
        .bind(&redemption_code)
        .bind(code_expires_at)
        .bind(&landing_url)
        .bind(&qr_image_url)
        .bind(&token_hash)
        .execute(&mut *tx)
        .await?;

        // 10. Insertar transacción negativa en fact_accumulations
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

        // 11. Actualizar balance
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

        // 12. Commit
        tx.commit().await?;

        // 13. Obtener nuevo balance
        let new_balance = self.offer_service.get_user_balance(user_id).await?;

        // 14. Registrar métricas
        record_redemption_created("standard", true, lumis_cost as f64);
        if qr_image_bytes.is_some() {
            record_qr_generated("png_with_logo");
        }
        REDEMPTION_PROCESSING_DURATION
            .with_label_values(&["create_redemption"])
            .observe(start_time.elapsed().as_secs_f64());

        // ✨ OPTIMIZATION: Calculate offer_name once to avoid multiple clones
        let offer_name = offer.name_friendly.unwrap_or(offer.name);

        // 15. Enviar push notification (asíncrono, no bloqueante)
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

        // 16. Enviar webhook si merchant lo tiene configurado
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

        // 17. Retornar respuesta
        Ok(RedemptionCreatedResponse {
            redemption_id,
            redemption_code,
            offer_name,
            lumis_spent: lumis_cost,
            qr_landing_url: landing_url,
            qr_image_url,
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
            .map(|row| UserRedemptionItem::new(
                row.redemption_id,
                row.offer_name,
                Some(row.merchant_name),
                row.lumis_spent,
                row.redemption_code,
                row.qr_landing_url.unwrap_or_default(),
                row.redemption_status,
                row.code_expires_at,
                row.created_at,
                row.validated_at,
            ))
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

        // 5. CRÍTICO: Restaurar stock de la oferta
        // Primero obtener offer_id de la redención
        let offer_id_row: Option<(Uuid,)> = sqlx::query_as(
            r#"SELECT offer_id FROM rewards.user_redemptions WHERE redemption_id = $1"#
        )
        .bind(redemption_id)
        .fetch_optional(&mut *tx)
        .await?;
        
        if let Some((offer_id,)) = offer_id_row {
            sqlx::query(
                r#"
                UPDATE rewards.redemption_offers
                SET stock_quantity = COALESCE(stock_quantity, 0) + 1
                WHERE offer_id = $1
                  AND stock_quantity IS NOT NULL
                "#,
            )
            .bind(offer_id)
            .execute(&mut *tx)
            .await?;
        }

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

        Ok(UserRedemptionItem::new(
            row.redemption_id,
            row.offer_name,
            Some(row.merchant_name),
            row.lumis_spent,
            row.redemption_code,
            row.qr_landing_url.unwrap_or_default(),
            row.redemption_status,
            row.code_expires_at,
            row.created_at,
            row.validated_at,
        ))
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

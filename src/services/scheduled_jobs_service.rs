// ============================================================================
// SCHEDULED JOBS SERVICE - Tareas programadas con Cron
// ============================================================================

use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};
use crate::observability::metrics::record_redemption_expired;

pub struct ScheduledJobsService {
    scheduler: JobScheduler,
    db: PgPool,
}

impl ScheduledJobsService {
    pub async fn new(db: PgPool) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;

        Ok(Self { scheduler, db })
    }

    /// Iniciar todos los jobs programados
    pub async fn start(&self) -> Result<()> {
        info!("Starting scheduled jobs...");

        // Job 1: Expirar redenciones antiguas (cada hora)
        self.add_expire_redemptions_job().await?;

        // Job 2: Limpiar códigos QR antiguos (cada día a las 3 AM)
        self.add_cleanup_old_qr_codes_job().await?;

        // Job 2.5: Refresh integrity materialized views (cada día a las 3:15 AM)
        self.add_refresh_integrity_views_job().await?;

        // Job 3: Recalcular stats de merchants (cada día a las 4 AM)
        self.add_recalculate_merchant_stats_job().await?;

        // Job 4: Enviar alertas de redenciones próximas a expirar (cada 5 minutos)
        self.add_expiration_alerts_job().await?;

        // Job 5: Enviar reportes semanales a comercios (domingos a las 9 AM)
        self.add_weekly_merchant_reports_job().await?;

        // Iniciar el scheduler
        self.scheduler.start().await?;

        info!("All scheduled jobs started successfully");
        Ok(())
    }

    /// Job 1: Expirar redenciones pendientes que ya pasaron su fecha
    async fn add_expire_redemptions_job(&self) -> Result<()> {
        let db = self.db.clone();

        let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
            let db = db.clone();
            Box::pin(async move {
                info!("Running expire_redemptions job...");

                match expire_old_redemptions(&db).await {
                    Ok(count) => {
                        info!("Expired {} redemptions", count);
                        for _ in 0..count {
                            record_redemption_expired("auto_expired");
                        }
                    }
                    Err(e) => error!("Error expiring redemptions: {}", e),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        info!("Added expire_redemptions job (hourly)");
        Ok(())
    }

    /// Job 2: Limpiar códigos QR antiguos (>30 días)
    async fn add_cleanup_old_qr_codes_job(&self) -> Result<()> {
        let db = self.db.clone();

        let job = Job::new_async("0 0 3 * * *", move |_uuid, _l| {
            let db = db.clone();
            Box::pin(async move {
                info!("Running cleanup_old_qr_codes job...");

                match cleanup_old_qr_codes(&db).await {
                    Ok(count) => info!("Cleaned up {} old QR codes", count),
                    Err(e) => error!("Error cleaning up QR codes: {}", e),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        info!("Added cleanup_old_qr_codes job (daily at 3 AM)");
        Ok(())
    }

    /// Job 2.5: Refresh integrity materialized views (3:15 AM UTC daily)
    async fn add_refresh_integrity_views_job(&self) -> Result<()> {
        let db = self.db.clone();

        let job = Job::new_async("0 15 3 * * *", move |_uuid, _l| {
            let db = db.clone();
            Box::pin(async move {
                info!("🔄 Starting daily integrity views refresh...");

                match refresh_integrity_materialized_views(&db).await {
                    Ok(_) => info!("✅ Integrity views refreshed successfully"),
                    Err(e) => error!("❌ Error refreshing integrity views: {}", e),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        info!("Added refresh_integrity_views job (daily at 3:15 AM)");
        Ok(())
    }

    /// Job 3: Recalcular estadísticas de merchants
    async fn add_recalculate_merchant_stats_job(&self) -> Result<()> {
        let db = self.db.clone();

        let job = Job::new_async("0 0 4 * * *", move |_uuid, _l| {
            let db = db.clone();
            Box::pin(async move {
                info!("Running recalculate_merchant_stats job...");

                match recalculate_merchant_stats(&db).await {
                    Ok(count) => info!("Recalculated stats for {} merchants", count),
                    Err(e) => error!("Error recalculating merchant stats: {}", e),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        info!("Added recalculate_merchant_stats job (daily at 4 AM)");
        Ok(())
    }

    /// Job 4: Enviar alertas de redenciones próximas a expirar
    async fn add_expiration_alerts_job(&self) -> Result<()> {
        let db = self.db.clone();

        let job = Job::new_async("0 */5 * * * *", move |_uuid, _l| {
            let db = db.clone();
            Box::pin(async move {
                info!("Running expiration_alerts job...");

                match send_expiration_alerts(&db).await {
                    Ok(count) => info!("Sent {} expiration alerts", count),
                    Err(e) => error!("Error sending expiration alerts: {}", e),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        info!("Added expiration_alerts job (every 5 minutes)");
        Ok(())
    }

    /// Job 5: Enviar reportes semanales a comercios (domingos a las 9 AM UTC)
    async fn add_weekly_merchant_reports_job(&self) -> Result<()> {
        let db = self.db.clone();

        let job = Job::new_async("0 0 9 * * SUN", move |_uuid, _l| {
            let db = db.clone();
            Box::pin(async move {
                info!("📧 Running weekly merchant reports job...");

                match crate::services::send_weekly_reports_task(db.clone()).await {
                    Ok(_) => info!("✅ Weekly merchant reports sent successfully"),
                    Err(e) => error!("❌ Error sending weekly merchant reports: {}", e),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        info!("Added weekly_merchant_reports job (Sundays at 9 AM)");
        Ok(())
    }

    /// Detener el scheduler
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down scheduled jobs...");
        self.scheduler.shutdown().await?;
        Ok(())
    }
}

// ============================================================================
// JOB IMPLEMENTATIONS
// ============================================================================

/// Expirar redenciones antiguas
async fn expire_old_redemptions(db: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE rewards.user_redemptions
        SET 
            redemption_status = 'expired',
            updated_at = NOW()
        WHERE redemption_status = 'pending'
          AND code_expires_at < NOW()
        "#,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

/// Limpiar códigos QR antiguos (opcional: si se almacenan imágenes)
async fn cleanup_old_qr_codes(db: &PgPool) -> Result<u64> {
    // 1. Limpiar tabla de cache si existe
    let db_result = sqlx::query(
        r#"
        DELETE FROM rewards.qr_code_cache
        WHERE created_at < NOW() - INTERVAL '30 days'
        "#,
    )
    .execute(db)
    .await;

    let db_count = match db_result {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            // Si la tabla no existe, no es un error
            if e.to_string().contains("does not exist") {
                0
            } else {
                return Err(e.into());
            }
        }
    };
    
    // 2. Limpiar archivos QR del sistema de archivos (assets/qr/)
    let qr_dir = std::path::Path::new("assets/qr");
    let mut files_deleted = 0u64;
    
    if qr_dir.exists() {
        let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
        
        if let Ok(entries) = std::fs::read_dir(qr_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time: chrono::DateTime<chrono::Utc> = modified.into();
                        if modified_time < thirty_days_ago {
                            if std::fs::remove_file(entry.path()).is_ok() {
                                files_deleted += 1;
                            }
                        }
                    }
                }
            }
        }
        
        if files_deleted > 0 {
            info!("🗑️ Cleaned up {} old QR image files", files_deleted);
        }
    }
    
    Ok(db_count + files_deleted)
}

/// Recalcular estadísticas de merchants
async fn recalculate_merchant_stats(db: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE rewards.merchants m
        SET 
            total_redemptions = (
                SELECT COUNT(*) 
                FROM rewards.user_redemptions ur
                WHERE ur.merchant_id = m.merchant_id
                  AND ur.redemption_status = 'confirmed'
            ),
            total_revenue = (
                SELECT COALESCE(SUM(lumis_cost), 0)
                FROM rewards.user_redemptions ur
                JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
                WHERE ur.merchant_id = m.merchant_id
                  AND ur.redemption_status = 'confirmed'
            ),
            last_stats_update = NOW()
        WHERE is_active = true
        "#,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

/// Enviar alertas de redenciones próximas a expirar
async fn send_expiration_alerts(db: &PgPool) -> Result<u64> {
    use crate::services::push_notification_service::get_push_service;

    // Obtener redenciones que expiran en los próximos 5 minutos
    let expiring_redemptions = sqlx::query_as::<_, ExpiringRedemption>(
        r#"
        SELECT 
            ur.redemption_id,
            ur.user_id,
            ur.code_expires_at,
            COALESCE(ro.name_friendly, ro.name) as offer_name
        FROM rewards.user_redemptions ur
        JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
        WHERE ur.redemption_status = 'pending'
          AND ur.code_expires_at BETWEEN NOW() AND NOW() + INTERVAL '5 minutes'
          AND ur.expiration_alert_sent = false
        "#,
    )
    .fetch_all(db)
    .await?;

    let count = expiring_redemptions.len() as u64;

    if let Some(push_service) = get_push_service() {
        for redemption in expiring_redemptions {
            let minutes_remaining = (redemption.code_expires_at - chrono::Utc::now())
                .num_minutes() as i32;

            if let Err(e) = push_service
                .notify_redemption_expiring(
                    redemption.user_id,
                    redemption.redemption_id,
                    &redemption.offer_name,
                    minutes_remaining,
                )
                .await
            {
                error!("Failed to send expiration alert: {}", e);
            } else {
                // Marcar como enviada
                let _ = sqlx::query(
                    r#"
                    UPDATE rewards.user_redemptions
                    SET expiration_alert_sent = true
                    WHERE redemption_id = $1
                    "#,
                )
                .bind(redemption.redemption_id)
                .execute(db)
                .await;
            }
        }
    }

    Ok(count)
}

#[derive(sqlx::FromRow)]
struct ExpiringRedemption {
    redemption_id: uuid::Uuid,
    user_id: i32,
    code_expires_at: chrono::DateTime<chrono::Utc>,
    offer_name: String,
}

/// Actualiza las vistas materializadas de integridad
/// Se ejecuta a las 3:15 AM UTC para refrescar los hashes de validación
async fn refresh_integrity_materialized_views(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    info!("Starting refresh of integrity materialized views");
    
    let start = std::time::Instant::now();
    
    // Refrescar las 4 vistas materializadas de forma concurrente
    let views = [
        "user_product_integrity_daily",
        "user_issuer_integrity_daily", 
        "user_header_integrity_daily",
        "user_detail_integrity_daily",
    ];
    
    for view in views.iter() {
        let view_start = std::time::Instant::now();
        
        let sql = format!("REFRESH MATERIALIZED VIEW CONCURRENTLY {}", view);
        
        match sqlx::query(&sql).execute(db).await {
            Ok(_) => {
                let elapsed = view_start.elapsed();
                info!("✓ Refreshed {} in {:?}", view, elapsed);
            }
            Err(e) => {
                error!("✗ Failed to refresh {}: {}", view, e);
                return Err(e);
            }
        }
    }
    
    let total_elapsed = start.elapsed();
    info!(
        "Completed refresh of all integrity views in {:?}",
        total_elapsed
    );
    
    Ok(())
}

// ============================================================================
// SHARED INSTANCE
// ============================================================================

use std::sync::OnceLock;

static SCHEDULED_JOBS: OnceLock<Arc<ScheduledJobsService>> = OnceLock::new();

pub async fn init_scheduled_jobs(db: PgPool) -> Result<()> {
    let service = Arc::new(ScheduledJobsService::new(db).await?);
    service.start().await?;
    
    if SCHEDULED_JOBS.set(service).is_err() {
        error!("Scheduled jobs already initialized");
    }
    
    Ok(())
}

pub fn get_scheduled_jobs() -> Option<Arc<ScheduledJobsService>> {
    SCHEDULED_JOBS.get().cloned()
}

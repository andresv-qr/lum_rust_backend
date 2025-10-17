use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};
use std::sync::Arc;
use chrono::Timelike; // Para poder usar .hour()

use crate::api::ofertasws_v4::{get_ofertasws_cached, log_refresh_execution};

/// Inicia el scheduler para auto-refresh de ofertas
/// Ejecuta a las 10am y 3pm hora Panamá (UTC-5)
/// 10am Panamá = 3pm UTC
/// 3pm Panamá = 8pm UTC
pub async fn start_ofertasws_refresh_scheduler(
    ws_pool: PgPool,
    redis_pool: deadpool_redis::Pool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = JobScheduler::new().await?;
    
    // Wrappear una sola vez en Arc - evita clones innecesarios
    let ws_pool = Arc::new(ws_pool);
    let redis_pool = Arc::new(redis_pool);
    
    // Job 1: 10am Panamá (3pm UTC) = "0 0 15 * * *"
    let job_10am = {
        let ws_pool = Arc::clone(&ws_pool);
        let redis_pool = Arc::clone(&redis_pool);
        
        Job::new_async("0 0 15 * * *", move |_uuid, _lock| {
            let ws_pool = Arc::clone(&ws_pool);
            let redis_pool = Arc::clone(&redis_pool);
            
            Box::pin(async move {
                tracing::info!("⏰ Executing scheduled ofertasws refresh (10am Panamá)");
                execute_refresh(&ws_pool, &redis_pool).await;
            })
        })?
    };
    
    // Job 2: 3pm Panamá (8pm UTC) = "0 0 20 * * *"
    let job_3pm = {
        let ws_pool = Arc::clone(&ws_pool);
        let redis_pool = Arc::clone(&redis_pool);
        
        Job::new_async("0 0 20 * * *", move |_uuid, _lock| {
            let ws_pool = Arc::clone(&ws_pool);
            let redis_pool = Arc::clone(&redis_pool);
            
            Box::pin(async move {
                tracing::info!("⏰ Executing scheduled ofertasws refresh (3pm Panamá)");
                execute_refresh(&ws_pool, &redis_pool).await;
            })
        })?
    };
    
    scheduler.add(job_10am).await?;
    scheduler.add(job_3pm).await?;
    
    scheduler.start().await?;
    
    tracing::info!("✅ OfertasWs refresh scheduler started");
    tracing::info!("   → 10am Panamá (3pm UTC): Daily refresh");
    tracing::info!("   → 3pm Panamá (8pm UTC): Daily refresh");
    
    Ok(())
}

/// Ejecuta el refresh invalidando cache y regenerando
async fn execute_refresh(ws_pool: &PgPool, redis_pool: &deadpool_redis::Pool) {
    let start = std::time::Instant::now();
    
    // Generar cache key
    let now = chrono::Utc::now().with_timezone(&chrono_tz::America::Panama);
    let slot_hour = if now.hour() < 15 { 10 } else { 15 };
    let cache_key = format!(
        "ofertasws:cache:{}:{:02}:00",
        now.format("%Y-%m-%d"),
        slot_hour
    );
    
    tracing::info!("🔄 Starting ofertasws refresh for key: {}", cache_key);
    
    // Invalidar cache anterior
    match redis_pool.get().await {
        Ok(mut conn) => {
            let _: Result<(), redis::RedisError> = redis::cmd("DEL")
                .arg(&cache_key)
                .query_async(&mut *conn)
                .await;
        }
        Err(e) => {
            tracing::error!("Redis connection error: {}", e);
        }
    }
    
    // Regenerar cache
    match get_ofertasws_cached(ws_pool, redis_pool).await {
        Ok((compressed_data, _etag, count)) => {
            let execution_time = start.elapsed().as_millis() as i32;
            
            // Ya tenemos el count del resultado - no necesitamos descomprimir (optimización)
            
            tracing::info!(
                "✅ Scheduled refresh completed: {} ofertas, {} bytes, {}ms",
                count,
                compressed_data.len(),
                execution_time
            );
            
            // Log exitoso (el log ya se hace en get_ofertasws_cached)
        }
        Err(e) => {
            let execution_time = start.elapsed().as_millis() as i32;
            
            tracing::error!("❌ Scheduled refresh failed: {}", e);
            
            // Log error
            if let Err(log_err) = log_refresh_execution(
                ws_pool,
                "error",
                None,
                execution_time,
                None,
                Some(&e),
                &cache_key,
            )
            .await
            {
                tracing::error!("Failed to log error: {}", log_err);
            }
        }
    }
}

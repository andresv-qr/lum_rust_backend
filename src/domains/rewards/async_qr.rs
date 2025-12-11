// ============================================================================
// QR ASYNC GENERATOR - Generación asíncrona de códigos QR
// ============================================================================
//
// Permite generar QRs en background para mejorar tiempo de respuesta
// La redención se crea inmediatamente y el QR se genera en una tarea separada
//
// Flow:
// 1. Usuario solicita redención
// 2. Se crea redención con status="pending_qr"
// 3. Se encola tarea de generación QR
// 4. Worker genera QR y actualiza redención
// 5. Se notifica al usuario cuando el QR está listo
//

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::domains::rewards::qr_generator::{QrGenerator, QrConfig};

/// Mensaje para la cola de generación de QR
#[derive(Debug, Clone)]
pub struct QrGenerationTask {
    pub redemption_id: Uuid,
    pub redemption_code: String,
    pub user_id: i32,
    pub validation_token: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub retry_count: u32,
}

/// Resultado de la generación de QR
#[derive(Debug)]
pub struct QrGenerationResult {
    pub redemption_id: Uuid,
    pub success: bool,
    pub qr_image_bytes: Option<Vec<u8>>,
    pub qr_image_url: Option<String>,
    pub landing_url: Option<String>,
    pub error: Option<String>,
}

/// Configuración del worker de QR
#[derive(Clone)]
pub struct QrWorkerConfig {
    /// Número máximo de reintentos
    pub max_retries: u32,
    /// Delay entre reintentos (segundos)
    pub retry_delay_secs: u64,
    /// Tamaño del buffer del canal
    pub channel_buffer_size: usize,
    /// Número de workers paralelos
    pub num_workers: usize,
}

impl Default for QrWorkerConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_secs: 2,
            channel_buffer_size: 1000,
            num_workers: 2,
        }
    }
}

/// Servicio de generación asíncrona de QR
pub struct AsyncQrService {
    sender: mpsc::Sender<QrGenerationTask>,
    qr_generator: Arc<QrGenerator>,
    #[allow(dead_code)]
    db_pool: sqlx::PgPool,
    #[allow(dead_code)]
    config: QrWorkerConfig,
}

impl AsyncQrService {
    /// Crea el servicio e inicia los workers
    pub fn new(
        db_pool: sqlx::PgPool,
        qr_config: QrConfig,
        worker_config: QrWorkerConfig,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(worker_config.channel_buffer_size);
        let qr_generator = Arc::new(QrGenerator::new(qr_config));
        
        let service = Self {
            sender,
            qr_generator: Arc::clone(&qr_generator),
            db_pool: db_pool.clone(),
            config: worker_config.clone(),
        };
        
        // Iniciar workers
        Self::spawn_workers(
            receiver,
            Arc::clone(&qr_generator),
            db_pool,
            worker_config,
        );
        
        service
    }
    
    /// Encola una tarea de generación de QR
    pub async fn enqueue(&self, task: QrGenerationTask) -> Result<(), String> {
        self.sender
            .send(task)
            .await
            .map_err(|e| format!("Failed to enqueue QR task: {}", e))
    }
    
    /// Genera QR inmediato (para casos donde se necesita respuesta síncrona)
    pub async fn generate_immediate(
        &self,
        redemption_code: &str,
        user_id: i32,
        redemption_id: &Uuid,
    ) -> Result<(Vec<u8>, String, String), String> {
        // Generar token de validación
        let validation_token = self.qr_generator
            .generate_validation_token(redemption_code, user_id, redemption_id)
            .map_err(|e| format!("Token generation failed: {}", e))?;
        
        // Generar QR con logo
        let qr_bytes = self.qr_generator
            .generate_qr_with_logo(redemption_code, &validation_token)
            .await
            .map_err(|e| format!("QR generation failed: {}", e))?;
        
        // Generar landing URL
        let landing_url = self.qr_generator
            .generate_landing_url(redemption_code, Some(&validation_token));
        
        // Guardar QR en archivo
        let qr_filename = format!("{}.png", redemption_code);
        let qr_path = format!("assets/qr/{}", qr_filename);
        let _ = std::fs::create_dir_all("assets/qr");
        
        if std::fs::write(&qr_path, &qr_bytes).is_err() {
            warn!("Failed to save QR to disk: {}", qr_path);
        }
        
        let qr_url = format!("https://api.lumis.pa/static/qr/{}", qr_filename);
        
        Ok((qr_bytes, qr_url, landing_url))
    }
    
    /// Spawns los workers que procesan la cola
    fn spawn_workers(
        receiver: mpsc::Receiver<QrGenerationTask>,
        qr_generator: Arc<QrGenerator>,
        db_pool: sqlx::PgPool,
        config: QrWorkerConfig,
    ) {
        // Crear un receiver compartido usando Arc<Mutex>
        let receiver = Arc::new(tokio::sync::Mutex::new(receiver));
        
        for worker_id in 0..config.num_workers {
            let qr_gen = Arc::clone(&qr_generator);
            let pool = db_pool.clone();
            let cfg = config.clone();
            let rx = Arc::clone(&receiver);
            
            tokio::spawn(async move {
                info!("QR async worker {} started", worker_id);
                
                loop {
                    let task = {
                        let mut guard = rx.lock().await;
                        guard.recv().await
                    };
                    
                    match task {
                        Some(task) => {
                            Self::process_task(
                                worker_id,
                                task,
                                Arc::clone(&qr_gen),
                                pool.clone(),
                                &cfg,
                            ).await;
                        }
                        None => {
                            info!("QR async worker {} shutting down", worker_id);
                            break;
                        }
                    }
                }
            });
        }
    }
    
    /// Procesa una tarea de generación de QR
    async fn process_task(
        worker_id: usize,
        mut task: QrGenerationTask,
        qr_generator: Arc<QrGenerator>,
        db_pool: sqlx::PgPool,
        config: &QrWorkerConfig,
    ) {
        debug!("Worker {} processing QR for redemption {}", worker_id, task.redemption_id);
        
        loop {
            match Self::generate_and_save(
                &task,
                Arc::clone(&qr_generator),
                &db_pool,
            ).await {
                Ok(result) => {
                    // Actualizar redención con el QR
                    if let Err(e) = Self::update_redemption(
                        &db_pool,
                        &task.redemption_id,
                        &result,
                    ).await {
                        error!("Failed to update redemption with QR: {}", e);
                    } else {
                        info!("QR generated successfully for redemption {}", task.redemption_id);
                    }
                    break;
                }
                Err(e) => {
                    task.retry_count += 1;
                    
                    if task.retry_count >= config.max_retries {
                        error!(
                            "Failed to generate QR for redemption {} after {} retries: {}",
                            task.redemption_id, config.max_retries, e
                        );
                        
                        // Marcar redención con error de QR
                        let _ = Self::mark_qr_error(&db_pool, &task.redemption_id, &e).await;
                        break;
                    }
                    
                    warn!(
                        "QR generation failed for {}, retry {}/{}: {}",
                        task.redemption_id, task.retry_count, config.max_retries, e
                    );
                    
                    tokio::time::sleep(Duration::from_secs(config.retry_delay_secs)).await;
                }
            }
        }
    }
    
    /// Genera el QR y guarda el archivo
    async fn generate_and_save(
        task: &QrGenerationTask,
        qr_generator: Arc<QrGenerator>,
        _db_pool: &sqlx::PgPool,
    ) -> Result<QrGenerationResult, String> {
        // Generar QR con logo
        let qr_bytes = qr_generator
            .generate_qr_with_logo(&task.redemption_code, &task.validation_token)
            .await
            .map_err(|e| format!("QR generation failed: {}", e))?;
        
        // Generar landing URL
        let landing_url = qr_generator
            .generate_landing_url(&task.redemption_code, Some(&task.validation_token));
        
        // Guardar QR en archivo
        let qr_filename = format!("{}.png", task.redemption_code);
        let qr_path = format!("assets/qr/{}", qr_filename);
        
        let _ = std::fs::create_dir_all("assets/qr");
        std::fs::write(&qr_path, &qr_bytes)
            .map_err(|e| format!("Failed to save QR file: {}", e))?;
        
        let qr_url = format!("https://api.lumis.pa/static/qr/{}", qr_filename);
        
        Ok(QrGenerationResult {
            redemption_id: task.redemption_id,
            success: true,
            qr_image_bytes: Some(qr_bytes),
            qr_image_url: Some(qr_url),
            landing_url: Some(landing_url),
            error: None,
        })
    }
    
    /// Actualiza la redención con la información del QR
    async fn update_redemption(
        db_pool: &sqlx::PgPool,
        redemption_id: &Uuid,
        result: &QrGenerationResult,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE rewards.user_redemptions
            SET 
                qr_image_url = $2,
                qr_landing_url = $3,
                redemption_status = CASE 
                    WHEN redemption_status = 'pending_qr' THEN 'pending'
                    ELSE redemption_status
                END
            WHERE redemption_id = $1
            "#
        )
        .bind(redemption_id)
        .bind(&result.qr_image_url)
        .bind(&result.landing_url)
        .execute(db_pool)
        .await
        .map_err(|e| format!("Database update failed: {}", e))?;
        
        Ok(())
    }
    
    /// Marca la redención con error de generación de QR
    async fn mark_qr_error(
        db_pool: &sqlx::PgPool,
        redemption_id: &Uuid,
        error: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE rewards.user_redemptions
            SET 
                redemption_status = 'qr_error',
                cancellation_reason = $2
            WHERE redemption_id = $1
            "#
        )
        .bind(redemption_id)
        .bind(error)
        .execute(db_pool)
        .await
        .map_err(|e| format!("Database update failed: {}", e))?;
        
        Ok(())
    }
}

/// Worker para procesar QRs pendientes al arrancar el servidor
pub async fn recover_pending_qrs(
    db_pool: &sqlx::PgPool,
    async_qr_service: &AsyncQrService,
) {
    info!("Recovering pending QR generation tasks...");
    
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct PendingQr {
        redemption_id: Uuid,
        redemption_code: String,
        user_id: i32,
        validation_token_hash: Option<String>,
    }
    
    let pending: Vec<PendingQr> = match sqlx::query_as(
        r#"
        SELECT redemption_id, redemption_code, user_id, validation_token_hash
        FROM rewards.user_redemptions
        WHERE redemption_status = 'pending_qr'
          AND created_at > NOW() - INTERVAL '1 hour'
        "#
    )
    .fetch_all(db_pool)
    .await {
        Ok(rows) => rows,
        Err(e) => {
            error!("Failed to recover pending QRs: {}", e);
            return;
        }
    };
    
    if pending.is_empty() {
        info!("No pending QR tasks to recover");
        return;
    }
    
    info!("Found {} pending QR tasks, re-enqueueing...", pending.len());
    
    for pq in pending {
        // Para recuperación, necesitamos regenerar el token
        // Como no tenemos el token original, regeneramos
        if let Ok((_, _, _)) = async_qr_service.generate_immediate(
            &pq.redemption_code,
            pq.user_id,
            &pq.redemption_id,
        ).await {
            // Actualizar en DB
            let _ = sqlx::query(
                "UPDATE rewards.user_redemptions SET redemption_status = 'pending' WHERE redemption_id = $1"
            )
            .bind(pq.redemption_id)
            .execute(db_pool)
            .await;
            
            debug!("Recovered QR for redemption {}", pq.redemption_id);
        }
    }
    
    info!("Finished recovering pending QR tasks");
}

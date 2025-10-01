use anyhow::Result;

use redis::Client as RedisClient;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{info, warn, debug};

/// Configuration for performance tuning and connection pooling
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    // Database connection pool settings
    pub db_max_connections: u32,
    pub db_min_connections: u32,
    pub db_acquire_timeout_seconds: u64,
    pub db_idle_timeout_seconds: u64,
    pub db_max_lifetime_seconds: u64,
    
    // Redis connection pool settings
    pub redis_max_connections: u32,
    pub redis_min_connections: u32,
    pub redis_acquire_timeout_seconds: u64,
    
    // Concurrency control limits
    pub max_concurrent_qr_detections: usize,
    pub max_concurrent_ocr_processing: usize,
    pub max_concurrent_webhook_processing: usize,
    pub max_concurrent_api_requests: usize,
    
    // Performance monitoring settings
    pub enable_cache_warming: bool,
    pub enable_connection_preallocation: bool,
    pub metrics_collection_interval_seconds: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            // Database defaults
            db_max_connections: 50,
            db_min_connections: 10,
            db_acquire_timeout_seconds: 30,
            db_idle_timeout_seconds: 600,
            db_max_lifetime_seconds: 1800,
            
            // Redis defaults
            redis_max_connections: 20,
            redis_min_connections: 5,
            redis_acquire_timeout_seconds: 10,
            
            // Concurrency defaults
            max_concurrent_qr_detections: 50,
            max_concurrent_ocr_processing: 20,
            max_concurrent_webhook_processing: 100,
            max_concurrent_api_requests: 200,
            
            // Performance defaults
            enable_cache_warming: true,
            enable_connection_preallocation: true,
            metrics_collection_interval_seconds: 60,
        }
    }
}

impl PerformanceConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            // Database settings
            db_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "50".to_string())
                .parse().unwrap_or(50),
            db_min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse().unwrap_or(10),
            db_acquire_timeout_seconds: env::var("DATABASE_ACQUIRE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse().unwrap_or(30),
            db_idle_timeout_seconds: env::var("DATABASE_IDLE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "600".to_string())
                .parse().unwrap_or(600),
            db_max_lifetime_seconds: env::var("DATABASE_MAX_LIFETIME_SECONDS")
                .unwrap_or_else(|_| "1800".to_string())
                .parse().unwrap_or(1800),
                
            // Redis settings
            redis_max_connections: env::var("REDIS_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse().unwrap_or(20),
            redis_min_connections: env::var("REDIS_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse().unwrap_or(5),
            redis_acquire_timeout_seconds: env::var("REDIS_ACQUIRE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "10".to_string())
                .parse().unwrap_or(10),
                
            // Concurrency settings
            max_concurrent_qr_detections: env::var("MAX_CONCURRENT_QR_DETECTIONS")
                .unwrap_or_else(|_| "50".to_string())
                .parse().unwrap_or(50),
            max_concurrent_ocr_processing: env::var("MAX_CONCURRENT_OCR_PROCESSING")
                .unwrap_or_else(|_| "20".to_string())
                .parse().unwrap_or(20),
            max_concurrent_webhook_processing: env::var("MAX_CONCURRENT_WEBHOOK_PROCESSING")
                .unwrap_or_else(|_| "100".to_string())
                .parse().unwrap_or(100),
            max_concurrent_api_requests: env::var("MAX_CONCURRENT_API_REQUESTS")
                .unwrap_or_else(|_| "200".to_string())
                .parse().unwrap_or(200),
                
            // Performance settings
            enable_cache_warming: env::var("ENABLE_CACHE_WARMING")
                .unwrap_or_else(|_| "true".to_string())
                .parse().unwrap_or(true),
            enable_connection_preallocation: env::var("ENABLE_CONNECTION_PREALLOCATION")
                .unwrap_or_else(|_| "true".to_string())
                .parse().unwrap_or(true),
            metrics_collection_interval_seconds: env::var("METRICS_COLLECTION_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "60".to_string())
                .parse().unwrap_or(60),
        }
    }
}

/// Domain-specific metrics for tracking performance per domain
#[derive(Debug, Clone, Serialize)]
pub struct DomainMetrics {
    pub active_permits: usize,
    pub max_permits: usize,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_latency_ms: f64,
    pub average_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    #[serde(skip)]
    pub last_request_time: Option<Instant>,
}

impl Default for DomainMetrics {
    fn default() -> Self {
        Self {
            active_permits: 0,
            max_permits: 0,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_latency_ms: 0.0,
            average_latency_ms: 0.0,
            min_latency_ms: f64::MAX,
            max_latency_ms: 0.0,
            last_request_time: None,
        }
    }
}

impl DomainMetrics {
    pub fn update_request(&mut self, latency_ms: f64, success: bool) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
        
        self.total_latency_ms += latency_ms;
        self.average_latency_ms = self.total_latency_ms / self.total_requests as f64;
        
        if latency_ms < self.min_latency_ms {
            self.min_latency_ms = latency_ms;
        }
        if latency_ms > self.max_latency_ms {
            self.max_latency_ms = latency_ms;
        }
        
        self.last_request_time = Some(Instant::now());
    }
}

/// Overall performance metrics
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub concurrent_requests: usize,
    pub uptime_seconds: u64,
    #[serde(skip)]
    pub start_time: Instant,
    
    // Domain-specific metrics
    pub qr_detection: DomainMetrics,
    pub ocr_processing: DomainMetrics,
    pub webhook_processing: DomainMetrics,
    pub api_requests: DomainMetrics,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_latency_ms: 0.0,
            concurrent_requests: 0,
            uptime_seconds: 0,
            start_time: Instant::now(),
            qr_detection: DomainMetrics::default(),
            ocr_processing: DomainMetrics::default(),
            webhook_processing: DomainMetrics::default(),
            api_requests: DomainMetrics::default(),
        }
    }
}

/// Cache statistics for monitoring cache performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub l1_size: usize,
    pub l2_connected: bool,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            l1_size: 0,
            l2_connected: false,
        }
    }
}

/// Combined cache statistics for all cache managers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllCacheStats {
    pub qr_cache: CacheStats,
    pub ocr_cache: CacheStats,
    pub user_session_cache: CacheStats,
}

impl Default for AllCacheStats {
    fn default() -> Self {
        Self {
            qr_cache: CacheStats::default(),
            ocr_cache: CacheStats::default(),
            user_session_cache: CacheStats::default(),
        }
    }
}

/// Advanced Performance Manager with connection pooling, concurrency control, and metrics
pub struct PerformanceManager {
    config: PerformanceConfig,
    
    // Semaphores for concurrency control
    qr_detection_semaphore: Arc<Semaphore>,
    ocr_processing_semaphore: Arc<Semaphore>,
    webhook_processing_semaphore: Arc<Semaphore>,
    api_requests_semaphore: Arc<Semaphore>,
    
    // Performance metrics
    metrics: Arc<tokio::sync::Mutex<PerformanceMetrics>>,
    
    // Cache managers (will be integrated with the cache system)
    cache_stats: Arc<tokio::sync::Mutex<AllCacheStats>>,
}

impl PerformanceManager {
    /// Create a new PerformanceManager with configuration
    pub fn new(config: PerformanceConfig) -> Self {
        info!("🚀 Initializing PerformanceManager with advanced configuration");
        info!("📊 QR Detection: {} max concurrent", config.max_concurrent_qr_detections);
        info!("📊 OCR Processing: {} max concurrent", config.max_concurrent_ocr_processing);
        info!("📊 Webhook Processing: {} max concurrent", config.max_concurrent_webhook_processing);
        info!("📊 API Requests: {} max concurrent", config.max_concurrent_api_requests);
        
        let mut metrics = PerformanceMetrics::default();
        metrics.qr_detection.max_permits = config.max_concurrent_qr_detections;
        metrics.ocr_processing.max_permits = config.max_concurrent_ocr_processing;
        metrics.webhook_processing.max_permits = config.max_concurrent_webhook_processing;
        metrics.api_requests.max_permits = config.max_concurrent_api_requests;
        
        Self {
            qr_detection_semaphore: Arc::new(Semaphore::new(config.max_concurrent_qr_detections)),
            ocr_processing_semaphore: Arc::new(Semaphore::new(config.max_concurrent_ocr_processing)),
            webhook_processing_semaphore: Arc::new(Semaphore::new(config.max_concurrent_webhook_processing)),
            api_requests_semaphore: Arc::new(Semaphore::new(config.max_concurrent_api_requests)),
            metrics: Arc::new(tokio::sync::Mutex::new(metrics)),
            cache_stats: Arc::new(tokio::sync::Mutex::new(AllCacheStats::default())),
            config,
        }
    }
    
    /// Acquire a permit for QR detection with timing
    pub async fn acquire_qr_detection_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        let start = Instant::now();
        let permit = self.qr_detection_semaphore.acquire().await?;
        let latency = start.elapsed().as_millis() as f64;
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.qr_detection.active_permits = self.config.max_concurrent_qr_detections - self.qr_detection_semaphore.available_permits();
        
        debug!("🎯 QR detection permit acquired in {}ms", latency);
        Ok(permit)
    }
    
    /// Acquire a permit for OCR processing with timing
    pub async fn acquire_ocr_processing_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        let start = Instant::now();
        let permit = self.ocr_processing_semaphore.acquire().await?;
        let latency = start.elapsed().as_millis() as f64;
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.ocr_processing.active_permits = self.config.max_concurrent_ocr_processing - self.ocr_processing_semaphore.available_permits();
        
        debug!("📄 OCR processing permit acquired in {}ms", latency);
        Ok(permit)
    }
    
    /// Acquire a permit for webhook processing with timing
    pub async fn acquire_webhook_processing_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        let start = Instant::now();
        let permit = self.webhook_processing_semaphore.acquire().await?;
        let latency = start.elapsed().as_millis() as f64;
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.webhook_processing.active_permits = self.config.max_concurrent_webhook_processing - self.webhook_processing_semaphore.available_permits();
        
        debug!("🔗 Webhook processing permit acquired in {}ms", latency);
        Ok(permit)
    }
    
    /// Acquire a permit for API requests with timing
    pub async fn acquire_api_request_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        let start = Instant::now();
        let permit = self.api_requests_semaphore.acquire().await?;
        let latency = start.elapsed().as_millis() as f64;
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.api_requests.active_permits = self.config.max_concurrent_api_requests - self.api_requests_semaphore.available_permits();
        
        debug!("🌐 API request permit acquired in {}ms", latency);
        Ok(permit)
    }
    
    /// Record a completed request with timing and success status
    pub async fn record_request(&self, domain: &str, latency_ms: f64, success: bool) {
        let mut metrics = self.metrics.lock().await;
        
        // Update overall metrics
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
        
        // Update domain-specific metrics
        match domain {
            "qr_detection" => metrics.qr_detection.update_request(latency_ms, success),
            "ocr_processing" => metrics.ocr_processing.update_request(latency_ms, success),
            "webhook_processing" => metrics.webhook_processing.update_request(latency_ms, success),
            "api_requests" => metrics.api_requests.update_request(latency_ms, success),
            _ => warn!("Unknown domain for metrics: {}", domain),
        }
        
        // Update overall average latency
        let total_latency = metrics.qr_detection.total_latency_ms + 
                           metrics.ocr_processing.total_latency_ms + 
                           metrics.webhook_processing.total_latency_ms + 
                           metrics.api_requests.total_latency_ms;
        
        if metrics.total_requests > 0 {
            metrics.average_latency_ms = total_latency / metrics.total_requests as f64;
        }
        
        // Update uptime
        metrics.uptime_seconds = metrics.start_time.elapsed().as_secs();
        
        debug!("📊 Recorded {} request: {}ms, success: {}", domain, latency_ms, success);
    }
    
    /// Get current performance metrics
    pub async fn get_metrics(&self) -> Result<PerformanceMetrics> {
        let metrics = self.metrics.lock().await;
        Ok(metrics.clone())
    }
    
    /// Get current cache statistics
    pub async fn get_cache_stats(&self) -> Result<AllCacheStats> {
        let stats = self.cache_stats.lock().await;
        Ok(stats.clone())
    }
    
    /// Reset performance metrics (admin function)
    pub async fn reset_metrics(&self) -> Result<()> {
        let mut metrics = self.metrics.lock().await;
        *metrics = PerformanceMetrics::default();
        metrics.start_time = Instant::now();
        
        // Restore max permits
        metrics.qr_detection.max_permits = self.config.max_concurrent_qr_detections;
        metrics.ocr_processing.max_permits = self.config.max_concurrent_ocr_processing;
        metrics.webhook_processing.max_permits = self.config.max_concurrent_webhook_processing;
        metrics.api_requests.max_permits = self.config.max_concurrent_api_requests;
        
        info!("🔄 Performance metrics have been reset");
        Ok(())
    }
    
    /// Update cache statistics (called by cache managers)
    pub async fn update_cache_stats(&self, cache_type: &str, hits: u64, misses: u64, l1_size: usize, l2_connected: bool) {
        let mut stats = self.cache_stats.lock().await;
        
        let cache_stats = match cache_type {
            "qr" => &mut stats.qr_cache,
            "ocr" => &mut stats.ocr_cache,
            "user_session" => &mut stats.user_session_cache,
            _ => {
                warn!("Unknown cache type for stats update: {}", cache_type);
                return;
            }
        };
        
        cache_stats.hits = hits;
        cache_stats.misses = misses;
        cache_stats.l1_size = l1_size;
        cache_stats.l2_connected = l2_connected;
        
        let total_requests = hits + misses;
        cache_stats.hit_rate = if total_requests > 0 {
            (hits as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        debug!("📈 Updated {} cache stats: {}% hit rate", cache_type, cache_stats.hit_rate);
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &PerformanceConfig {
        &self.config
    }
    
    /// Warm up connections and caches (called during startup)
    pub async fn warm_up(&self, db_pool: &PgPool, redis_client: &RedisClient) -> Result<()> {
        if !self.config.enable_cache_warming {
            info!("🔥 Cache warming disabled by configuration");
            return Ok(());
        }
        
        info!("🔥 Starting performance warm-up process...");
        
        // Test database connection
        match sqlx::query("SELECT 1").fetch_one(db_pool).await {
            Ok(_) => info!("✅ Database connection verified"),
            Err(e) => warn!("⚠️ Database connection test failed: {}", e),
        }
        
        // Test Redis connection
        match redis_client.get_connection() {
            Ok(_) => info!("✅ Redis connection verified"),
            Err(e) => warn!("⚠️ Redis connection test failed: {}", e),
        }
        
        info!("🔥 Performance warm-up completed");
        Ok(())
    }
}

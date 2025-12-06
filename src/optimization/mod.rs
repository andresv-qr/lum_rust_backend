// Performance optimization utilities and configurations
use sqlx::{PgPool, postgres::PgPoolOptions};
use redis::Client as RedisClient;
use std::time::Duration;
use tracing::info;

/// Database connection pool optimization
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 5,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
        }
    }
}

impl DatabaseConfig {
    /// Production configuration optimized for 8 cores + SSD
    /// Formula: connections = (core_count * 2) + effective_spindle_count
    /// For SSD: spindle_count ≈ 0, so ~16-20 connections optimal per instance
    pub fn production() -> Self {
        Self {
            max_connections: 25,      // Optimal for 8 cores + SSD
            min_connections: 5,       // Keep warm connections ready
            acquire_timeout: Duration::from_secs(5),  // Fail fast instead of blocking
            idle_timeout: Duration::from_secs(300),   // 5 minutes - recycle idle connections
            max_lifetime: Duration::from_secs(3600),  // 1 hour - prevent stale connections
        }
    }

    pub fn development() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            acquire_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

/// Create optimized database pool
pub async fn create_optimized_db_pool(
    database_url: &str,
    config: DatabaseConfig,
) -> Result<PgPool, sqlx::Error> {
    info!("🔧 Creating optimized database pool with {} max connections", config.max_connections);
    
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        .test_before_acquire(true)
        .connect(database_url)
        .await?;

    info!("✅ Database pool created successfully");
    Ok(pool)
}

/// Redis connection optimization
pub struct RedisConfig {
    pub connection_timeout: Duration,
    pub response_timeout: Duration,
    pub retry_attempts: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(5),
            response_timeout: Duration::from_secs(3),
            retry_attempts: 3,
        }
    }
}

/// Create optimized Redis client
pub fn create_optimized_redis_client(
    redis_url: &str,
    _config: RedisConfig,
) -> Result<RedisClient, redis::RedisError> {
    info!("🔧 Creating optimized Redis client");
    
    let client = redis::Client::open(redis_url)?;
    
    info!("✅ Redis client created successfully");
    Ok(client)
}

/// Query optimization utilities
pub mod query_optimization {
    use sqlx::{PgPool, Row};
    use tracing::{info, warn};
    
    /// Analyze slow queries and suggest optimizations
    pub async fn analyze_slow_queries(pool: &PgPool) -> Result<Vec<SlowQuery>, sqlx::Error> {
        let rows = sqlx::query(r#"
            SELECT 
                query,
                calls,
                total_time,
                mean_time,
                rows
            FROM pg_stat_statements 
            WHERE mean_time > 100 
            ORDER BY mean_time DESC 
            LIMIT 10
        "#)
        .fetch_all(pool)
        .await?;

        let slow_queries: Vec<SlowQuery> = rows
            .into_iter()
            .map(|row| SlowQuery {
                query: row.get("query"),
                calls: row.get("calls"),
                total_time: row.get("total_time"),
                mean_time: row.get("mean_time"),
                rows: row.get("rows"),
            })
            .collect();

        if !slow_queries.is_empty() {
            warn!("🐌 Found {} slow queries", slow_queries.len());
        }

        Ok(slow_queries)
    }

    #[derive(Debug)]
    pub struct SlowQuery {
        pub query: String,
        pub calls: i64,
        pub total_time: f64,
        pub mean_time: f64,
        pub rows: i64,
    }

    /// Check for missing indexes
    pub async fn check_missing_indexes(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(r#"
            SELECT schemaname, tablename, attname, n_distinct, correlation
            FROM pg_stats
            WHERE schemaname = 'public'
            AND n_distinct > 100
            AND correlation < 0.1
        "#)
        .fetch_all(pool)
        .await?;

        let suggestions: Vec<String> = rows
            .into_iter()
            .map(|row| {
                format!(
                    "Consider index on {}.{} (n_distinct: {}, correlation: {})",
                    row.get::<String, _>("tablename"),
                    row.get::<String, _>("attname"),
                    row.get::<i64, _>("n_distinct"),
                    row.get::<f64, _>("correlation")
                )
            })
            .collect();

        if !suggestions.is_empty() {
            info!("💡 Found {} index suggestions", suggestions.len());
        }

        Ok(suggestions)
    }
}

/// Caching strategies
pub mod caching {
    use redis::AsyncCommands;
    use serde::{Serialize, Deserialize};
    use std::time::Duration;
    use tracing::{warn, error};

    #[derive(Debug, Clone)]
    pub struct CacheConfig {
        pub default_ttl: Duration,
        pub max_key_size: usize,
        pub max_value_size: usize,
    }

    impl Default for CacheConfig {
        fn default() -> Self {
            Self {
                default_ttl: Duration::from_secs(300), // 5 minutes
                max_key_size: 250,
                max_value_size: 1024 * 1024, // 1MB
            }
        }
    }

    pub struct CacheManager {
        config: CacheConfig,
    }

    impl CacheManager {
        pub fn new(config: CacheConfig) -> Self {
            Self { config }
        }

        pub async fn get<T>(&self, 
            redis_client: &redis::Client, 
            key: &str
        ) -> Result<Option<T>, redis::RedisError> 
        where 
            T: for<'de> Deserialize<'de>
        {
            if key.len() > self.config.max_key_size {
                warn!("🚫 Cache key too long: {}", key.len());
                return Ok(None);
            }

            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            
            match conn.get::<_, Option<String>>(key).await? {
                Some(value) => {
                    match serde_json::from_str(&value) {
                        Ok(data) => Ok(Some(data)),
                        Err(e) => {
                            error!("❌ Cache deserialization error: {}", e);
                            Ok(None)
                        }
                    }
                }
                None => Ok(None)
            }
        }

        pub async fn set<T>(&self,
            redis_client: &redis::Client,
            key: &str,
            value: &T,
            ttl: Option<Duration>
        ) -> Result<(), redis::RedisError>
        where
            T: Serialize
        {
            if key.len() > self.config.max_key_size {
                warn!("🚫 Cache key too long, skipping: {}", key.len());
                return Ok(());
            }

            let serialized = match serde_json::to_string(value) {
                Ok(s) => s,
                Err(e) => {
                    error!("❌ Cache serialization error: {}", e);
                    return Ok(());
                }
            };

            if serialized.len() > self.config.max_value_size {
                warn!("🚫 Cache value too large, skipping: {}", serialized.len());
                return Ok(());
            }

            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let ttl_seconds = ttl.unwrap_or(self.config.default_ttl).as_secs();
            
            conn.set_ex::<_, _, ()>(key, serialized, ttl_seconds).await?;
            Ok(())
        }

        pub async fn delete(&self,
            redis_client: &redis::Client,
            key: &str
        ) -> Result<(), redis::RedisError> {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            conn.del::<_, ()>(key).await?;
            Ok(())
        }

        pub async fn exists(&self,
            redis_client: &redis::Client,
            key: &str
        ) -> Result<bool, redis::RedisError> {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let exists: bool = conn.exists(key).await?;
            Ok(exists)
        }
    }

    /// Cache key generators
    pub fn user_profile_key(user_id: i32) -> String {
        format!("user:profile:{}", user_id)
    }

    pub fn user_balance_key(user_id: i32) -> String {
        format!("user:balance:{}", user_id)
    }

    pub fn invoice_details_key(invoice_id: i32) -> String {
        format!("invoice:details:{}", invoice_id)
    }

    pub fn cufe_processing_key(cufe: &str) -> String {
        format!("cufe:processing:{}", cufe)
    }
}

/// Memory optimization utilities
pub mod memory {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Memory usage tracker
    pub struct MemoryTracker;

    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for MemoryTracker {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc(layout);
            if !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            }
            ret
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        }
    }

    pub fn get_allocated_bytes() -> usize {
        ALLOCATED.load(Ordering::SeqCst)
    }

    pub fn get_allocated_mb() -> f64 {
        get_allocated_bytes() as f64 / 1024.0 / 1024.0
    }
}

/// Performance monitoring
pub mod performance {
    use std::time::{Duration, Instant};
    use tracing::{info, warn};

    pub struct PerformanceMonitor {
        start_time: Instant,
        operation: String,
    }

    impl PerformanceMonitor {
        pub fn new(operation: String) -> Self {
            Self {
                start_time: Instant::now(),
                operation,
            }
        }

        pub fn finish(self) -> Duration {
            let duration = self.start_time.elapsed();
            
            if duration > Duration::from_millis(1000) {
                warn!("🐌 Slow operation '{}' took {}ms", self.operation, duration.as_millis());
            } else if duration > Duration::from_millis(100) {
                info!("⚠️ Operation '{}' took {}ms", self.operation, duration.as_millis());
            }
            
            duration
        }
    }

    #[macro_export]
    macro_rules! monitor_performance {
        ($operation:expr, $code:block) => {{
            let _monitor = $crate::optimization::performance::PerformanceMonitor::new($operation.to_string());
            let result = $code;
            _monitor.finish();
            result
        }};
    }
}

use crate::models::user::User;
use crate::processing::qr_detection::QrScanResult;
use redis::Client as RedisClient;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, debug};
use anyhow::Result;
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use dashmap::DashMap;
use lru::LruCache;
use hex;

// Default TTL values (can be overridden by environment variables)
const DEFAULT_CACHE_TTL_SECONDS: u64 = 300; // 5 minutes
const QR_CACHE_TTL_SECONDS: u64 = 1800; // 30 minutes
const OCR_CACHE_TTL_SECONDS: u64 = 3600; // 1 hour
const USER_SESSION_CACHE_TTL_SECONDS: u64 = 900; // 15 minutes

// Cache size limits
const QR_CACHE_CAPACITY: usize = 5000;
const OCR_CACHE_CAPACITY: usize = 2000;
const USER_SESSION_CACHE_CAPACITY: usize = 10000;

/// Get cache TTL for general purposes.
fn get_cache_ttl() -> u64 {
    env::var("USER_CACHE_TTL_SECONDS")
        .map(|val| val.parse::<u64>().unwrap_or(DEFAULT_CACHE_TTL_SECONDS))
        .unwrap_or(DEFAULT_CACHE_TTL_SECONDS)
}

/// Get cache TTL specifically for user state in Redis.
pub fn get_user_state_cache_ttl() -> u64 {
    env::var("USER_STATE_CACHE_TTL_SECONDS")
        .map(|val| val.parse::<u64>().unwrap_or(900))
        .unwrap_or(900) // 15 minutes default
}

// ============================================================================
// CACHE ENTRY STRUCTURES
// ============================================================================

#[derive(Clone)]
struct CacheEntry {
    user: User,
    expiry: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedQrResult {
    content: String,
    decoder: String,
    timestamp: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CachedOcrResult {
    pub text: String,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub cached_at: u64, // timestamp
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CachedUserSession {
    pub user_id: i64,
    pub state: String,
    pub last_activity: u64,
    pub cached_at: u64,
}

// ============================================================================
// BASIC USER CACHE (LEGACY COMPATIBILITY)
// ============================================================================

#[derive(Clone, Default)]
pub struct UserCache {
    store: Arc<DashMap<String, CacheEntry>>,
}

impl UserCache {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<User> {
        if let Some(entry) = self.store.get(key) {
            if entry.expiry > Instant::now() {
                // Cache hit and entry is not expired
                return Some(entry.user.clone());
            } else {
                // Entry expired, remove it
                self.store.remove(key);
            }
        }
        // Cache miss or expired
        None
    }

    pub fn set(&self, key: String, user: User) {
        let ttl = get_cache_ttl();
        let expiry = Instant::now() + Duration::from_secs(ttl);
        let entry = CacheEntry { user, expiry };
        self.store.insert(key, entry);
    }
}

// ============================================================================
// ADVANCED CACHE MANAGER TRAIT
// ============================================================================

#[async_trait]
#[allow(async_fn_in_trait)]
pub trait AdvancedCacheManager {
    type Item;
    
    async fn get(&self, key: &str) -> Option<Self::Item>;
    async fn set(&self, key: &str, value: &Self::Item) -> Result<()>;
    async fn invalidate(&self, key: &str) -> Result<()>;
    async fn clear(&self) -> Result<()>;
    fn get_stats(&self) -> CacheStats;
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub l1_size: usize,
    pub l2_connected: bool,
}

// ============================================================================
// QR CACHE MANAGER
// ============================================================================

#[derive(Clone)]
pub struct QrCacheManager {
    l1_cache: Arc<Mutex<LruCache<String, CachedQrResult>>>,
    redis_client: RedisClient,
    stats: Arc<Mutex<CacheStats>>,
}

impl QrCacheManager {
    pub fn new(redis_client: RedisClient) -> Self {
        info!("🎯 Initializing QR Cache Manager with L1+L2 architecture");
        Self {
            l1_cache: Arc::new(Mutex::new(LruCache::new(QR_CACHE_CAPACITY.try_into().unwrap()))),
            redis_client,
            stats: Arc::new(Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                hit_rate: 0.0,
                l1_size: 0,
                l2_connected: true,
            })),
        }
    }
    
    pub async fn get_qr_result(&self, image_hash: &[u8]) -> Option<QrScanResult> {
        let key = format!("qr:{}", hex::encode(&image_hash[..16])); // Use first 16 bytes as key
        
        // Try L1 cache first
        let cached_result = {
            if let Ok(mut cache) = self.l1_cache.lock() {
                if let Some(cached) = cache.get(&key) {
                    debug!("🎯 QR cache L1 hit for key: {}", key);
                    let result = QrScanResult {
                        content: cached.content.clone(),
                        decoder: cached.decoder.clone(),
                        processing_time_ms: 0, // Not stored in this cache version
                        level_used: 0, // Not stored
                        preprocessing_applied: false, // Not stored in cache
                        rotation_angle: None, // Not stored in cache
                    };
                    let cache_len = cache.len();
                    drop(cache); // Release the lock before async call
                    self.update_stats(true, cache_len).await;
                    Some(result)
                } else {
                    None
                }
            } else {
                None
            }
        };
        
        if let Some(result) = cached_result {
            return Some(result);
        }
        
        // Try L2 (Redis) cache
        if let Ok(mut conn) = self.redis_client.get_connection() {
            if let Ok(cached_data) = redis::cmd("GET")
                .arg(&key)
                .query::<Vec<u8>>(&mut conn) {
                
                if let Ok(cached) = bincode::deserialize::<CachedQrResult>(&cached_data) {
                    debug!("🎯 QR cache L2 hit for key: {}", key);
                    
                    // Store in L1 for faster access
                    if let Ok(mut cache) = self.l1_cache.lock() {
                        cache.put(key, cached.clone());
                        self.update_stats(true, cache.len()).await;
                    }
                    
                    return Some(QrScanResult {
                        content: cached.content,
                        decoder: cached.decoder,
                        processing_time_ms: 0, // Not stored
                        level_used: 0, // Not stored
                        preprocessing_applied: false, // Not stored in cache
                        rotation_angle: None, // Not stored in cache
                    });
                }
            }
        }
        
        // Cache miss
        if let Ok(cache) = self.l1_cache.lock() {
            self.update_stats(false, cache.len()).await;
        }
        None
    }
    
    pub async fn cache_qr_result(&self, image_hash: &[u8], result: &QrScanResult) -> Result<()> {
        let key = format!("qr:{}", hex::encode(&image_hash[..16]));
        let cached_result = CachedQrResult {
            content: result.content.clone(),
            decoder: format!("{:?}", result.decoder),
            timestamp: Utc::now(),
        };
        
        // Store in L1
        if let Ok(mut cache) = self.l1_cache.lock() {
            cache.put(key.clone(), cached_result.clone());
        }
        
        // Store in L2 (Redis)
        if let Ok(serialized) = bincode::serialize(&cached_result) {
            if let Ok(mut conn) = self.redis_client.get_connection() {
                let _: () = redis::cmd("SETEX")
                    .arg(&key)
                    .arg(QR_CACHE_TTL_SECONDS)
                    .arg(serialized)
                    .query(&mut conn)?;
                
                debug!("🎯 QR result cached with key: {}", key);
            }
        }
        
        Ok(())
    }
    
    async fn update_stats(&self, hit: bool, l1_size: usize) {
        if let Ok(mut stats) = self.stats.lock() {
            if hit {
                stats.hits += 1;
            } else {
                stats.misses += 1;
            }
            stats.l1_size = l1_size;
            let total = stats.hits + stats.misses;
            stats.hit_rate = if total > 0 { stats.hits as f64 / total as f64 } else { 0.0 };
        }
    }
}

// ============================================================================
// OCR CACHE MANAGER
// ============================================================================

#[derive(Clone)]
pub struct OcrCacheManager {
    l1_cache: Arc<Mutex<LruCache<String, CachedOcrResult>>>,
    redis_client: RedisClient,
    stats: Arc<Mutex<CacheStats>>,
}

impl OcrCacheManager {
    pub fn new(redis_client: RedisClient) -> Self {
        info!("📄 Initializing OCR Cache Manager with L1+L2 architecture");
        Self {
            l1_cache: Arc::new(Mutex::new(LruCache::new(OCR_CACHE_CAPACITY.try_into().unwrap()))),
            redis_client,
            stats: Arc::new(Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                hit_rate: 0.0,
                l1_size: 0,
                l2_connected: true,
            })),
        }
    }
    
    pub async fn get_ocr_result(&self, image_hash: &[u8]) -> Option<String> {
        let key = format!("ocr:{}", hex::encode(&image_hash[..16]));
        
        // Try L1 cache first
        let cached_result = {
            if let Ok(mut cache) = self.l1_cache.lock() {
                if let Some(cached) = cache.get(&key) {
                    debug!("📄 OCR cache L1 hit for key: {}", key);
                    let result = cached.text.clone();
                    let cache_len = cache.len();
                    drop(cache); // Release the lock before async call
                    self.update_stats(true, cache_len).await;
                    Some(result)
                } else {
                    None
                }
            } else {
                None
            }
        };
        
        if let Some(result) = cached_result {
            return Some(result);
        }
        
        // Try L2 (Redis) cache
        if let Ok(mut conn) = self.redis_client.get_connection() {
            if let Ok(cached_data) = redis::cmd("GET")
                .arg(&key)
                .query::<Vec<u8>>(&mut conn) {
                
                if let Ok(cached) = bincode::deserialize::<CachedOcrResult>(&cached_data) {
                    debug!("📄 OCR cache L2 hit for key: {}", key);
                    
                    // Store in L1 for faster access
                    if let Ok(mut cache) = self.l1_cache.lock() {
                        cache.put(key, cached.clone());
                        self.update_stats(true, cache.len()).await;
                    }
                    
                    return Some(cached.text);
                }
            }
        }
        
        // Cache miss
        if let Ok(cache) = self.l1_cache.lock() {
            self.update_stats(false, cache.len()).await;
        }
        None
    }
    
    pub async fn cache_ocr_result(&self, image_hash: &[u8], text: &str) -> Result<()> {
        let key = format!("ocr:{}", hex::encode(&image_hash[..16]));
        let cached_result = CachedOcrResult {
            text: text.to_string(),
            confidence: 0.95, // Default confidence
            processing_time_ms: 0, // Would be set by caller
            cached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };
        
        // Store in L1
        if let Ok(mut cache) = self.l1_cache.lock() {
            cache.put(key.clone(), cached_result.clone());
        }
        
        // Store in L2 (Redis)
        if let Ok(serialized) = bincode::serialize(&cached_result) {
            if let Ok(mut conn) = self.redis_client.get_connection() {
                let _: () = redis::cmd("SETEX")
                    .arg(&key)
                    .arg(OCR_CACHE_TTL_SECONDS)
                    .arg(serialized)
                    .query(&mut conn)?;
                
                debug!("📄 OCR result cached with key: {}", key);
            }
        }
        
        Ok(())
    }
    
    async fn update_stats(&self, hit: bool, l1_size: usize) {
        if let Ok(mut stats) = self.stats.lock() {
            if hit {
                stats.hits += 1;
            } else {
                stats.misses += 1;
            }
            stats.l1_size = l1_size;
            let total = stats.hits + stats.misses;
            stats.hit_rate = if total > 0 { stats.hits as f64 / total as f64 } else { 0.0 };
        }
    }
}

// ============================================================================
// USER SESSION CACHE MANAGER
// ============================================================================

#[derive(Clone)]
pub struct UserSessionCacheManager {
    l1_cache: Arc<Mutex<LruCache<String, CachedUserSession>>>,
    redis_client: RedisClient,
    stats: Arc<Mutex<CacheStats>>,
}

impl UserSessionCacheManager {
    pub fn new(redis_client: RedisClient) -> Self {
        info!("👤 Initializing User Session Cache Manager with L1+L2 architecture");
        Self {
            l1_cache: Arc::new(Mutex::new(LruCache::new(USER_SESSION_CACHE_CAPACITY.try_into().unwrap()))),
            redis_client,
            stats: Arc::new(Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                hit_rate: 0.0,
                l1_size: 0,
                l2_connected: true,
            })),
        }
    }
    
    pub async fn get_user_session(&self, user_id: i64) -> Option<CachedUserSession> {
        let key = format!("session:{}", user_id);
        
        // Try L1 cache first
        let cached_result = {
            if let Ok(mut cache) = self.l1_cache.lock() {
                if let Some(cached) = cache.get(&key) {
                    debug!("👤 User session cache L1 hit for user: {}", user_id);
                    let result = cached.clone();
                    let cache_len = cache.len();
                    drop(cache); // Release the lock before async call
                    self.update_stats(true, cache_len).await;
                    Some(result)
                } else {
                    None
                }
            } else {
                None
            }
        };
        
        if let Some(result) = cached_result {
            return Some(result);
        }
        
        // Try L2 (Redis) cache
        if let Ok(mut conn) = self.redis_client.get_connection() {
            if let Ok(cached_data) = redis::cmd("GET")
                .arg(&key)
                .query::<Vec<u8>>(&mut conn) {
                
                if let Ok(cached) = bincode::deserialize::<CachedUserSession>(&cached_data) {
                    debug!("👤 User session cache L2 hit for user: {}", user_id);
                    
                    // Store in L1 for faster access
                    if let Ok(mut cache) = self.l1_cache.lock() {
                        cache.put(key, cached.clone());
                        self.update_stats(true, cache.len()).await;
                    }
                    
                    return Some(cached);
                }
            }
        }
        
        // Cache miss
        if let Ok(cache) = self.l1_cache.lock() {
            self.update_stats(false, cache.len()).await;
        }
        None
    }
    
    pub async fn cache_user_session(&self, session: &CachedUserSession) -> Result<()> {
        let key = format!("session:{}", session.user_id);
        
        // Store in L1
        if let Ok(mut cache) = self.l1_cache.lock() {
            cache.put(key.clone(), session.clone());
        }
        
        // Store in L2 (Redis)
        if let Ok(serialized) = bincode::serialize(session) {
            if let Ok(mut conn) = self.redis_client.get_connection() {
                let _: () = redis::cmd("SETEX")
                    .arg(&key)
                    .arg(USER_SESSION_CACHE_TTL_SECONDS)
                    .arg(serialized)
                    .query(&mut conn)?;
                
                debug!("👤 User session cached for user: {}", session.user_id);
            }
        }
        
        Ok(())
    }
    
    async fn update_stats(&self, hit: bool, l1_size: usize) {
        if let Ok(mut stats) = self.stats.lock() {
            if hit {
                stats.hits += 1;
            } else {
                stats.misses += 1;
            }
            stats.l1_size = l1_size;
            let total = stats.hits + stats.misses;
            stats.hit_rate = if total > 0 { stats.hits as f64 / total as f64 } else { 0.0 };
        }
    }
}

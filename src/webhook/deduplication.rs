use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{info, debug, warn};

/// High-performance message deduplication system
/// More efficient than Python's MemoryLimitedCache using DashMap + automatic cleanup
#[derive(Clone)]
pub struct MessageDeduplicator {
    cache: Arc<DashMap<String, Instant>>,
    ttl: Duration,
    max_entries: usize,
}

impl MessageDeduplicator {
    /// Create a new message deduplicator
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        let deduplicator = Self {
            cache: Arc::new(DashMap::new()),
            ttl,
            max_entries,
        };
        
        // Start background cleanup task
        let cleanup_cache = deduplicator.cache.clone();
        let cleanup_ttl = ttl;
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(60)); // Cleanup every minute
            loop {
                cleanup_interval.tick().await;
                Self::cleanup_expired_entries(&cleanup_cache, cleanup_ttl).await;
            }
        });
        
        info!("🔄 MessageDeduplicator initialized with TTL: {:?}, max_entries: {}", ttl, max_entries);
        deduplicator
    }
    
    /// Check if a message was already processed (and mark it if not)
    pub fn is_duplicate(&self, message_id: &str) -> bool {
        let now = Instant::now();
        
        // Check if message exists and is still valid
        if let Some(entry) = self.cache.get(message_id) {
            if now.duration_since(*entry) < self.ttl {
                debug!("⚠️ Duplicate message detected: {}", message_id);
                return true;
            } else {
                // Entry expired, remove it
                self.cache.remove(message_id);
            }
        }
        
        // Check cache size limit
        if self.cache.len() >= self.max_entries {
            warn!("🚫 Message cache full ({} entries), forcing cleanup", self.cache.len());
            let cache_clone = self.cache.clone();
            let ttl = self.ttl;
            tokio::spawn(async move {
                Self::cleanup_expired_entries(&cache_clone, ttl).await;
            });
        }
        
        // Mark message as processed
        self.cache.insert(message_id.to_string(), now);
        debug!("✅ Message marked as processed: {}", message_id);
        false
    }
    
    /// Get cache statistics for monitoring
    pub fn get_stats(&self) -> DeduplicationStats {
        let total_entries = self.cache.len();
        let now = Instant::now();
        
        // Count valid (non-expired) entries
        let valid_entries = self.cache
            .iter()
            .filter(|entry| now.duration_since(*entry.value()) < self.ttl)
            .count();
        
        DeduplicationStats {
            total_entries,
            valid_entries,
            expired_entries: total_entries - valid_entries,
            ttl_seconds: self.ttl.as_secs(),
            max_entries: self.max_entries,
            memory_usage_estimate_kb: total_entries * 64 / 1024, // Rough estimate
        }
    }
    
    /// Force cleanup of expired entries
    pub async fn force_cleanup(&self) {
        Self::cleanup_expired_entries(&self.cache, self.ttl).await;
    }
    
    /// Background cleanup task
    async fn cleanup_expired_entries(cache: &DashMap<String, Instant>, ttl: Duration) {
        let now = Instant::now();
        let initial_size = cache.len();
        
        // Collect expired keys
        let expired_keys: Vec<String> = cache
            .iter()
            .filter_map(|entry| {
                if now.duration_since(*entry.value()) >= ttl {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();
        
        // Remove expired entries
        for key in &expired_keys {
            cache.remove(key);
        }
        
        if !expired_keys.is_empty() {
            info!(
                "🧹 Cleaned up {} expired message entries (cache: {} -> {})",
                expired_keys.len(),
                initial_size,
                cache.len()
            );
        }
    }
}

/// Statistics for the deduplication system
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeduplicationStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
    pub ttl_seconds: u64,
    pub max_entries: usize,
    pub memory_usage_estimate_kb: usize,
}

impl Default for MessageDeduplicator {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(300), // 5 minutes TTL like Python
            10000, // 10k max entries
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_message_deduplication() {
        let deduplicator = MessageDeduplicator::new(Duration::from_millis(100), 1000);
        
        // First message should not be duplicate
        assert!(!deduplicator.is_duplicate("msg1"));
        
        // Same message should be duplicate
        assert!(deduplicator.is_duplicate("msg1"));
        
        // Different message should not be duplicate
        assert!(!deduplicator.is_duplicate("msg2"));
        
        // Wait for TTL to expire
        sleep(Duration::from_millis(150)).await;
        
        // After TTL, same message should not be duplicate anymore
        assert!(!deduplicator.is_duplicate("msg1"));
    }
    
    #[tokio::test]
    async fn test_cache_stats() {
        let deduplicator = MessageDeduplicator::new(Duration::from_secs(60), 100);
        
        // Add some messages
        deduplicator.is_duplicate("msg1");
        deduplicator.is_duplicate("msg2");
        deduplicator.is_duplicate("msg3");
        
        let stats = deduplicator.get_stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.valid_entries, 3);
        assert_eq!(stats.expired_entries, 0);
        assert_eq!(stats.ttl_seconds, 60);
    }
    
    #[tokio::test]
    async fn test_force_cleanup() {
        let deduplicator = MessageDeduplicator::new(Duration::from_millis(50), 100);
        
        // Add messages
        deduplicator.is_duplicate("msg1");
        deduplicator.is_duplicate("msg2");
        
        // Wait for expiration
        sleep(Duration::from_millis(100)).await;
        
        // Force cleanup
        deduplicator.force_cleanup().await;
        
        let stats = deduplicator.get_stats();
        assert_eq!(stats.total_entries, 0);
    }
}

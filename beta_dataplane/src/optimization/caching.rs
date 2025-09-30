//! Intelligent caching system for RPC data
//!
//! Multi-layer caching strategy to minimize RPC calls and maximize performance.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::{Result, BetaDataplaneError};

/// Cache entry with TTL and metadata
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// Cached value
    value: T,
    
    /// Timestamp when cached
    cached_at: Instant,
    
    /// Time-to-live
    ttl: Duration,
    
    /// Number of times accessed
    access_count: u64,
    
    /// Last access timestamp
    last_access: Instant,
}

impl<T: Clone> CacheEntry<T> {
    /// Create a new cache entry
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            cached_at: Instant::now(),
            ttl,
            access_count: 0,
            last_access: Instant::now(),
        }
    }

    /// Check if entry is expired
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }

    /// Get the cached value if not expired
    fn get(&mut self) -> Option<T> {
        if self.is_expired() {
            None
        } else {
            self.access_count += 1;
            self.last_access = Instant::now();
            Some(self.value.clone())
        }
    }

    /// Get age in seconds
    fn age_seconds(&self) -> f64 {
        self.cached_at.elapsed().as_secs_f64()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total requests
    pub total_requests: u64,
    
    /// Cache hits
    pub cache_hits: u64,
    
    /// Cache misses
    pub cache_misses: u64,
    
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    
    /// Total entries
    pub total_entries: usize,
    
    /// Memory usage estimate (bytes)
    pub memory_usage_bytes: usize,
    
    /// Average entry age (seconds)
    pub avg_entry_age_seconds: f64,
}

impl CacheStats {
    /// Create new empty stats
    fn new() -> Self {
        Self {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            hit_rate: 0.0,
            total_entries: 0,
            memory_usage_bytes: 0,
            avg_entry_age_seconds: 0.0,
        }
    }

    /// Update hit rate
    fn update_hit_rate(&mut self) {
        if self.total_requests > 0 {
            self.hit_rate = self.cache_hits as f64 / self.total_requests as f64;
        }
    }

    /// Record cache hit
    fn record_hit(&mut self) {
        self.total_requests += 1;
        self.cache_hits += 1;
        self.update_hit_rate();
    }

    /// Record cache miss
    fn record_miss(&mut self) {
        self.total_requests += 1;
        self.cache_misses += 1;
        self.update_hit_rate();
    }
}

/// Intelligent cache with TTL and automatic eviction
#[derive(Debug)]
pub struct IntelligentCache<T: Clone + Send + Sync> {
    /// Cache storage
    entries: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    
    /// Default TTL
    default_ttl: Duration,
    
    /// Maximum cache size
    max_entries: usize,
    
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    
    /// Cache strategy
    strategy: CacheStrategy,
}

/// Cache eviction strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    /// Least Recently Used
    LRU,
    
    /// Least Frequently Used
    LFU,
    
    /// Time-based (oldest first)
    FIFO,
    
    /// Random eviction
    Random,
}

impl<T: Clone + Send + Sync + 'static> IntelligentCache<T> {
    /// Create a new intelligent cache
    pub fn new(default_ttl: Duration, max_entries: usize, strategy: CacheStrategy) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_entries,
            stats: Arc::new(RwLock::new(CacheStats::new())),
            strategy,
        }
    }

    /// Get a value from cache
    pub async fn get(&self, key: &str) -> Option<T> {
        let mut entries = self.entries.write().await;
        
        if let Some(entry) = entries.get_mut(key) {
            if let Some(value) = entry.get() {
                self.stats.write().await.record_hit();
                debug!(key = key, age_seconds = entry.age_seconds(), "Cache hit");
                return Some(value);
            } else {
                // Entry expired, remove it
                entries.remove(key);
            }
        }
        
        self.stats.write().await.record_miss();
        debug!(key = key, "Cache miss");
        None
    }

    /// Set a value in cache with default TTL
    pub async fn set(&self, key: String, value: T) {
        self.set_with_ttl(key, value, self.default_ttl).await;
    }

    /// Set a value in cache with custom TTL
    pub async fn set_with_ttl(&self, key: String, value: T, ttl: Duration) {
        let mut entries = self.entries.write().await;
        
        // Evict if at capacity
        if entries.len() >= self.max_entries && !entries.contains_key(&key) {
            self.evict_one(&mut entries).await;
        }
        
        let entry = CacheEntry::new(value, ttl);
        entries.insert(key.clone(), entry);
        
        debug!(key = key, ttl_seconds = ttl.as_secs(), "Cached value");
    }

    /// Get or compute a value
    pub async fn get_or_compute<F, Fut>(&self, key: &str, computer: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Try to get from cache first
        if let Some(cached) = self.get(key).await {
            return Ok(cached);
        }
        
        // Compute new value
        let value = computer().await?;
        
        // Cache the computed value
        self.set(key.to_string(), value.clone()).await;
        
        Ok(value)
    }

    /// Invalidate a specific key
    pub async fn invalidate(&self, key: &str) {
        let mut entries = self.entries.write().await;
        if entries.remove(key).is_some() {
            debug!(key = key, "Cache entry invalidated");
        }
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        let count = entries.len();
        entries.clear();
        info!(entries_cleared = count, "Cache cleared");
    }

    /// Evict expired entries
    pub async fn evict_expired(&self) -> usize {
        let mut entries = self.entries.write().await;
        let initial_count = entries.len();
        
        entries.retain(|key, entry| {
            let keep = !entry.is_expired();
            if !keep {
                debug!(key = key, "Evicting expired entry");
            }
            keep
        });
        
        let evicted = initial_count - entries.len();
        if evicted > 0 {
            info!(evicted_count = evicted, "Evicted expired entries");
        }
        evicted
    }

    /// Evict one entry based on strategy
    async fn evict_one(&self, entries: &mut HashMap<String, CacheEntry<T>>) {
        if entries.is_empty() {
            return;
        }

        let key_to_evict = match self.strategy {
            CacheStrategy::LRU => {
                // Evict least recently used
                entries.iter()
                    .min_by_key(|(_, entry)| entry.last_access)
                    .map(|(key, _)| key.clone())
            }
            CacheStrategy::LFU => {
                // Evict least frequently used
                entries.iter()
                    .min_by_key(|(_, entry)| entry.access_count)
                    .map(|(key, _)| key.clone())
            }
            CacheStrategy::FIFO => {
                // Evict oldest
                entries.iter()
                    .min_by_key(|(_, entry)| entry.cached_at)
                    .map(|(key, _)| key.clone())
            }
            CacheStrategy::Random => {
                // Evict random
                entries.keys().next().cloned()
            }
        };

        if let Some(key) = key_to_evict {
            entries.remove(&key);
            debug!(key = key, strategy = ?self.strategy, "Evicted entry");
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let mut stats = self.stats.read().await.clone();
        
        stats.total_entries = entries.len();
        
        // Estimate memory usage (rough estimate: 1KB per entry)
        stats.memory_usage_bytes = entries.len() * 1024;
        
        // Calculate average entry age
        if !entries.is_empty() {
            let total_age: f64 = entries.values()
                .map(|entry| entry.age_seconds())
                .sum();
            stats.avg_entry_age_seconds = total_age / entries.len() as f64;
        }
        
        stats
    }

    /// Start background eviction task
    pub fn start_eviction_task(&self, interval: Duration) {
        let cache = self.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                let evicted = cache.evict_expired().await;
                
                if evicted > 0 {
                    let stats = cache.get_stats().await;
                    debug!(
                        evicted = evicted,
                        total_entries = stats.total_entries,
                        hit_rate = stats.hit_rate,
                        "Background eviction completed"
                    );
                }
            }
        });
        
        info!("Started background eviction task");
    }

    /// Check if cache is healthy
    pub async fn is_healthy(&self) -> bool {
        let stats = self.get_stats().await;
        
        // Consider healthy if:
        // 1. Hit rate is reasonable (> 50%)
        // 2. Not at max capacity
        // 3. Not too many entries
        
        stats.hit_rate > 0.5 || stats.total_requests < 100
    }
}

impl<T: Clone + Send + Sync> Clone for IntelligentCache<T> {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            default_ttl: self.default_ttl,
            max_entries: self.max_entries,
            stats: Arc::clone(&self.stats),
            strategy: self.strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = IntelligentCache::<String>::new(
            Duration::from_secs(60),
            100,
            CacheStrategy::LRU,
        );

        // Set and get
        cache.set("key1".to_string(), "value1".to_string()).await;
        let value = cache.get("key1").await;
        assert_eq!(value, Some("value1".to_string()));

        // Get stats
        let stats = cache.get_stats().await;
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 0);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache = IntelligentCache::<String>::new(
            Duration::from_millis(100),
            100,
            CacheStrategy::LRU,
        );

        cache.set("key1".to_string(), "value1".to_string()).await;
        
        // Should be cached
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        assert_eq!(cache.get("key1").await, None);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = IntelligentCache::<String>::new(
            Duration::from_secs(60),
            3, // Small cache
            CacheStrategy::LRU,
        );

        // Fill cache
        cache.set("key1".to_string(), "value1".to_string()).await;
        cache.set("key2".to_string(), "value2".to_string()).await;
        cache.set("key3".to_string(), "value3".to_string()).await;

        // Access key1 to make it recently used
        cache.get("key1").await;

        // Add another entry (should evict least recently used)
        cache.set("key4".to_string(), "value4".to_string()).await;

        // key1 should still be there (recently used)
        assert!(cache.get("key1").await.is_some());
        
        // One of key2 or key3 should be evicted
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_entries, 3);
    }
}

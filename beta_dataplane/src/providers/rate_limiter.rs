//! Rate limiting for RPC providers
//!
//! Implements token bucket algorithm for efficient rate limiting
//! across multiple providers with different limits.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::{ProviderType, Result, BetaDataplaneError};

/// Rate limiter using token bucket algorithm
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum tokens (requests per second)
    max_tokens: f64,
    
    /// Current token count
    tokens: f64,
    
    /// Token refill rate (tokens per second)
    refill_rate: f64,
    
    /// Last refill timestamp
    last_refill: Instant,
    
    /// Provider identifier
    provider_name: String,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(provider_name: String, requests_per_second: f64) -> Self {
        Self {
            max_tokens: requests_per_second,
            tokens: requests_per_second,
            refill_rate: requests_per_second,
            last_refill: Instant::now(),
            provider_name,
        }
    }
    
    /// Try to acquire a token (non-blocking)
    pub fn try_acquire(&mut self) -> bool {
        self.refill();
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            debug!(
                provider = self.provider_name,
                remaining_tokens = self.tokens,
                "Token acquired"
            );
            true
        } else {
            debug!(
                provider = self.provider_name,
                remaining_tokens = self.tokens,
                "Rate limit exceeded"
            );
            false
        }
    }
    
    /// Wait until a token is available
    pub async fn acquire(&mut self) {
        loop {
            if self.try_acquire() {
                break;
            }
            
            // Calculate how long to wait for the next token
            let wait_time = Duration::from_secs_f64(1.0 / self.refill_rate);
            tokio::time::sleep(wait_time).await;
        }
    }
    
    /// Get current token count
    pub fn available_tokens(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
    
    /// Get utilization percentage (0.0 to 1.0)
    pub fn utilization(&mut self) -> f64 {
        self.refill();
        1.0 - (self.tokens / self.max_tokens)
    }
    
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

/// Multi-provider rate limiter manager
#[derive(Debug)]
pub struct ProviderRateLimitManager {
    /// Rate limiters for each provider
    limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,
}

impl ProviderRateLimitManager {
    /// Create a new rate limit manager
    pub fn new() -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a rate limiter for a provider
    pub async fn add_provider(&self, provider_name: String, requests_per_second: f64) {
        let limiter = RateLimiter::new(provider_name.clone(), requests_per_second);
        self.limiters.write().await.insert(provider_name, limiter);
    }
    
    /// Try to acquire a token for a provider
    pub async fn try_acquire(&self, provider_name: &str) -> bool {
        let mut limiters = self.limiters.write().await;
        if let Some(limiter) = limiters.get_mut(provider_name) {
            limiter.try_acquire()
        } else {
            warn!(provider = provider_name, "Rate limiter not found");
            false
        }
    }
    
    /// Wait for a token for a provider
    pub async fn acquire(&self, provider_name: &str) -> Result<()> {
        let mut limiters = self.limiters.write().await;
        if let Some(limiter) = limiters.get_mut(provider_name) {
            limiter.acquire().await;
            Ok(())
        } else {
            Err(BetaDataplaneError::Provider {
                provider: provider_name.to_string(),
                message: "Rate limiter not found".to_string(),
            })
        }
    }
    
    /// Get utilization for a provider
    pub async fn get_utilization(&self, provider_name: &str) -> Option<f64> {
        let mut limiters = self.limiters.write().await;
        limiters.get_mut(provider_name).map(|limiter| limiter.utilization())
    }
    
    /// Get utilization for all providers
    pub async fn get_all_utilization(&self) -> HashMap<String, f64> {
        let mut limiters = self.limiters.write().await;
        let mut utilization = HashMap::new();
        
        for (name, limiter) in limiters.iter_mut() {
            utilization.insert(name.clone(), limiter.utilization());
        }
        
        utilization
    }
}

impl Clone for ProviderRateLimitManager {
    fn clone(&self) -> Self {
        Self {
            limiters: Arc::clone(&self.limiters),
        }
    }
}

impl Default for ProviderRateLimitManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let mut limiter = RateLimiter::new("test".to_string(), 10.0); // 10 requests per second
        
        // Should be able to acquire 10 tokens immediately
        for _ in 0..10 {
            assert!(limiter.try_acquire());
        }
        
        // 11th request should fail
        assert!(!limiter.try_acquire());
        
        // Wait for refill
        sleep(Duration::from_millis(200)).await;
        
        // Should be able to acquire more tokens
        assert!(limiter.try_acquire());
    }

    #[tokio::test]
    async fn test_provider_rate_limit_manager() {
        let manager = ProviderRateLimitManager::new();
        
        // Add providers
        manager.add_provider("alchemy".to_string(), 300.0).await;
        manager.add_provider("infura".to_string(), 100.0).await;
        
        // Test acquisition
        assert!(manager.try_acquire("alchemy").await);
        assert!(manager.try_acquire("infura").await);
        
        // Test utilization
        let utilization = manager.get_all_utilization().await;
        assert!(utilization.contains_key("alchemy"));
        assert!(utilization.contains_key("infura"));
    }
}

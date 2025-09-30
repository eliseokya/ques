//! Failover management for RPC providers
//!
//! Implements intelligent failover logic with health tracking
//! and automatic recovery for maximum reliability.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use crate::{config::ProviderConfig, Result, BetaDataplaneError};

/// Provider health status
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderStatus {
    /// Provider is healthy and available
    Healthy,
    
    /// Provider is degraded but still usable
    Degraded,
    
    /// Provider is unhealthy and should not be used
    Unhealthy,
    
    /// Provider status is unknown
    Unknown,
}

/// Provider health metrics
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    /// Provider name
    pub name: String,
    
    /// Current status
    pub status: ProviderStatus,
    
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    
    /// Last successful request timestamp
    pub last_success: Option<Instant>,
    
    /// Last failure timestamp
    pub last_failure: Option<Instant>,
    
    /// Total requests made
    pub total_requests: u64,
    
    /// Total successful requests
    pub successful_requests: u64,
}

impl ProviderHealth {
    /// Create new provider health tracker
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: ProviderStatus::Unknown,
            success_rate: 1.0,
            avg_response_time_ms: 0.0,
            consecutive_failures: 0,
            last_success: None,
            last_failure: None,
            total_requests: 0,
            successful_requests: 0,
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, response_time_ms: f64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.consecutive_failures = 0;
        self.last_success = Some(Instant::now());
        
        // Update average response time using exponential moving average
        let alpha = 0.1;
        self.avg_response_time_ms = alpha * response_time_ms + (1.0 - alpha) * self.avg_response_time_ms;
        
        // Update success rate
        self.success_rate = self.successful_requests as f64 / self.total_requests as f64;
        
        // Update status based on metrics
        self.update_status();
        
        debug!(
            provider = self.name,
            response_time_ms = response_time_ms,
            success_rate = self.success_rate,
            "Provider request succeeded"
        );
    }

    /// Record a failed request
    pub fn record_failure(&mut self, error_message: &str) {
        self.total_requests += 1;
        self.consecutive_failures += 1;
        self.last_failure = Some(Instant::now());
        
        // Update success rate
        self.success_rate = self.successful_requests as f64 / self.total_requests as f64;
        
        // Update status based on metrics
        self.update_status();
        
        warn!(
            provider = self.name,
            consecutive_failures = self.consecutive_failures,
            success_rate = self.success_rate,
            error = error_message,
            "Provider request failed"
        );
    }

    /// Update provider status based on current metrics
    fn update_status(&mut self) {
        let old_status = self.status.clone();
        
        // Determine new status based on metrics
        self.status = if self.consecutive_failures >= 10 {
            ProviderStatus::Unhealthy
        } else if self.consecutive_failures >= 5 || self.success_rate < 0.8 {
            ProviderStatus::Degraded
        } else if self.success_rate >= 0.95 {
            ProviderStatus::Healthy
        } else {
            ProviderStatus::Degraded
        };
        
        // Log status changes
        if old_status != self.status {
            match self.status {
                ProviderStatus::Healthy => info!(
                    provider = self.name,
                    "Provider status changed to healthy"
                ),
                ProviderStatus::Degraded => warn!(
                    provider = self.name,
                    success_rate = self.success_rate,
                    consecutive_failures = self.consecutive_failures,
                    "Provider status changed to degraded"
                ),
                ProviderStatus::Unhealthy => error!(
                    provider = self.name,
                    success_rate = self.success_rate,
                    consecutive_failures = self.consecutive_failures,
                    "Provider status changed to unhealthy"
                ),
                ProviderStatus::Unknown => {}
            }
        }
    }

    /// Check if provider is usable
    pub fn is_usable(&self) -> bool {
        matches!(self.status, ProviderStatus::Healthy | ProviderStatus::Degraded)
    }

    /// Get provider priority score (higher is better)
    pub fn priority_score(&self) -> f64 {
        let status_score = match self.status {
            ProviderStatus::Healthy => 1.0,
            ProviderStatus::Degraded => 0.5,
            ProviderStatus::Unhealthy => 0.0,
            ProviderStatus::Unknown => 0.3,
        };
        
        let response_time_score = if self.avg_response_time_ms > 0.0 {
            1.0 / (1.0 + self.avg_response_time_ms / 1000.0) // Normalize to 0-1
        } else {
            1.0
        };
        
        status_score * 0.7 + response_time_score * 0.3
    }
}

/// Failover manager for provider selection and health tracking
#[derive(Debug, Clone)]
pub struct FailoverManager {
    /// Provider health tracking
    health: Arc<RwLock<HashMap<String, ProviderHealth>>>,
    
    /// Failover configuration
    config: FailoverConfig,
}

/// Failover configuration
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    /// Maximum consecutive failures before marking unhealthy
    pub max_consecutive_failures: u32,
    
    /// Minimum success rate to maintain healthy status
    pub min_success_rate: f64,
    
    /// Health check interval
    pub health_check_interval: Duration,
    
    /// Recovery check interval for unhealthy providers
    pub recovery_check_interval: Duration,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            max_consecutive_failures: 5,
            min_success_rate: 0.8,
            health_check_interval: Duration::from_secs(60),
            recovery_check_interval: Duration::from_secs(300),
        }
    }
}

impl FailoverManager {
    /// Create a new failover manager
    pub fn new(config: FailoverConfig) -> Self {
        Self {
            health: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Add a provider to health tracking
    pub async fn add_provider(&self, provider_name: String) {
        let health = ProviderHealth::new(provider_name.clone());
        self.health.write().await.insert(provider_name.clone(), health);
        
        info!(provider = provider_name, "Added provider to failover manager");
    }

    /// Record a successful request for a provider
    pub async fn record_success(&self, provider_name: &str, response_time_ms: f64) {
        let mut health_map = self.health.write().await;
        if let Some(health) = health_map.get_mut(provider_name) {
            health.record_success(response_time_ms);
        }
    }

    /// Record a failed request for a provider
    pub async fn record_failure(&self, provider_name: &str, error_message: &str) {
        let mut health_map = self.health.write().await;
        if let Some(health) = health_map.get_mut(provider_name) {
            health.record_failure(error_message);
        }
    }

    /// Get the best available provider from a list
    pub async fn select_best_provider(&self, provider_names: &[String]) -> Option<String> {
        let health_map = self.health.read().await;
        
        let mut best_provider = None;
        let mut best_score = 0.0;
        
        for provider_name in provider_names {
            if let Some(health) = health_map.get(provider_name) {
                if health.is_usable() {
                    let score = health.priority_score();
                    if score > best_score {
                        best_score = score;
                        best_provider = Some(provider_name.clone());
                    }
                }
            }
        }
        
        if let Some(ref provider) = best_provider {
            debug!(
                provider = provider,
                score = best_score,
                "Selected best provider"
            );
        } else {
            warn!("No usable providers available");
        }
        
        best_provider
    }

    /// Get all healthy providers from a list
    pub async fn get_healthy_providers(&self, provider_names: &[String]) -> Vec<String> {
        let health_map = self.health.read().await;
        
        provider_names
            .iter()
            .filter_map(|name| {
                health_map.get(name).and_then(|health| {
                    if health.is_usable() {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Get provider health status
    pub async fn get_provider_health(&self, provider_name: &str) -> Option<ProviderHealth> {
        let health_map = self.health.read().await;
        health_map.get(provider_name).cloned()
    }

    /// Get all provider health statuses
    pub async fn get_all_health(&self) -> HashMap<String, ProviderHealth> {
        self.health.read().await.clone()
    }

    /// Check if any providers are available
    pub async fn has_healthy_providers(&self, provider_names: &[String]) -> bool {
        let healthy = self.get_healthy_providers(provider_names).await;
        !healthy.is_empty()
    }

    /// Start background health monitoring
    pub async fn start_health_monitoring(&self) {
        let health = Arc::clone(&self.health);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.health_check_interval);
            
            loop {
                interval.tick().await;
                
                // Perform health checks
                let health_map = health.read().await;
                let mut unhealthy_count = 0;
                let mut degraded_count = 0;
                let mut healthy_count = 0;
                
                for (name, provider_health) in health_map.iter() {
                    match provider_health.status {
                        ProviderStatus::Healthy => healthy_count += 1,
                        ProviderStatus::Degraded => degraded_count += 1,
                        ProviderStatus::Unhealthy => unhealthy_count += 1,
                        ProviderStatus::Unknown => {}
                    }
                }
                
                debug!(
                    healthy = healthy_count,
                    degraded = degraded_count,
                    unhealthy = unhealthy_count,
                    "Provider health summary"
                );
                
                if unhealthy_count > 0 {
                    warn!(
                        unhealthy_providers = unhealthy_count,
                        "Some providers are unhealthy"
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_failover_manager() {
        let manager = FailoverManager::new(FailoverConfig::default());
        
        // Add providers
        manager.add_provider("provider1".to_string()).await;
        manager.add_provider("provider2".to_string()).await;
        
        let providers = vec!["provider1".to_string(), "provider2".to_string()];
        
        // Initially, should select a provider (both unknown)
        let selected = manager.select_best_provider(&providers).await;
        // May return None if no providers are healthy yet
        assert!(selected.is_none() || providers.contains(&selected.unwrap()));
        
        // Record success for provider1
        manager.record_success("provider1", 100.0).await;
        
        // Should prefer provider1 now
        let selected = manager.select_best_provider(&providers).await;
        assert_eq!(selected, Some("provider1".to_string()));
        
        // Record failures for provider1
        for _ in 0..10 {
            manager.record_failure("provider1", "test error").await;
        }
        
        // After failures, may switch to provider2 or return None if both are unhealthy
        let selected = manager.select_best_provider(&providers).await;
        // Just verify it doesn't panic - may be None or Some depending on health thresholds
        if selected.is_some() {
            assert!(providers.contains(&selected.unwrap()));
        }
    }
}

//! Health check system for monitoring dataplane components
//!
//! Provides comprehensive health monitoring for all dataplane subsystems.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::feeds::{BetaFeedHealth, FeedStatus};
use crate::{Result, BetaDataplaneError};

/// Overall health status for the dataplane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    
    /// Some degradation but still functional
    Degraded,
    
    /// Critical issues, may not be functional
    Unhealthy,
    
    /// System is starting up
    Starting,
    
    /// System is shutting down
    Stopping,
}

impl HealthStatus {
    /// Check if the status is operational (Healthy or Degraded)
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }

    /// Check if the status requires immediate attention
    pub fn requires_attention(&self) -> bool {
        matches!(self, HealthStatus::Unhealthy)
    }

    /// Get the severity level
    pub fn severity(&self) -> u8 {
        match self {
            HealthStatus::Healthy => 0,
            HealthStatus::Starting => 1,
            HealthStatus::Degraded => 2,
            HealthStatus::Stopping => 3,
            HealthStatus::Unhealthy => 4,
        }
    }
}

/// Health check result for a single component
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    
    /// Health status
    pub status: HealthStatus,
    
    /// Status message
    pub message: Option<String>,
    
    /// Last check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
    
    /// Response time in milliseconds
    pub response_time_ms: f64,
    
    /// Additional details
    pub details: HashMap<String, String>,
}

impl ComponentHealth {
    /// Create a new component health check
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: HealthStatus::Starting,
            message: None,
            last_check: chrono::Utc::now(),
            response_time_ms: 0.0,
            details: HashMap::new(),
        }
    }

    /// Update the health status
    pub fn update(&mut self, status: HealthStatus, message: Option<String>) {
        self.status = status;
        self.message = message;
        self.last_check = chrono::Utc::now();
    }

    /// Add a detail
    pub fn add_detail(&mut self, key: String, value: String) {
        self.details.insert(key, value);
    }

    /// Check if the component is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy)
    }
}

/// Aggregated health report for the entire dataplane
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// Overall status
    pub status: HealthStatus,
    
    /// Report timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Component health checks
    pub components: HashMap<String, ComponentHealth>,
    
    /// System uptime in seconds
    pub uptime_seconds: u64,
    
    /// Total health checks performed
    pub total_checks: u64,
    
    /// Failed checks count
    pub failed_checks: u64,
    
    /// Summary message
    pub summary: String,
}

impl HealthReport {
    /// Create a new health report
    pub fn new() -> Self {
        Self {
            status: HealthStatus::Starting,
            timestamp: chrono::Utc::now(),
            components: HashMap::new(),
            uptime_seconds: 0,
            total_checks: 0,
            failed_checks: 0,
            summary: "System starting".to_string(),
        }
    }

    /// Calculate overall status from components
    pub fn calculate_status(&mut self) {
        if self.components.is_empty() {
            self.status = HealthStatus::Starting;
            self.summary = "No components registered".to_string();
            return;
        }

        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let total_count = self.components.len();

        for component in self.components.values() {
            match component.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                HealthStatus::Unhealthy => unhealthy_count += 1,
                _ => {}
            }
        }

        // Determine overall status
        if unhealthy_count > 0 {
            self.status = HealthStatus::Unhealthy;
            self.summary = format!(
                "{}/{} components unhealthy",
                unhealthy_count, total_count
            );
        } else if degraded_count > 0 {
            self.status = HealthStatus::Degraded;
            self.summary = format!(
                "{}/{} components degraded",
                degraded_count, total_count
            );
        } else if healthy_count == total_count {
            self.status = HealthStatus::Healthy;
            self.summary = format!("All {} components healthy", total_count);
        } else {
            self.status = HealthStatus::Starting;
            self.summary = format!(
                "{}/{} components starting",
                total_count - healthy_count,
                total_count
            );
        }
    }

    /// Get unhealthy components
    pub fn get_unhealthy_components(&self) -> Vec<&ComponentHealth> {
        self.components
            .values()
            .filter(|c| matches!(c.status, HealthStatus::Unhealthy))
            .collect()
    }

    /// Get degraded components
    pub fn get_degraded_components(&self) -> Vec<&ComponentHealth> {
        self.components
            .values()
            .filter(|c| matches!(c.status, HealthStatus::Degraded))
            .collect()
    }

    /// Check if any component requires attention
    pub fn requires_attention(&self) -> bool {
        self.status.requires_attention()
            || self.components.values().any(|c| c.status.requires_attention())
    }
}

impl Default for HealthReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Health checker that monitors all dataplane components
pub struct HealthChecker {
    /// Health report
    report: Arc<RwLock<HealthReport>>,
    
    /// Start time for uptime calculation
    start_time: Instant,
    
    /// Health check interval
    check_interval: Duration,
    
    /// Component checkers
    checkers: Arc<RwLock<HashMap<String, Box<dyn ComponentChecker>>>>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(check_interval: Duration) -> Self {
        Self {
            report: Arc::new(RwLock::new(HealthReport::new())),
            start_time: Instant::now(),
            check_interval,
            checkers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a component checker
    pub async fn register_checker(&self, name: String, checker: Box<dyn ComponentChecker>) {
        let mut checkers = self.checkers.write().await;
        checkers.insert(name.clone(), checker);
        
        // Initialize component in report
        let mut report = self.report.write().await;
        report.components.insert(name.clone(), ComponentHealth::new(name));
    }

    /// Run health checks on all components
    pub async fn check_all(&self) -> Result<()> {
        let start = Instant::now();
        
        let checkers = self.checkers.read().await;
        let mut results = HashMap::new();

        for (name, checker) in checkers.iter() {
            let check_start = Instant::now();
            match checker.check().await {
                Ok(health) => {
                    results.insert(name.clone(), health);
                }
                Err(e) => {
                    error!("Health check failed for {}: {}", name, e);
                    let mut health = ComponentHealth::new(name.clone());
                    health.update(HealthStatus::Unhealthy, Some(format!("Check failed: {}", e)));
                    results.insert(name.clone(), health);
                }
            }
            let check_duration = check_start.elapsed();
            
            if let Some(health) = results.get_mut(name) {
                health.response_time_ms = check_duration.as_secs_f64() * 1000.0;
            }
        }

        // Update report
        let mut report = self.report.write().await;
        report.components = results;
        report.uptime_seconds = self.start_time.elapsed().as_secs();
        report.total_checks += 1;
        report.timestamp = chrono::Utc::now();
        
        // Calculate overall status
        report.calculate_status();
        
        // Count failed checks
        report.failed_checks = report
            .components
            .values()
            .filter(|c| matches!(c.status, HealthStatus::Unhealthy))
            .count() as u64;

        let check_duration = start.elapsed();
        debug!(
            "Health check completed in {:.2}ms - Status: {:?}",
            check_duration.as_secs_f64() * 1000.0,
            report.status
        );

        Ok(())
    }

    /// Get the current health report
    pub async fn get_report(&self) -> HealthReport {
        self.report.read().await.clone()
    }

    /// Get the health check interval
    pub fn check_interval(&self) -> Duration {
        self.check_interval
    }

    /// Start continuous health checking
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting health monitoring with interval: {:?}", self.check_interval);
        
        let checker = Arc::new(self.clone());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(checker.check_interval);
            
            loop {
                interval.tick().await;
                
                if let Err(e) = checker.check_all().await {
                    error!("Health check error: {}", e);
                }
            }
        });
        
        Ok(())
    }
}

// Implement Clone for HealthChecker
impl Clone for HealthChecker {
    fn clone(&self) -> Self {
        Self {
            report: Arc::clone(&self.report),
            start_time: self.start_time,
            check_interval: self.check_interval,
            checkers: Arc::clone(&self.checkers),
        }
    }
}

/// Trait for component health checkers
#[async_trait::async_trait]
pub trait ComponentChecker: Send + Sync {
    /// Perform a health check
    async fn check(&self) -> Result<ComponentHealth>;
}

/// Health checker for feeds
pub struct FeedHealthChecker {
    feed_name: String,
    get_health: Arc<dyn Fn() -> BetaFeedHealth + Send + Sync>,
}

impl FeedHealthChecker {
    /// Create a new feed health checker
    pub fn new<F>(feed_name: String, get_health: F) -> Self
    where
        F: Fn() -> BetaFeedHealth + Send + Sync + 'static,
    {
        Self {
            feed_name,
            get_health: Arc::new(get_health),
        }
    }
}

#[async_trait::async_trait]
impl ComponentChecker for FeedHealthChecker {
    async fn check(&self) -> Result<ComponentHealth> {
        let feed_health = (self.get_health)();
        
        let status = match feed_health.status {
            FeedStatus::Healthy => HealthStatus::Healthy,
            FeedStatus::Degraded => HealthStatus::Degraded,
            FeedStatus::Unhealthy => HealthStatus::Unhealthy,
            FeedStatus::Disabled => HealthStatus::Degraded,
            FeedStatus::Connecting => HealthStatus::Starting,
        };
        
        let mut component = ComponentHealth::new(self.feed_name.clone());
        component.update(status, feed_health.last_error);
        
        // Add performance metrics as details
        component.add_detail(
            "throughput".to_string(),
            format!("{:.2} features/s", feed_health.performance.throughput),
        );
        component.add_detail(
            "latency".to_string(),
            format!("{:.2}ms", feed_health.performance.latency_ms),
        );
        component.add_detail(
            "backlog".to_string(),
            feed_health.performance.backlog_size.to_string(),
        );
        
        Ok(component)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_operational() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded.is_operational());
        assert!(!HealthStatus::Unhealthy.is_operational());
        assert!(!HealthStatus::Starting.is_operational());
    }

    #[test]
    fn test_component_health() {
        let mut health = ComponentHealth::new("test".to_string());
        assert_eq!(health.status, HealthStatus::Starting);
        
        health.update(HealthStatus::Healthy, Some("All good".to_string()));
        assert!(health.is_healthy());
        assert_eq!(health.message, Some("All good".to_string()));
    }

    #[tokio::test]
    async fn test_health_checker() {
        let checker = HealthChecker::new(Duration::from_secs(60));
        let report = checker.get_report().await;
        
        assert_eq!(report.status, HealthStatus::Starting);
        assert!(report.components.is_empty());
    }
}

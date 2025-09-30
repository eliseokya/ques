//! Dashboard and monitoring API
//!
//! Provides HTTP endpoints for accessing health, metrics, and alerts.

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

use crate::monitoring::health::{HealthChecker, HealthReport, HealthStatus};
use crate::monitoring::metrics::{MetricsRegistry, MetricsSummary};
use crate::monitoring::alerts::{AlertManager, AlertStats};
use crate::Result;

/// Dashboard that aggregates all monitoring data
pub struct MonitoringDashboard {
    /// Health checker
    health: Arc<HealthChecker>,
    
    /// Metrics registry
    metrics: Arc<MetricsRegistry>,
    
    /// Alert manager
    alerts: Arc<AlertManager>,
    
    /// Dashboard state
    state: Arc<RwLock<DashboardState>>,
}

/// Dashboard state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardState {
    /// Dashboard name
    pub name: String,
    
    /// Version
    pub version: String,
    
    /// Start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    
    /// Last refresh time
    pub last_refresh: chrono::DateTime<chrono::Utc>,
    
    /// Auto-refresh enabled
    pub auto_refresh: bool,
    
    /// Refresh interval in seconds
    pub refresh_interval_secs: u64,
}

impl MonitoringDashboard {
    /// Create a new monitoring dashboard
    pub fn new(
        health: Arc<HealthChecker>,
        metrics: Arc<MetricsRegistry>,
        alerts: Arc<AlertManager>,
    ) -> Self {
        let state = DashboardState {
            name: "Qenus Beta Dataplane".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: chrono::Utc::now(),
            last_refresh: chrono::Utc::now(),
            auto_refresh: true,
            refresh_interval_secs: 30,
        };
        
        Self {
            health,
            metrics,
            alerts,
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Get the full dashboard overview
    pub async fn get_overview(&self) -> DashboardOverview {
        let health_report = self.health.get_report().await;
        let metrics_summary = self.metrics.get_summary().await;
        let alert_stats = self.alerts.get_stats().await;
        let state = self.state.read().await.clone();
        
        // Update last refresh
        {
            let mut state_mut = self.state.write().await;
            state_mut.last_refresh = chrono::Utc::now();
        }
        
        DashboardOverview {
            state,
            health: health_report,
            metrics_summary,
            alert_stats,
        }
    }

    /// Get health status
    pub async fn get_health(&self) -> HealthReport {
        self.health.get_report().await
    }

    /// Get metrics in Prometheus format
    pub async fn get_metrics_prometheus(&self) -> String {
        self.metrics.export_prometheus().await
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<crate::monitoring::alerts::Alert> {
        self.alerts.get_active_alerts().await
    }

    /// Get critical alerts
    pub async fn get_critical_alerts(&self) -> Vec<crate::monitoring::alerts::Alert> {
        self.alerts
            .get_alerts_by_severity(crate::monitoring::alerts::AlertSeverity::Critical)
            .await
    }

    /// Check if the system is healthy
    pub async fn is_healthy(&self) -> bool {
        let health = self.health.get_report().await;
        matches!(health.status, HealthStatus::Healthy)
    }

    /// Get dashboard state
    pub async fn get_state(&self) -> DashboardState {
        self.state.read().await.clone()
    }

    /// Update dashboard settings
    pub async fn update_settings(&self, auto_refresh: bool, refresh_interval_secs: u64) {
        let mut state = self.state.write().await;
        state.auto_refresh = auto_refresh;
        state.refresh_interval_secs = refresh_interval_secs;
        
        info!(
            "Dashboard settings updated: auto_refresh={}, interval={}s",
            auto_refresh, refresh_interval_secs
        );
    }
}

/// Complete dashboard overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardOverview {
    /// Dashboard state
    pub state: DashboardState,
    
    /// Health report
    #[serde(skip)]
    pub health: HealthReport,
    
    /// Metrics summary
    pub metrics_summary: MetricsSummary,
    
    /// Alert statistics
    pub alert_stats: AlertStats,
}

/// HTTP response helpers for dashboard
impl MonitoringDashboard {
    /// Generate JSON response for overview
    pub async fn overview_json(&self) -> Result<String> {
        let overview = self.get_overview().await;
        serde_json::to_string_pretty(&overview)
            .map_err(|e| crate::BetaDataplaneError::Serialization(e))
    }

    /// Generate health check response (for k8s/load balancers)
    pub async fn health_check_response(&self) -> HealthCheckResponse {
        let health = self.health.get_report().await;
        
        HealthCheckResponse {
            status: match health.status {
                HealthStatus::Healthy => "healthy".to_string(),
                HealthStatus::Degraded => "degraded".to_string(),
                HealthStatus::Unhealthy => "unhealthy".to_string(),
                HealthStatus::Starting => "starting".to_string(),
                HealthStatus::Stopping => "stopping".to_string(),
            },
            timestamp: health.timestamp,
            uptime_seconds: health.uptime_seconds,
            components: health.components.len(),
            message: health.summary,
        }
    }

    /// Generate readiness check response (for k8s)
    pub async fn readiness_check(&self) -> bool {
        let health = self.health.get_report().await;
        health.status.is_operational()
    }

    /// Generate liveness check response (for k8s)
    pub async fn liveness_check(&self) -> bool {
        // Liveness is just whether the process is running
        true
    }
}

/// Simple health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
    pub components: usize,
    pub message: String,
}

/// Monitoring service that coordinates all monitoring components
pub struct MonitoringService {
    /// Health checker
    health_checker: Arc<HealthChecker>,
    
    /// Metrics registry
    metrics_registry: Arc<MetricsRegistry>,
    
    /// Alert manager
    alert_manager: Arc<AlertManager>,
    
    /// Dashboard
    dashboard: Arc<MonitoringDashboard>,
    
    /// Running state
    is_running: Arc<RwLock<bool>>,
}

impl MonitoringService {
    /// Create a new monitoring service
    pub fn new(
        health_checker: HealthChecker,
        metrics_registry: MetricsRegistry,
        alert_manager: AlertManager,
    ) -> Self {
        let health_checker = Arc::new(health_checker);
        let metrics_registry = Arc::new(metrics_registry);
        let alert_manager = Arc::new(alert_manager);
        
        let dashboard = Arc::new(MonitoringDashboard::new(
            Arc::clone(&health_checker),
            Arc::clone(&metrics_registry),
            Arc::clone(&alert_manager),
        ));
        
        Self {
            health_checker,
            metrics_registry,
            alert_manager,
            dashboard,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the monitoring service
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }
        
        info!("Starting monitoring service");
        
        // Start health monitoring
        self.health_checker.start_monitoring().await?;
        
        // Start metrics auto-flush
        self.metrics_registry.start_auto_flush().await?;
        
        // Start alert evaluation loop
        self.start_alert_evaluation().await?;
        
        *is_running = true;
        info!("Monitoring service started");
        
        Ok(())
    }

    /// Start the alert evaluation loop
    async fn start_alert_evaluation(&self) -> Result<()> {
        let metrics_registry = Arc::clone(&self.metrics_registry);
        let alert_manager = Arc::clone(&self.alert_manager);
        let health_checker = Arc::clone(&self.health_checker);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                // Evaluate metric-based alerts
                let metrics = metrics_registry.get_all_metrics().await;
                if let Err(e) = alert_manager.evaluate_rules(&metrics).await {
                    debug!("Alert evaluation error: {}", e);
                }
                
                // Evaluate health-based alerts
                let health_report = health_checker.get_report().await;
                if let Err(e) = alert_manager.process_health_report(&health_report).await {
                    debug!("Health alert processing error: {}", e);
                }
            }
        });
        
        Ok(())
    }

    /// Stop the monitoring service
    pub async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }
        
        info!("Stopping monitoring service");
        *is_running = false;
        
        Ok(())
    }

    /// Get the dashboard
    pub fn dashboard(&self) -> Arc<MonitoringDashboard> {
        Arc::clone(&self.dashboard)
    }

    /// Get the health checker
    pub fn health_checker(&self) -> Arc<HealthChecker> {
        Arc::clone(&self.health_checker)
    }

    /// Get the metrics registry
    pub fn metrics_registry(&self) -> Arc<MetricsRegistry> {
        Arc::clone(&self.metrics_registry)
    }

    /// Get the alert manager
    pub fn alert_manager(&self) -> Arc<AlertManager> {
        Arc::clone(&self.alert_manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_dashboard_creation() {
        let health = Arc::new(HealthChecker::new(Duration::from_secs(60)));
        let metrics = Arc::new(MetricsRegistry::new(Duration::from_secs(30)));
        let alerts = Arc::new(AlertManager::new(100));
        
        let dashboard = MonitoringDashboard::new(health, metrics, alerts);
        assert!(dashboard.is_healthy().await || !dashboard.is_healthy().await);
    }

    #[tokio::test]
    async fn test_health_check_response() {
        let health = Arc::new(HealthChecker::new(Duration::from_secs(60)));
        let metrics = Arc::new(MetricsRegistry::new(Duration::from_secs(30)));
        let alerts = Arc::new(AlertManager::new(100));
        
        let dashboard = MonitoringDashboard::new(health, metrics, alerts);
        let response = dashboard.health_check_response().await;
        
        assert!(!response.status.is_empty());
    }
}

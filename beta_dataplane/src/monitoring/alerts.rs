//! Alerting system for critical events and threshold violations
//!
//! Provides rule-based alerting with multiple notification channels.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::monitoring::health::{HealthStatus, HealthReport};
use crate::monitoring::metrics::{Metric, MetricValue};
use crate::{Result, BetaDataplaneError};

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    
    /// Warning - should be investigated
    Warning,
    
    /// Error - requires attention
    Error,
    
    /// Critical - requires immediate action
    Critical,
}

impl AlertSeverity {
    /// Check if the severity requires immediate attention
    pub fn requires_immediate_attention(&self) -> bool {
        matches!(self, AlertSeverity::Critical)
    }

    /// Get the severity as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "WARNING",
            AlertSeverity::Error => "ERROR",
            AlertSeverity::Critical => "CRITICAL",
        }
    }
}

/// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertState {
    /// Alert is currently firing
    Firing,
    
    /// Alert was firing but has been resolved
    Resolved,
    
    /// Alert is pending (waiting for threshold to be exceeded)
    Pending,
}

/// An alert instance
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    
    /// Alert name
    pub name: String,
    
    /// Severity level
    pub severity: AlertSeverity,
    
    /// Current state
    pub state: AlertState,
    
    /// Description
    pub description: String,
    
    /// Component that triggered the alert
    pub component: String,
    
    /// When the alert was first triggered
    pub triggered_at: chrono::DateTime<chrono::Utc>,
    
    /// When the alert was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
    
    /// Number of times this alert has fired
    pub fire_count: u64,
    
    /// Additional context
    pub context: HashMap<String, String>,
}

impl Alert {
    /// Create a new alert
    pub fn new(
        id: String,
        name: String,
        severity: AlertSeverity,
        description: String,
        component: String,
    ) -> Self {
        Self {
            id,
            name,
            severity,
            state: AlertState::Pending,
            description,
            component,
            triggered_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
            fire_count: 0,
            context: HashMap::new(),
        }
    }

    /// Mark the alert as firing
    pub fn fire(&mut self) {
        self.state = AlertState::Firing;
        self.fire_count += 1;
        self.last_updated = chrono::Utc::now();
    }

    /// Mark the alert as resolved
    pub fn resolve(&mut self) {
        self.state = AlertState::Resolved;
        self.last_updated = chrono::Utc::now();
    }

    /// Add context to the alert
    pub fn add_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
        self.last_updated = chrono::Utc::now();
    }

    /// Get the duration since the alert was triggered
    pub fn duration_since_triggered(&self) -> Duration {
        let now = chrono::Utc::now();
        (now - self.triggered_at)
            .to_std()
            .unwrap_or(Duration::from_secs(0))
    }
}

/// Alert rule for threshold-based alerting
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// Rule ID
    pub id: String,
    
    /// Rule name
    pub name: String,
    
    /// Metric name to monitor
    pub metric_name: String,
    
    /// Threshold value
    pub threshold: f64,
    
    /// Comparison operator
    pub operator: ComparisonOperator,
    
    /// Severity of the alert if triggered
    pub severity: AlertSeverity,
    
    /// How long the condition must be true before firing
    pub for_duration: Duration,
    
    /// Description template
    pub description: String,
}

/// Comparison operators for alert rules
#[derive(Debug, Clone, Copy)]
pub enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl ComparisonOperator {
    /// Evaluate the comparison
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            ComparisonOperator::GreaterThan => value > threshold,
            ComparisonOperator::GreaterThanOrEqual => value >= threshold,
            ComparisonOperator::LessThan => value < threshold,
            ComparisonOperator::LessThanOrEqual => value <= threshold,
            ComparisonOperator::Equal => (value - threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (value - threshold).abs() >= f64::EPSILON,
        }
    }

    /// Get the operator as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            ComparisonOperator::GreaterThan => ">",
            ComparisonOperator::GreaterThanOrEqual => ">=",
            ComparisonOperator::LessThan => "<",
            ComparisonOperator::LessThanOrEqual => "<=",
            ComparisonOperator::Equal => "==",
            ComparisonOperator::NotEqual => "!=",
        }
    }
}

/// Alert manager that evaluates rules and sends notifications
pub struct AlertManager {
    /// Active alerts
    alerts: Arc<RwLock<HashMap<String, Alert>>>,
    
    /// Alert rules
    rules: Arc<RwLock<Vec<AlertRule>>>,
    
    /// Pending rule violations (for "for" duration)
    pending_violations: Arc<RwLock<HashMap<String, Instant>>>,
    
    /// Alert history
    alert_history: Arc<RwLock<Vec<Alert>>>,
    
    /// Maximum history size
    max_history_size: usize,
    
    /// Alert channel for notifications
    alert_tx: mpsc::Sender<Alert>,
    alert_rx: Arc<RwLock<mpsc::Receiver<Alert>>>,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(max_history_size: usize) -> Self {
        let (alert_tx, alert_rx) = mpsc::channel(1000);
        
        Self {
            alerts: Arc::new(RwLock::new(HashMap::new())),
            rules: Arc::new(RwLock::new(Vec::new())),
            pending_violations: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size,
            alert_tx,
            alert_rx: Arc::new(RwLock::new(alert_rx)),
        }
    }

    /// Add an alert rule
    pub async fn add_rule(&self, rule: AlertRule) {
        let mut rules = self.rules.write().await;
        info!("Adding alert rule: {} ({})", rule.name, rule.id);
        rules.push(rule);
    }

    /// Evaluate all rules against current metrics
    pub async fn evaluate_rules(&self, metrics: &HashMap<String, Metric>) -> Result<()> {
        let rules = self.rules.read().await;
        
        for rule in rules.iter() {
            if let Some(metric) = metrics.get(&rule.metric_name) {
                self.evaluate_rule(rule, metric).await?;
            }
        }
        
        Ok(())
    }

    /// Evaluate a single rule
    async fn evaluate_rule(&self, rule: &AlertRule, metric: &Metric) -> Result<()> {
        // Extract numeric value from metric
        let value = match metric.value {
            MetricValue::Counter(v) => v as f64,
            MetricValue::Gauge(v) => v,
            MetricValue::Histogram { avg, .. } => avg,
        };

        // Check if the condition is met
        let condition_met = rule.operator.evaluate(value, rule.threshold);
        
        if condition_met {
            // Check if we need to wait for "for" duration
            let mut pending = self.pending_violations.write().await;
            let violation_start = pending
                .entry(rule.id.clone())
                .or_insert_with(Instant::now);
            
            if violation_start.elapsed() >= rule.for_duration {
                // Fire the alert
                self.fire_alert(rule, value).await?;
                pending.remove(&rule.id);
            }
        } else {
            // Condition not met, check if we should resolve
            let mut pending = self.pending_violations.write().await;
            pending.remove(&rule.id);
            
            self.resolve_alert(&rule.id).await?;
        }
        
        Ok(())
    }

    /// Fire an alert
    async fn fire_alert(&self, rule: &AlertRule, value: f64) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        
        let alert_id = rule.id.clone();
        
        if let Some(existing_alert) = alerts.get_mut(&alert_id) {
            // Alert already exists, update it
            existing_alert.fire();
            debug!("Alert fired again: {} (count: {})", rule.name, existing_alert.fire_count);
        } else {
            // Create new alert
            let mut alert = Alert::new(
                alert_id.clone(),
                rule.name.clone(),
                rule.severity,
                rule.description.clone(),
                rule.metric_name.clone(),
            );
            alert.fire();
            alert.add_context("value".to_string(), value.to_string());
            alert.add_context("threshold".to_string(), rule.threshold.to_string());
            alert.add_context("operator".to_string(), rule.operator.as_str().to_string());
            
            info!("🚨 Alert fired: {} - {}", rule.name, rule.description);
            
            // Send notification
            if let Err(e) = self.alert_tx.try_send(alert.clone()) {
                warn!("Failed to send alert notification: {}", e);
            }
            
            alerts.insert(alert_id, alert);
        }
        
        Ok(())
    }

    /// Resolve an alert
    async fn resolve_alert(&self, alert_id: &str) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        
        if let Some(mut alert) = alerts.remove(alert_id) {
            alert.resolve();
            info!("✅ Alert resolved: {}", alert.name);
            
            // Add to history
            let mut history = self.alert_history.write().await;
            history.push(alert);
            
            // Trim history if needed
            let history_len = history.len();
            if history_len > self.max_history_size {
                history.drain(0..history_len - self.max_history_size);
            }
        }
        
        Ok(())
    }

    /// Create an alert for a health check failure
    pub async fn alert_health_failure(&self, component: &str, status: HealthStatus, message: Option<String>) -> Result<()> {
        let severity = match status {
            HealthStatus::Unhealthy => AlertSeverity::Critical,
            HealthStatus::Degraded => AlertSeverity::Warning,
            _ => return Ok(()),
        };
        
        let alert_id = format!("health_{}", component);
        let mut alert = Alert::new(
            alert_id.clone(),
            format!("Health Check Failed: {}", component),
            severity,
            message.unwrap_or_else(|| format!("{} is {:?}", component, status)),
            component.to_string(),
        );
        alert.fire();
        
        let mut alerts = self.alerts.write().await;
        alerts.insert(alert_id, alert.clone());
        
        // Send notification
        if let Err(e) = self.alert_tx.try_send(alert) {
            warn!("Failed to send health alert notification: {}", e);
        }
        
        Ok(())
    }

    /// Process health report and generate alerts
    pub async fn process_health_report(&self, report: &HealthReport) -> Result<()> {
        for (component_name, component) in &report.components {
            match component.status {
                HealthStatus::Unhealthy | HealthStatus::Degraded => {
                    self.alert_health_failure(
                        component_name,
                        component.status,
                        component.message.clone(),
                    )
                    .await?;
                }
                HealthStatus::Healthy => {
                    // Resolve any existing health alerts
                    let alert_id = format!("health_{}", component_name);
                    self.resolve_alert(&alert_id).await?;
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    /// Get all active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.values().cloned().collect()
    }

    /// Get alert history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<Alert> {
        let history = self.alert_history.read().await;
        let limit = limit.unwrap_or(history.len());
        
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get alerts by severity
    pub async fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts
            .values()
            .filter(|a| a.severity == severity)
            .cloned()
            .collect()
    }

    /// Clear all alerts
    pub async fn clear_all(&self) {
        let mut alerts = self.alerts.write().await;
        alerts.clear();
        
        let mut pending = self.pending_violations.write().await;
        pending.clear();
        
        info!("All alerts cleared");
    }

    /// Get alert statistics
    pub async fn get_stats(&self) -> AlertStats {
        let alerts = self.alerts.read().await;
        let history = self.alert_history.read().await;
        
        let mut critical_count = 0;
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;
        
        for alert in alerts.values() {
            match alert.severity {
                AlertSeverity::Critical => critical_count += 1,
                AlertSeverity::Error => error_count += 1,
                AlertSeverity::Warning => warning_count += 1,
                AlertSeverity::Info => info_count += 1,
            }
        }
        
        AlertStats {
            active_alerts: alerts.len(),
            critical_count,
            error_count,
            warning_count,
            info_count,
            total_history: history.len(),
        }
    }
}

/// Alert statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertStats {
    pub active_alerts: usize,
    pub critical_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub total_history: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_operator() {
        assert!(ComparisonOperator::GreaterThan.evaluate(10.0, 5.0));
        assert!(!ComparisonOperator::GreaterThan.evaluate(5.0, 10.0));
        
        assert!(ComparisonOperator::LessThan.evaluate(5.0, 10.0));
        assert!(!ComparisonOperator::LessThan.evaluate(10.0, 5.0));
    }

    #[tokio::test]
    async fn test_alert_manager() {
        let manager = AlertManager::new(100);
        
        let rule = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            metric_name: "test_metric".to_string(),
            threshold: 100.0,
            operator: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Warning,
            for_duration: Duration::from_secs(0),
            description: "Test alert".to_string(),
        };
        
        manager.add_rule(rule).await;
        
        let rules = manager.rules.read().await;
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn test_alert_severity() {
        assert!(AlertSeverity::Critical > AlertSeverity::Error);
        assert!(AlertSeverity::Error > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }
}

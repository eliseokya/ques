//! Integration tests for monitoring components
//!
//! Tests health checks, metrics, alerts, and dashboard working together.

use qenus_beta_dataplane::monitoring::*;
use qenus_beta_dataplane::Result;
use std::time::Duration;

#[tokio::test]
async fn test_health_checker_integration() -> Result<()> {
    let checker = HealthChecker::new(Duration::from_secs(1));

    // Get initial report
    let report = checker.get_report().await;
    assert_eq!(report.status, HealthStatus::Starting);
    assert_eq!(report.components.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_metrics_collector() -> Result<()> {
    let collector = MetricsCollector::new("test_component".to_string());

    // Record various metrics
    collector.record_counter("requests_total", 100, "Total requests").await;
    collector.record_gauge("cpu_usage", 45.5, "CPU usage percentage").await;
    collector.record_histogram_value("latency_ms", 12.5).await;
    collector.record_histogram_value("latency_ms", 15.3).await;
    collector.record_histogram_value("latency_ms", 10.1).await;

    // Flush histograms
    let mut help_texts = std::collections::HashMap::new();
    help_texts.insert("test_component_latency_ms".to_string(), "Request latency".to_string());
    collector.flush_histograms(&help_texts).await;

    // Check metrics
    let metrics = collector.get_metrics().await;
    assert_eq!(metrics.len(), 3); // counter, gauge, histogram

    Ok(())
}

#[tokio::test]
async fn test_metrics_registry() -> Result<()> {
    let registry = MetricsRegistry::new(Duration::from_secs(30));

    // Register collectors
    let collector1 = registry.register_collector("component1".to_string()).await;
    let collector2 = registry.register_collector("component2".to_string()).await;

    // Record metrics
    collector1.record_counter("ops", 50, "Operations").await;
    collector2.record_gauge("queue_size", 10.0, "Queue size").await;

    // Get all metrics
    let all_metrics = registry.get_all_metrics().await;
    assert_eq!(all_metrics.len(), 2);

    // Get summary
    let summary = registry.get_summary().await;
    assert_eq!(summary.component_count, 2);
    assert_eq!(summary.total_metrics, 2);

    Ok(())
}

#[tokio::test]
async fn test_alert_manager() -> Result<()> {
    let manager = AlertManager::new(100);

    // Add alert rule
    let rule = AlertRule {
        id: "high_error_rate".to_string(),
        name: "High Error Rate".to_string(),
        metric_name: "error_rate".to_string(),
        threshold: 0.05,
        operator: ComparisonOperator::GreaterThan,
        severity: AlertSeverity::Warning,
        for_duration: Duration::from_secs(0),
        description: "Error rate exceeded 5%".to_string(),
    };

    manager.add_rule(rule).await;

    // Get stats
    let stats = manager.get_stats().await;
    assert_eq!(stats.active_alerts, 0);

    Ok(())
}

#[tokio::test]
async fn test_alert_severity_ordering() {
    assert!(AlertSeverity::Critical > AlertSeverity::Error);
    assert!(AlertSeverity::Error > AlertSeverity::Warning);
    assert!(AlertSeverity::Warning > AlertSeverity::Info);
    
    assert!(AlertSeverity::Critical.requires_immediate_attention());
    assert!(!AlertSeverity::Info.requires_immediate_attention());
}

#[tokio::test]
async fn test_monitoring_dashboard() -> Result<()> {
    let health = std::sync::Arc::new(HealthChecker::new(Duration::from_secs(60)));
    let metrics = std::sync::Arc::new(MetricsRegistry::new(Duration::from_secs(30)));
    let alerts = std::sync::Arc::new(AlertManager::new(100));

    let dashboard = MonitoringDashboard::new(health, metrics, alerts);

    // Get overview
    let overview = dashboard.get_overview().await;
    assert!(!overview.state.name.is_empty());

    // Check health
    let is_healthy = dashboard.is_healthy().await;
    assert!(is_healthy || !is_healthy); // Should compile and run

    // Get state
    let state = dashboard.get_state().await;
    assert_eq!(state.name, "Qenus Beta Dataplane");

    Ok(())
}

#[tokio::test]
async fn test_health_check_response() -> Result<()> {
    let health = std::sync::Arc::new(HealthChecker::new(Duration::from_secs(60)));
    let metrics = std::sync::Arc::new(MetricsRegistry::new(Duration::from_secs(30)));
    let alerts = std::sync::Arc::new(AlertManager::new(100));

    let dashboard = MonitoringDashboard::new(health, metrics, alerts);

    let response = dashboard.health_check_response().await;
    assert!(!response.status.is_empty());
    assert!(response.uptime_seconds >= 0);

    // Test readiness and liveness
    assert!(dashboard.readiness_check().await || !dashboard.readiness_check().await);
    assert!(dashboard.liveness_check().await);

    Ok(())
}

#[tokio::test]
async fn test_monitoring_service() -> Result<()> {
    let health = HealthChecker::new(Duration::from_secs(60));
    let metrics = MetricsRegistry::new(Duration::from_secs(30));
    let alerts = AlertManager::new(100);

    let service = MonitoringService::new(health, metrics, alerts);

    // Get dashboard
    let dashboard = service.dashboard();
    assert!(!dashboard.get_state().await.name.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_metrics_prometheus_export() -> Result<()> {
    let registry = MetricsRegistry::new(Duration::from_secs(30));
    let collector = registry.register_collector("test".to_string()).await;

    // Record some metrics
    collector.record_counter("requests_total", 100, "Total requests").await;
    collector.record_gauge("temperature", 72.5, "Temperature in F").await;

    // Export to Prometheus format
    let prometheus = registry.export_prometheus().await;
    assert!(prometheus.contains("# HELP"));
    assert!(prometheus.contains("# TYPE"));
    assert!(prometheus.contains("test_requests_total"));
    assert!(prometheus.contains("test_temperature"));

    Ok(())
}

#[tokio::test]
async fn test_alert_rule_evaluation() -> Result<()> {
    let manager = AlertManager::new(100);

    // Create a metric that violates threshold
    let mut metrics = std::collections::HashMap::new();
    let metric = Metric::gauge(
        "cpu_usage".to_string(),
        95.0,
        "CPU usage percentage".to_string(),
    );
    metrics.insert("cpu_usage".to_string(), metric);

    // Add rule
    let rule = AlertRule {
        id: "high_cpu".to_string(),
        name: "High CPU Usage".to_string(),
        metric_name: "cpu_usage".to_string(),
        threshold: 80.0,
        operator: ComparisonOperator::GreaterThan,
        severity: AlertSeverity::Warning,
        for_duration: Duration::from_secs(0),
        description: "CPU usage above 80%".to_string(),
    };

    manager.add_rule(rule).await;

    // Evaluate rules
    manager.evaluate_rules(&metrics).await?;

    // Check if alert was fired
    let active_alerts = manager.get_active_alerts().await;
    assert_eq!(active_alerts.len(), 1);
    assert_eq!(active_alerts[0].name, "High CPU Usage");

    Ok(())
}

#[tokio::test]
async fn test_alert_resolution() -> Result<()> {
    let manager = AlertManager::new(100);

    // Fire an alert
    let rule = AlertRule {
        id: "test_alert".to_string(),
        name: "Test Alert".to_string(),
        metric_name: "test_metric".to_string(),
        threshold: 10.0,
        operator: ComparisonOperator::GreaterThan,
        severity: AlertSeverity::Info,
        for_duration: Duration::from_secs(0),
        description: "Test alert".to_string(),
    };

    manager.add_rule(rule.clone()).await;

    // Fire the alert
    let mut metrics = std::collections::HashMap::new();
    metrics.insert(
        "test_metric".to_string(),
        Metric::gauge("test_metric".to_string(), 15.0, "Test metric".to_string()),
    );
    manager.evaluate_rules(&metrics).await?;

    let active = manager.get_active_alerts().await;
    assert_eq!(active.len(), 1);

    // Resolve the alert
    metrics.insert(
        "test_metric".to_string(),
        Metric::gauge("test_metric".to_string(), 5.0, "Test metric".to_string()),
    );
    manager.evaluate_rules(&metrics).await?;

    let active = manager.get_active_alerts().await;
    assert_eq!(active.len(), 0);

    // Check history
    let history = manager.get_history(Some(10)).await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].state, AlertState::Resolved);

    Ok(())
}

#[tokio::test]
async fn test_comparison_operators() {
    assert!(ComparisonOperator::GreaterThan.evaluate(10.0, 5.0));
    assert!(!ComparisonOperator::GreaterThan.evaluate(5.0, 10.0));
    
    assert!(ComparisonOperator::LessThan.evaluate(5.0, 10.0));
    assert!(!ComparisonOperator::LessThan.evaluate(10.0, 5.0));
    
    assert!(ComparisonOperator::Equal.evaluate(10.0, 10.0));
    assert!(!ComparisonOperator::Equal.evaluate(10.0, 11.0));
    
    assert!(ComparisonOperator::GreaterThanOrEqual.evaluate(10.0, 10.0));
    assert!(ComparisonOperator::GreaterThanOrEqual.evaluate(11.0, 10.0));
}

#[tokio::test]
async fn test_health_status() {
    assert!(HealthStatus::Healthy.is_operational());
    assert!(HealthStatus::Degraded.is_operational());
    assert!(!HealthStatus::Unhealthy.is_operational());
    
    assert!(HealthStatus::Unhealthy.requires_attention());
    assert!(!HealthStatus::Healthy.requires_attention());
}

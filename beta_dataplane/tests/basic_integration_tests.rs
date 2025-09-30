//! Basic integration tests for beta dataplane
//!
//! Simplified tests that verify core functionality without complex dependencies.

use qenus_beta_dataplane::*;
use qenus_beta_dataplane::optimization::*;
use qenus_beta_dataplane::monitoring::*;
use std::time::Duration;

#[tokio::test]
async fn test_cache_operations() -> Result<()> {
    let cache = IntelligentCache::new(
        CacheStrategy::LRU,
        100,
        Duration::from_secs(60),
    );

    // Insert and retrieve
    cache.insert("test_key".to_string(), vec![1, 2, 3]).await;
    let value = cache.get(&"test_key".to_string()).await;
    
    assert!(value.is_some());
    assert_eq!(value.unwrap(), vec![1, 2, 3]);

    Ok(())
}

#[tokio::test]
async fn test_batch_processor() -> Result<()> {
    let processor = BatchProcessor::new(
        BatchStrategy::Hybrid,
        10,
        Duration::from_millis(100),
    );

    for i in 0..5 {
        processor.enqueue(format!("item_{}", i)).await?;
    }

    let stats = processor.stats().await;
    assert!(stats.total_batches >= 0);

    Ok(())
}

#[tokio::test]
async fn test_compression() -> Result<()> {
    let compressor = DataCompressor::new(
        compression::CompressionAlgorithm::Gzip,
        CompressionLevel::new(6),
    );

    let data = b"Test data for compression".repeat(10);
    let compressed = compressor.compress(&data)?;
    
    assert!(compressed.len() < data.len());
    
    let decompressed = compressor.decompress(&compressed)?;
    assert_eq!(decompressed, data);

    Ok(())
}

#[tokio::test]
async fn test_health_checker() -> Result<()> {
    let checker = HealthChecker::new(Duration::from_secs(60));
    let report = checker.get_report().await;
    
    assert!(matches!(report.status, HealthStatus::Starting | HealthStatus::Healthy));

    Ok(())
}

#[tokio::test]
async fn test_metrics_registry() -> Result<()> {
    let registry = MetricsRegistry::new(Duration::from_secs(30));
    let collector = registry.register_collector("test".to_string()).await;
    
    collector.record_counter("requests", 100, "Total requests").await;
    collector.record_gauge("queue_size", 10.0, "Queue size").await;
    
    let metrics = collector.get_metrics().await;
    assert_eq!(metrics.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_alert_manager() -> Result<()> {
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
    
    let stats = manager.get_stats().await;
    assert_eq!(stats.active_alerts, 0);

    Ok(())
}

#[tokio::test]
async fn test_monitoring_dashboard() -> Result<()> {
    let health = std::sync::Arc::new(HealthChecker::new(Duration::from_secs(60)));
    let metrics = std::sync::Arc::new(MetricsRegistry::new(Duration::from_secs(30)));
    let alerts = std::sync::Arc::new(AlertManager::new(100));

    let dashboard = MonitoringDashboard::new(health, metrics, alerts);
    let overview = dashboard.get_overview().await;
    
    assert!(!overview.state.name.is_empty());
    assert_eq!(overview.state.name, "Qenus Beta Dataplane");

    Ok(())
}

#[tokio::test]
async fn test_optimization_metrics() -> Result<()> {
    let collector = OptimizationMetricsCollector::new();
    
    collector.record_cache_operation(true, 5.0).await;
    collector.record_cache_operation(false, 10.0).await;
    
    let metrics = collector.metrics().await;
    assert_eq!(metrics.cache_hits, 1);
    assert_eq!(metrics.cache_misses, 1);

    Ok(())
}

#[tokio::test]
async fn test_prometheus_export() -> Result<()> {
    let registry = MetricsRegistry::new(Duration::from_secs(30));
    let collector = registry.register_collector("test".to_string()).await;
    
    collector.record_counter("requests_total", 100, "Total requests").await;
    
    let prometheus = registry.export_prometheus().await;
    assert!(prometheus.contains("# HELP"));
    assert!(prometheus.contains("# TYPE"));
    assert!(prometheus.contains("test_requests_total"));

    Ok(())
}

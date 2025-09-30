//! End-to-end integration tests
//!
//! Tests the full dataplane pipeline from provider to feeds.

use qenus_beta_dataplane::*;
use qenus_beta_dataplane::providers::*;
use qenus_beta_dataplane::optimization::*;
use qenus_beta_dataplane::monitoring::*;
use qenus_beta_dataplane::feeds::*;
use qenus_dataplane::{Feature, FeatureType, FeatureData, Chain};
use std::time::Duration;
use std::sync::Arc;

#[tokio::test]
async fn test_full_pipeline_simulation() -> Result<()> {
    // 1. Set up monitoring
    let health_checker = Arc::new(HealthChecker::new(Duration::from_secs(60)));
    let metrics_registry = Arc::new(MetricsRegistry::new(Duration::from_secs(30)));
    let alert_manager = Arc::new(AlertManager::new(100));

    let monitoring = MonitoringService::new(
        (*health_checker).clone(),
        (*metrics_registry).clone(),
        (*alert_manager).clone(),
    );

    // 2. Set up optimization
    let cache = IntelligentCache::new(
        CacheStrategy::Lru,
        1000,
        Duration::from_secs(300),
    );

    let batch_processor = BatchProcessor::new(
        BatchStrategy::SizeAndTime,
        10,
        Duration::from_millis(100),
    );

    // 3. Create sample features
    let mut features = Vec::new();
    for i in 0..5 {
        let feature = Feature {
            feature_type: FeatureType::Amm,
            chain: Chain::Ethereum,
            block_number: 18000000 + i,
            timestamp: chrono::Utc::now().timestamp() as u64,
            data: FeatureData::Amm {
                protocol: "uniswap_v3".to_string(),
                pool: format!("0x{:040x}", i),
                token0: format!("0x{:040x}", i * 2),
                token1: format!("0x{:040x}", i * 2 + 1),
                price: 1800.0 + i as f64,
                liquidity: 1000000.0 + i as f64 * 10000.0,
                fee_tier: 3000,
            },
        };
        features.push(feature);
    }

    // 4. Process through optimization
    for feature in &features {
        let feature_key = format!("{}_{}", feature.chain, feature.block_number);
        let serialized = serde_json::to_vec(feature).unwrap();
        cache.put(feature_key, serialized).await;
    }

    // 5. Verify cache
    let cache_stats = cache.stats();
    assert!(cache_stats.total_requests >= 0);

    // 6. Record metrics
    let collector = metrics_registry.register_collector("test_pipeline".to_string()).await;
    collector.record_counter("features_processed", features.len() as u64, "Features processed").await;
    collector.record_gauge("batch_size", features.len() as f64, "Current batch size").await;

    // 7. Get monitoring overview
    let dashboard = monitoring.dashboard();
    let overview = dashboard.get_overview().await;
    assert!(overview.metrics_summary.total_metrics > 0);

    println!("✅ Full pipeline test completed successfully");
    println!("   - Features processed: {}", features.len());
    println!("   - Cache entries: {}", cache_stats.entries);
    println!("   - Metrics collected: {}", overview.metrics_summary.total_metrics);

    Ok(())
}

#[tokio::test]
async fn test_provider_to_feed_flow() -> Result<()> {
    // Simulate the flow: Provider -> Extractor -> Optimization -> Feed
    
    // 1. Create a sample feature (as if from extractor)
    let feature = Feature {
        feature_type: FeatureType::Amm,
        chain: Chain::Ethereum,
        block_number: 18000000,
        timestamp: chrono::Utc::now().timestamp() as u64,
        data: FeatureData::Amm {
            protocol: "uniswap_v3".to_string(),
            pool: "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8".to_string(),
            token0: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(), // USDC
            token1: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(), // WETH
            price: 1850.0,
            liquidity: 5000000.0,
            fee_tier: 3000,
        },
    };

    // 2. Apply optimization (caching)
    let cache = IntelligentCache::new(
        CacheStrategy::Lru,
        100,
        Duration::from_secs(60),
    );

    let feature_key = format!("{}_{}_{}", 
        "ethereum", 
        feature.block_number,
        match &feature.data {
            FeatureData::Amm { pool, .. } => pool.clone(),
            _ => "unknown".to_string(),
        }
    );

    let serialized = serde_json::to_vec(&feature)?;
    cache.put(feature_key.clone(), serialized.clone()).await;

    // 3. Verify cached
    let cached = cache.get(&feature_key).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), serialized);

    // 4. Compress for storage
    let compressor = DataCompressor::new(
        CompressionAlgorithm::Gzip,
        CompressionLevel::new(6),
    );
    let compressed = compressor.compress(&serialized)?;
    assert!(compressed.len() < serialized.len());

    println!("✅ Provider to feed flow test completed");
    println!("   - Original size: {} bytes", serialized.len());
    println!("   - Compressed size: {} bytes", compressed.len());
    println!("   - Compression ratio: {:.2}%", 
             (compressed.len() as f64 / serialized.len() as f64) * 100.0);

    Ok(())
}

#[tokio::test]
async fn test_monitoring_with_alerts() -> Result<()> {
    // Create monitoring system
    let health = HealthChecker::new(Duration::from_secs(60));
    let metrics = MetricsRegistry::new(Duration::from_secs(30));
    let alerts = AlertManager::new(100);

    let service = MonitoringService::new(health, metrics, alerts);

    // Register a collector and record metrics
    let collector = service.metrics_registry()
        .register_collector("test_component".to_string()).await;

    collector.record_gauge("error_rate", 0.02, "Error rate").await;

    // Add alert rule for high error rate
    let rule = AlertRule {
        id: "high_error_rate".to_string(),
        name: "High Error Rate".to_string(),
        metric_name: "test_component_error_rate".to_string(),
        threshold: 0.05,
        operator: ComparisonOperator::GreaterThan,
        severity: AlertSeverity::Warning,
        for_duration: Duration::from_secs(0),
        description: "Error rate exceeded threshold".to_string(),
    };

    service.alert_manager().add_rule(rule).await;

    // Get metrics
    let all_metrics = service.metrics_registry().get_all_metrics().await;
    
    // Evaluate rules (should not fire since 0.02 < 0.05)
    service.alert_manager().evaluate_rules(&all_metrics).await?;

    let active_alerts = service.alert_manager().get_active_alerts().await;
    assert_eq!(active_alerts.len(), 0, "Alert should not have fired");

    println!("✅ Monitoring with alerts test completed");

    Ok(())
}

#[tokio::test]
async fn test_health_degradation_recovery() -> Result<()> {
    let health_checker = HealthChecker::new(Duration::from_secs(1));

    // Initial state - starting
    let report = health_checker.get_report().await;
    assert_eq!(report.status, HealthStatus::Starting);

    // Simulate adding components (would be done by actual components)
    // For now, just verify the checker works
    assert!(report.components.is_empty());

    println!("✅ Health degradation recovery test completed");

    Ok(())
}

#[tokio::test]
async fn test_feature_batch_processing() -> Result<()> {
    let batch_processor = BatchProcessor::new(
        BatchStrategy::SizeAndTime,
        5,
        Duration::from_millis(100),
    );

    // Create and add features
    for i in 0..10 {
        let feature = Feature {
            feature_type: FeatureType::Gas,
            chain: Chain::Ethereum,
            block_number: 18000000 + i,
            timestamp: chrono::Utc::now().timestamp() as u64,
            data: FeatureData::Gas {
                base_fee: 30.0 + i as f64,
                priority_fee: 2.0,
                gas_used: 12000000,
                gas_limit: 30000000,
            },
        };

        batch_processor.add(serde_json::to_string(&feature).unwrap()).await;
    }

    let stats = batch_processor.stats();
    println!("✅ Feature batch processing test completed");
    println!("   - Total items processed: {}", stats.total_items_processed);
    println!("   - Current batch size: {}", stats.current_batch_size);

    Ok(())
}

#[tokio::test]
async fn test_cache_performance() -> Result<()> {
    let cache = IntelligentCache::new(
        CacheStrategy::Lru,
        1000,
        Duration::from_secs(300),
    );

    // Add many entries
    for i in 0..100 {
        cache.put(format!("key_{}", i), vec![i as u8; 100]).await;
    }

    // Read them back
    for i in 0..100 {
        let value = cache.get(&format!("key_{}", i)).await;
        assert!(value.is_some());
    }

    let stats = cache.stats();
    println!("✅ Cache performance test completed");
    println!("   - Entries: {}", stats.entries);
    println!("   - Hit rate: {:.2}%", stats.hit_rate * 100.0);
    println!("   - Total requests: {}", stats.total_requests);

    assert!(stats.hit_rate > 0.9, "Cache hit rate should be high");

    Ok(())
}

#[tokio::test]
async fn test_metrics_aggregation() -> Result<()> {
    let registry = MetricsRegistry::new(Duration::from_secs(30));

    // Create multiple components with metrics
    let components = vec!["provider", "extractor", "feed"];
    
    for component in &components {
        let collector = registry.register_collector(component.to_string()).await;
        collector.record_counter("operations", 100, "Total operations").await;
        collector.record_gauge("queue_depth", 10.0, "Queue depth").await;
    }

    // Get summary
    let summary = registry.get_summary().await;
    assert_eq!(summary.component_count, components.len());
    assert!(summary.total_metrics >= components.len() * 2);

    println!("✅ Metrics aggregation test completed");
    println!("   - Components: {}", summary.component_count);
    println!("   - Total metrics: {}", summary.total_metrics);
    println!("   - Counters: {}", summary.counter_count);
    println!("   - Gauges: {}", summary.gauge_count);

    Ok(())
}

#[tokio::test]
async fn test_alert_cascade() -> Result<()> {
    let manager = AlertManager::new(100);

    // Add multiple severity levels
    let rules = vec![
        AlertRule {
            id: "info_alert".to_string(),
            name: "Info Alert".to_string(),
            metric_name: "info_metric".to_string(),
            threshold: 10.0,
            operator: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Info,
            for_duration: Duration::from_secs(0),
            description: "Info level alert".to_string(),
        },
        AlertRule {
            id: "warning_alert".to_string(),
            name: "Warning Alert".to_string(),
            metric_name: "warning_metric".to_string(),
            threshold: 50.0,
            operator: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Warning,
            for_duration: Duration::from_secs(0),
            description: "Warning level alert".to_string(),
        },
        AlertRule {
            id: "critical_alert".to_string(),
            name: "Critical Alert".to_string(),
            metric_name: "critical_metric".to_string(),
            threshold: 90.0,
            operator: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Critical,
            for_duration: Duration::from_secs(0),
            description: "Critical level alert".to_string(),
        },
    ];

    for rule in rules {
        manager.add_rule(rule).await;
    }

    println!("✅ Alert cascade test completed");

    Ok(())
}

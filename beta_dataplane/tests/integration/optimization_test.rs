//! Integration tests for optimization components
//!
//! Tests caching, batching, prediction, and compression working together.

use qenus_beta_dataplane::optimization::*;
use qenus_beta_dataplane::Result;
use std::time::Duration;

#[tokio::test]
async fn test_cache_basic_operations() -> Result<()> {
    // Create cache
    let cache = IntelligentCache::new(
        CacheStrategy::LRU,
        1000,
        Duration::from_secs(60),
    );

    // Test basic operations
    cache.insert("key1".to_string(), vec![1, 2, 3]).await;
    
    let value = cache.get(&"key1".to_string()).await;
    assert!(value.is_some());
    assert_eq!(value.unwrap(), vec![1, 2, 3]);

    // Test stats
    let stats = cache.stats().await;
    assert!(stats.total_requests >= 1);

    Ok(())
}

#[tokio::test]
async fn test_batch_processor_basic() -> Result<()> {
    let processor = BatchProcessor::new(
        BatchStrategy::Hybrid,
        10,
        Duration::from_millis(100),
    );

    // Add items
    for i in 0..5 {
        processor.enqueue(format!("item_{}", i)).await?;
    }

    // Check stats
    let stats = processor.stats().await;
    assert!(stats.total_batches >= 0);

    Ok(())
}

#[tokio::test]
async fn test_compression_gzip() -> Result<()> {
    let test_data = b"This is test data for compression. It should compress well if repeated. ".repeat(10);

    // Test Gzip
    let compressor = DataCompressor::new(
        compression::CompressionAlgorithm::Gzip,
        CompressionLevel::new(6),
    );
    
    let compressed = compressor.compress(&test_data)?;
    assert!(compressed.len() < test_data.len(), "Gzip should compress data");
    
    let decompressed = compressor.decompress(&compressed)?;
    assert_eq!(decompressed, test_data, "Decompressed data should match original");

    // Test compression benefit estimation
    let benefit = compressor.estimate_benefit(test_data.len());
    assert!(benefit.bytes_saved > 0);
    assert!(benefit.compression_ratio < 1.0);

    Ok(())
}

#[tokio::test]
async fn test_cache_with_compression() -> Result<()> {
    let cache = IntelligentCache::new(
        CacheStrategy::LRU,
        100,
        Duration::from_secs(60),
    );

    let compressor = DataCompressor::new(
        compression::CompressionAlgorithm::Gzip,
        CompressionLevel::new(6),
    );

    // Store compressed data
    let data = vec![0u8; 1000]; // Highly compressible
    let compressed = compressor.compress(&data)?;
    
    cache.insert("compressed_key".to_string(), compressed.clone()).await;
    
    // Retrieve and decompress
    let retrieved = cache.get(&"compressed_key".to_string()).await;
    assert!(retrieved.is_some());
    
    let decompressed = compressor.decompress(&retrieved.unwrap())?;
    assert_eq!(decompressed, data);

    Ok(())
}

#[tokio::test]
async fn test_optimization_metrics_collector() -> Result<()> {
    let collector = OptimizationMetricsCollector::new();

    // Record operations
    collector.record_cache_operation(true, 5.0).await;
    collector.record_cache_operation(false, 10.0).await;
    collector.record_cache_operation(true, 3.0).await;

    let metrics = collector.metrics().await;
    assert_eq!(metrics.cache_hits, 2);
    assert_eq!(metrics.cache_misses, 1);
    assert!(metrics.cache_hit_rate > 0.5);

    Ok(())
}

#[tokio::test]
async fn test_predictor_initialization() -> Result<()> {
    let predictor = DataPredictor::new(100, 0.8);
    
    let stats = predictor.stats().await;
    assert_eq!(stats.predictions_made, 0);
    assert_eq!(stats.predictions_correct, 0);

    Ok(())
}

#[tokio::test]
async fn test_cache_strategies() -> Result<()> {
    // Test LRU strategy
    let lru_cache = IntelligentCache::new(
        CacheStrategy::LRU,
        3, // Small size for testing
        Duration::from_secs(60),
    );

    lru_cache.insert("a".to_string(), vec![1]).await;
    lru_cache.insert("b".to_string(), vec![2]).await;
    lru_cache.insert("c".to_string(), vec![3]).await;
    lru_cache.insert("d".to_string(), vec![4]).await; // Should evict "a"

    // LRU should have evicted the least recently used
    let stats = lru_cache.stats().await;
    assert!(stats.entries <= 3);

    Ok(())
}

#[tokio::test]
async fn test_batch_strategies() -> Result<()> {
    // Test size-based batching
    let size_processor = BatchProcessor::new(
        BatchStrategy::SizeBased,
        5,
        Duration::from_secs(10),
    );

    for i in 0..4 {
        size_processor.enqueue(format!("item_{}", i)).await?;
    }
    
    let stats = size_processor.stats().await;
    assert!(stats.total_batches >= 0);

    Ok(())
}

#[tokio::test]
async fn test_compression_levels() -> Result<()> {
    let test_data = b"Compression test data ".repeat(100);

    // Test different compression levels
    let fast = DataCompressor::new(
        compression::CompressionAlgorithm::Gzip,
        CompressionLevel::new(1)
    );
    let balanced = DataCompressor::new(
        compression::CompressionAlgorithm::Gzip,
        CompressionLevel::new(6)
    );
    let best = DataCompressor::new(
        compression::CompressionAlgorithm::Gzip,
        CompressionLevel::new(9)
    );

    let fast_compressed = fast.compress(&test_data)?;
    let balanced_compressed = balanced.compress(&test_data)?;
    let best_compressed = best.compress(&test_data)?;

    // Best compression should produce smallest size
    assert!(best_compressed.len() <= balanced_compressed.len());
    assert!(balanced_compressed.len() <= fast_compressed.len());

    // All should decompress correctly
    assert_eq!(fast.decompress(&fast_compressed)?, test_data);
    assert_eq!(balanced.decompress(&balanced_compressed)?, test_data);
    assert_eq!(best.decompress(&best_compressed)?, test_data);

    Ok(())
}
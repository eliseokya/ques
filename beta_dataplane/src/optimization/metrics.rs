//! Performance metrics for optimization components
//!
//! Tracks and reports on optimization system performance.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::optimization::{
    caching::CacheStats,
    batching::BatchStats,
    prediction::PredictionStats,
};

/// Comprehensive optimization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationMetrics {
    /// Cache metrics
    pub cache: CacheStats,
    
    /// Batch processing metrics
    pub batching: BatchStats,
    
    /// Prediction metrics
    pub prediction: PredictionStats,
    
    /// Overall performance improvement
    pub performance_improvement: PerformanceImprovement,
    
    /// Component-specific metrics
    pub components: HashMap<String, ComponentMetrics>,
}

/// Performance improvement metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImprovement {
    /// Latency reduction percentage
    pub latency_reduction_pct: f64,
    
    /// RPC calls saved
    pub rpc_calls_saved: u64,
    
    /// Bandwidth saved (bytes)
    pub bandwidth_saved: u64,
    
    /// Cost savings estimate (USD)
    pub estimated_cost_savings: f64,
}

/// Component-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// Component name
    pub name: String,
    
    /// Requests processed
    pub requests_processed: u64,
    
    /// Average processing time (ms)
    pub avg_processing_time_ms: f64,
    
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    
    /// Custom metrics
    pub custom: HashMap<String, f64>,
}

/// Metrics collector for optimization components
pub struct OptimizationMetricsCollector {
    /// Collected metrics
    metrics: Arc<RwLock<OptimizationMetrics>>,
}

impl OptimizationMetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(OptimizationMetrics {
                cache: CacheStats {
                    total_requests: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                    hit_rate: 0.0,
                    total_entries: 0,
                    memory_usage_bytes: 0,
                    avg_entry_age_seconds: 0.0,
                },
                batching: BatchStats {
                    total_batches: 0,
                    total_requests: 0,
                    avg_batch_size: 0.0,
                    avg_processing_time_ms: 0.0,
                    requests_saved: 0,
                },
                prediction: PredictionStats {
                    total_predictions: 0,
                    successful_predictions: 0,
                    failed_predictions: 0,
                    accuracy: 0.0,
                    avg_confidence: 0.0,
                },
                performance_improvement: PerformanceImprovement {
                    latency_reduction_pct: 0.0,
                    rpc_calls_saved: 0,
                    bandwidth_saved: 0,
                    estimated_cost_savings: 0.0,
                },
                components: HashMap::new(),
            })),
        }
    }

    /// Update cache metrics
    pub async fn update_cache_metrics(&self, stats: CacheStats) {
        let mut metrics = self.metrics.write().await;
        metrics.cache = stats;
    }

    /// Update batching metrics
    pub async fn update_batching_metrics(&self, stats: BatchStats) {
        let mut metrics = self.metrics.write().await;
        metrics.batching = stats;
    }

    /// Update prediction metrics
    pub async fn update_prediction_metrics(&self, stats: PredictionStats) {
        let mut metrics = self.metrics.write().await;
        metrics.prediction = stats;
    }

    /// Calculate performance improvements
    pub async fn calculate_improvements(&self) {
        let mut metrics = self.metrics.write().await;
        
        // Calculate RPC calls saved from caching
        let cache_saves = metrics.cache.cache_hits;
        
        // Calculate RPC calls saved from batching
        let batch_saves = metrics.batching.requests_saved;
        
        // Total RPC calls saved
        let total_saved = cache_saves + batch_saves;
        metrics.performance_improvement.rpc_calls_saved = total_saved;
        
        // Estimate latency reduction (cache hits ~100ms saved, batch ~50ms saved)
        let cache_time_saved = cache_saves as f64 * 100.0; // ms
        let batch_time_saved = batch_saves as f64 * 50.0; // ms
        let total_time_saved = cache_time_saved + batch_time_saved;
        
        let total_requests = metrics.cache.total_requests.max(1);
        let avg_time_without = total_requests as f64 * 150.0; // Assume 150ms per request
        
        metrics.performance_improvement.latency_reduction_pct = 
            (total_time_saved / avg_time_without) * 100.0;
        
        // Estimate cost savings (assume $0.50 per 1M requests)
        let cost_per_request = 0.50 / 1_000_000.0;
        metrics.performance_improvement.estimated_cost_savings = 
            total_saved as f64 * cost_per_request;
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> OptimizationMetrics {
        // Calculate improvements before returning
        self.calculate_improvements().await;
        self.metrics.read().await.clone()
    }

    /// Print metrics summary
    pub async fn print_summary(&self) {
        let metrics = self.get_metrics().await;
        
        println!("\n📊 Optimization Metrics Summary");
        println!("================================");
        
        println!("\n🗄️  Cache:");
        println!("  Hit Rate: {:.1}%", metrics.cache.hit_rate * 100.0);
        println!("  Total Entries: {}", metrics.cache.total_entries);
        println!("  Memory Usage: {} KB", metrics.cache.memory_usage_bytes / 1024);
        
        println!("\n📦 Batching:");
        println!("  Total Batches: {}", metrics.batching.total_batches);
        println!("  Avg Batch Size: {:.1}", metrics.batching.avg_batch_size);
        println!("  Requests Saved: {}", metrics.batching.requests_saved);
        
        println!("\n🔮 Prediction:");
        println!("  Accuracy: {:.1}%", metrics.prediction.accuracy * 100.0);
        println!("  Avg Confidence: {:.1}%", metrics.prediction.avg_confidence * 100.0);
        
        println!("\n⚡ Performance:");
        println!("  Latency Reduction: {:.1}%", metrics.performance_improvement.latency_reduction_pct);
        println!("  RPC Calls Saved: {}", metrics.performance_improvement.rpc_calls_saved);
        println!("  Est. Cost Savings: ${:.4}", metrics.performance_improvement.estimated_cost_savings);
    }
}

impl Clone for OptimizationMetricsCollector {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
        }
    }
}

impl Default for OptimizationMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

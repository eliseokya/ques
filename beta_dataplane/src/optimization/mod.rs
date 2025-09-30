//! Performance optimization for beta dataplane
//!
//! Intelligent caching, batching, and predictive systems to maximize
//! performance despite RPC latency constraints.

pub mod caching;
pub mod batching;
pub mod prediction;
pub mod compression;
pub mod metrics;

// Re-export commonly used types
pub use caching::{IntelligentCache, CacheStrategy, CacheStats};
pub use batching::{BatchProcessor, BatchStrategy, BatchRequest, BatchResult, BatchStats};
pub use prediction::{DataPredictor, PredictionEngine, PredictionStats};
pub use compression::{DataCompressor, CompressionAlgorithm, CompressionLevel, CompressionBenefit};
pub use metrics::{OptimizationMetrics, OptimizationMetricsCollector, PerformanceImprovement};

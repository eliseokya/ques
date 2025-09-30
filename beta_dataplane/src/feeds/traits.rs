//! Traits and common types for data feeds
//!
//! Provides the same interface as the full dataplane for seamless
//! Intelligence layer integration.

use async_trait::async_trait;
use std::collections::HashMap;

use qenus_dataplane::Feature;
use crate::Result;

/// Trait for data feeds that deliver features to consumers
#[async_trait]
pub trait BetaDataFeed: Send + Sync {
    /// Get the name of this feed
    fn name(&self) -> &'static str;

    /// Start the data feed
    async fn start(&mut self) -> Result<()>;

    /// Stop the data feed
    async fn stop(&mut self) -> Result<()>;

    /// Check if the feed is running
    fn is_running(&self) -> bool;

    /// Publish a single feature
    async fn publish_feature(&self, feature: Feature) -> Result<()>;

    /// Publish multiple features in batch
    async fn publish_features(&self, features: Vec<Feature>) -> Result<()>;

    /// Get feed configuration
    fn config(&self) -> BetaFeedConfig;

    /// Update feed configuration
    async fn update_config(&mut self, config: BetaFeedConfig) -> Result<()>;

    /// Get feed metrics
    fn metrics(&self) -> BetaFeedMetrics;

    /// Get feed health status
    fn health(&self) -> BetaFeedHealth;

    /// Flush any pending data
    async fn flush(&self) -> Result<()>;
}

/// Configuration for data feeds
#[derive(Debug, Clone)]
pub struct BetaFeedConfig {
    /// Whether the feed is enabled
    pub enabled: bool,

    /// Batch size for publishing
    pub batch_size: usize,

    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,

    /// Maximum queue size
    pub max_queue_size: usize,

    /// Retry configuration
    pub retry_config: RetryConfig,

    /// Compression settings
    pub compression: CompressionConfig,

    /// Feed-specific configuration
    pub custom_config: HashMap<String, String>,
}

/// Retry configuration for feeds
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,

    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,

    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Whether compression is enabled
    pub enabled: bool,

    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,

    /// Compression level (1-9)
    pub level: u8,
}

/// Supported compression algorithms
#[derive(Debug, Clone, Copy)]
pub enum CompressionAlgorithm {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

impl Default for BetaFeedConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            batch_size: 100,
            batch_timeout_ms: 1000,
            max_queue_size: 10000,
            retry_config: RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
            },
            compression: CompressionConfig {
                enabled: true,
                algorithm: CompressionAlgorithm::Snappy,
                level: 6,
            },
            custom_config: HashMap::new(),
        }
    }
}

/// Metrics for data feeds
#[derive(Debug, Clone)]
pub struct BetaFeedMetrics {
    /// Feed name
    pub feed: String,

    /// Total features published
    pub total_published: u64,

    /// Total bytes published
    pub total_bytes_published: u64,

    /// Features published per second
    pub features_per_second: f64,

    /// Bytes published per second
    pub bytes_per_second: f64,

    /// Average publish latency in milliseconds
    pub avg_publish_latency_ms: f64,

    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,

    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,

    /// Current queue size
    pub queue_size: usize,

    /// Queue utilization (0.0 to 1.0)
    pub queue_utilization: f64,

    /// Number of batches published
    pub batches_published: u64,

    /// Average batch size
    pub avg_batch_size: f64,

    /// Compression ratio (if enabled)
    pub compression_ratio: Option<f64>,

    /// Custom metrics specific to the feed
    pub custom_metrics: HashMap<String, f64>,
}

impl BetaFeedMetrics {
    /// Create new metrics for a feed
    pub fn new(feed: String) -> Self {
        Self {
            feed,
            total_published: 0,
            total_bytes_published: 0,
            features_per_second: 0.0,
            bytes_per_second: 0.0,
            avg_publish_latency_ms: 0.0,
            success_rate: 1.0,
            error_rate: 0.0,
            queue_size: 0,
            queue_utilization: 0.0,
            batches_published: 0,
            avg_batch_size: 0.0,
            compression_ratio: None,
            custom_metrics: HashMap::new(),
        }
    }

    /// Update publish metrics
    pub fn update_publish(&mut self, features_count: u64, bytes_count: u64, latency_ms: f64) {
        self.total_published += features_count;
        self.total_bytes_published += bytes_count;

        // Update average latency using exponential moving average
        let alpha = 0.1;
        self.avg_publish_latency_ms = alpha * latency_ms + (1.0 - alpha) * self.avg_publish_latency_ms;
    }

    /// Update batch metrics
    pub fn update_batch(&mut self, batch_size: usize) {
        self.batches_published += 1;

        // Update average batch size
        let alpha = 0.1;
        self.avg_batch_size = alpha * batch_size as f64 + (1.0 - alpha) * self.avg_batch_size;
    }

    /// Update queue metrics
    pub fn update_queue(&mut self, current_size: usize, max_size: usize) {
        self.queue_size = current_size;
        self.queue_utilization = if max_size > 0 {
            current_size as f64 / max_size as f64
        } else {
            0.0
        };
    }

    /// Update success/error rates
    pub fn update_rates(&mut self, successes: u64, errors: u64) {
        let total = successes + errors;
        if total > 0 {
            self.success_rate = successes as f64 / total as f64;
            self.error_rate = errors as f64 / total as f64;
        }
    }

    /// Add custom metric
    pub fn add_custom_metric(&mut self, name: String, value: f64) {
        self.custom_metrics.insert(name, value);
    }
}

/// Health status for data feeds
#[derive(Debug, Clone)]
pub struct BetaFeedHealth {
    /// Feed name
    pub feed: String,

    /// Overall health status
    pub status: FeedStatus,

    /// Health check timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Detailed health information
    pub details: HashMap<String, String>,

    /// Last error message (if any)
    pub last_error: Option<String>,

    /// Connection status
    pub connection_status: ConnectionStatus,

    /// Performance indicators
    pub performance: FeedPerformance,
}

/// Feed health status
#[derive(Debug, Clone, Copy)]
pub enum FeedStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Disabled,
    Connecting,
}

/// Connection status for feeds
#[derive(Debug, Clone, Copy)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

/// Performance indicators for feeds
#[derive(Debug, Clone)]
pub struct FeedPerformance {
    /// Current throughput (features/second)
    pub throughput: f64,

    /// Current publish latency (ms)
    pub latency_ms: f64,

    /// Queue backlog size
    pub backlog_size: usize,

    /// Memory usage in MB
    pub memory_usage_mb: f64,

    /// Network bandwidth usage (bytes/second)
    pub bandwidth_usage: f64,
}

impl BetaFeedHealth {
    /// Create new health status for a feed
    pub fn new(feed: String) -> Self {
        Self {
            feed,
            status: FeedStatus::Healthy,
            timestamp: chrono::Utc::now(),
            details: HashMap::new(),
            last_error: None,
            connection_status: ConnectionStatus::Disconnected,
            performance: FeedPerformance {
                throughput: 0.0,
                latency_ms: 0.0,
                backlog_size: 0,
                memory_usage_mb: 0.0,
                bandwidth_usage: 0.0,
            },
        }
    }

    /// Update health status
    pub fn update_status(&mut self, status: FeedStatus, message: Option<String>) {
        self.status = status;
        self.timestamp = chrono::Utc::now();

        if let Some(msg) = message {
            match self.status {
                FeedStatus::Unhealthy | FeedStatus::Degraded => {
                    self.last_error = Some(msg);
                }
                _ => {
                    self.last_error = None;
                }
            }
        }
    }

    /// Update connection status
    pub fn update_connection(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
        self.timestamp = chrono::Utc::now();
    }

    /// Add health detail
    pub fn add_detail(&mut self, key: String, value: String) {
        self.details.insert(key, value);
        self.timestamp = chrono::Utc::now();
    }

    /// Update performance metrics
    pub fn update_performance(&mut self, performance: FeedPerformance) {
        self.performance = performance;
        self.timestamp = chrono::Utc::now();
    }

    /// Check if the feed is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, FeedStatus::Healthy)
    }

    /// Check if the feed is degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self.status, FeedStatus::Degraded)
    }

    /// Check if the feed is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self.status, FeedStatus::Unhealthy)
    }

    /// Check if the feed is disabled
    pub fn is_disabled(&self) -> bool {
        matches!(self.status, FeedStatus::Disabled)
    }

    /// Check if the feed is connected
    pub fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected)
    }
}

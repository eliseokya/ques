//! Traits and common types for feature extractors

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    types::{Block, Feature, Log, Transaction},
    Chain, Result,
};

/// Trait for feature extractors that convert raw blockchain data into features
#[async_trait]
pub trait FeatureExtractor: Send + Sync {
    /// Get the name of this extractor
    fn name(&self) -> &'static str;

    /// Get the chains this extractor supports
    fn supported_chains(&self) -> Vec<Chain>;

    /// Check if this extractor supports the given chain
    fn supports_chain(&self, chain: Chain) -> bool {
        self.supported_chains().contains(&chain)
    }

    /// Extract features from a block
    async fn extract_from_block(
        &self,
        block: &Block,
        context: &ExtractorContext,
    ) -> Result<Vec<Feature>>;

    /// Extract features from a transaction
    async fn extract_from_transaction(
        &self,
        transaction: &Transaction,
        context: &ExtractorContext,
    ) -> Result<Vec<Feature>>;

    /// Extract features from a log/event
    async fn extract_from_log(
        &self,
        log: &Log,
        context: &ExtractorContext,
    ) -> Result<Vec<Feature>>;

    /// Get extractor configuration
    fn config(&self) -> ExtractorConfig;

    /// Update extractor configuration
    async fn update_config(&mut self, config: ExtractorConfig) -> Result<()>;

    /// Get extractor metrics
    fn metrics(&self) -> ExtractorMetrics;

    /// Health check for the extractor
    async fn health_check(&self) -> Result<ExtractorHealth>;
}

/// Context provided to extractors for additional data and configuration
#[derive(Debug, Clone)]
pub struct ExtractorContext {
    /// The chain being processed
    pub chain: Chain,
    
    /// Current block number
    pub block_number: u64,
    
    /// Historical data cache (if available)
    pub cache: Option<HashMap<String, String>>,
    
    /// External data sources (price feeds, etc.)
    pub external_data: HashMap<String, f64>,
    
    /// Configuration overrides
    pub config_overrides: HashMap<String, String>,
}

impl ExtractorContext {
    /// Create a new extractor context
    pub fn new(chain: Chain, block_number: u64) -> Self {
        Self {
            chain,
            block_number,
            cache: None,
            external_data: HashMap::new(),
            config_overrides: HashMap::new(),
        }
    }

    /// Add external data (e.g., token prices)
    pub fn with_external_data(mut self, key: String, value: f64) -> Self {
        self.external_data.insert(key, value);
        self
    }

    /// Add configuration override
    pub fn with_config_override(mut self, key: String, value: String) -> Self {
        self.config_overrides.insert(key, value);
        self
    }

    /// Get external data value
    pub fn get_external_data(&self, key: &str) -> Option<f64> {
        self.external_data.get(key).copied()
    }

    /// Get configuration override
    pub fn get_config_override(&self, key: &str) -> Option<&String> {
        self.config_overrides.get(key)
    }
}

/// Result from feature extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorResult {
    /// Extracted features
    pub features: Vec<Feature>,
    
    /// Extraction metadata
    pub metadata: ExtractorMetadata,
    
    /// Any warnings or issues during extraction
    pub warnings: Vec<String>,
}

/// Metadata about the extraction process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorMetadata {
    /// Extractor name
    pub extractor: String,
    
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
    
    /// Number of input items processed
    pub input_count: usize,
    
    /// Number of features extracted
    pub output_count: usize,
    
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
}

/// Configuration for feature extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    /// Whether the extractor is enabled
    pub enabled: bool,
    
    /// Processing batch size
    pub batch_size: usize,
    
    /// Processing timeout in seconds
    pub timeout_seconds: u64,
    
    /// Minimum confidence threshold for features
    pub min_confidence: f64,
    
    /// Maximum age of cached data in seconds
    pub max_cache_age_seconds: u64,
    
    /// Extractor-specific configuration
    pub custom_config: HashMap<String, String>,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            batch_size: 100,
            timeout_seconds: 30,
            min_confidence: 0.8,
            max_cache_age_seconds: 300,
            custom_config: HashMap::new(),
        }
    }
}

/// Metrics for feature extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorMetrics {
    /// Extractor name
    pub extractor: String,
    
    /// Total number of items processed
    pub total_processed: u64,
    
    /// Total number of features extracted
    pub total_features_extracted: u64,
    
    /// Average processing time per item (ms)
    pub avg_processing_time_ms: f64,
    
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    
    /// Current queue size
    pub queue_size: usize,
    
    /// Memory usage in MB
    pub memory_usage_mb: f64,
    
    /// Custom metrics specific to the extractor
    pub custom_metrics: HashMap<String, f64>,
}

impl ExtractorMetrics {
    /// Create new metrics for an extractor
    pub fn new(extractor: String) -> Self {
        Self {
            extractor,
            total_processed: 0,
            total_features_extracted: 0,
            avg_processing_time_ms: 0.0,
            success_rate: 1.0,
            error_rate: 0.0,
            cache_hit_rate: 0.0,
            queue_size: 0,
            memory_usage_mb: 0.0,
            custom_metrics: HashMap::new(),
        }
    }

    /// Update processing metrics
    pub fn update_processing(&mut self, processed: u64, features_extracted: u64, processing_time_ms: f64) {
        self.total_processed += processed;
        self.total_features_extracted += features_extracted;
        
        // Update average processing time using exponential moving average
        let alpha = 0.1;
        self.avg_processing_time_ms = alpha * processing_time_ms + (1.0 - alpha) * self.avg_processing_time_ms;
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

/// Health status for feature extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorHealth {
    /// Extractor name
    pub extractor: String,
    
    /// Overall health status
    pub status: ExtractorStatus,
    
    /// Health check timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Detailed health information
    pub details: HashMap<String, String>,
    
    /// Last error message (if any)
    pub last_error: Option<String>,
    
    /// Performance indicators
    pub performance: ExtractorPerformance,
}

/// Extractor health status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Disabled,
}

/// Performance indicators for extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorPerformance {
    /// Current throughput (items/second)
    pub throughput: f64,
    
    /// Current latency (ms)
    pub latency_ms: f64,
    
    /// Queue backlog size
    pub backlog_size: usize,
    
    /// Memory pressure (0.0 to 1.0)
    pub memory_pressure: f64,
    
    /// CPU usage (0.0 to 1.0)
    pub cpu_usage: f64,
}

impl ExtractorHealth {
    /// Create new health status for an extractor
    pub fn new(extractor: String) -> Self {
        Self {
            extractor,
            status: ExtractorStatus::Healthy,
            timestamp: chrono::Utc::now(),
            details: HashMap::new(),
            last_error: None,
            performance: ExtractorPerformance {
                throughput: 0.0,
                latency_ms: 0.0,
                backlog_size: 0,
                memory_pressure: 0.0,
                cpu_usage: 0.0,
            },
        }
    }

    /// Update health status
    pub fn update_status(&mut self, status: ExtractorStatus, message: Option<String>) {
        self.status = status;
        self.timestamp = chrono::Utc::now();
        
        if let Some(msg) = message {
            match self.status {
                ExtractorStatus::Unhealthy | ExtractorStatus::Degraded => {
                    self.last_error = Some(msg);
                }
                _ => {
                    self.last_error = None;
                }
            }
        }
    }

    /// Add health detail
    pub fn add_detail(&mut self, key: String, value: String) {
        self.details.insert(key, value);
        self.timestamp = chrono::Utc::now();
    }

    /// Update performance metrics
    pub fn update_performance(&mut self, performance: ExtractorPerformance) {
        self.performance = performance;
        self.timestamp = chrono::Utc::now();
    }

    /// Check if the extractor is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, ExtractorStatus::Healthy)
    }

    /// Check if the extractor is degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self.status, ExtractorStatus::Degraded)
    }

    /// Check if the extractor is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self.status, ExtractorStatus::Unhealthy)
    }

    /// Check if the extractor is disabled
    pub fn is_disabled(&self) -> bool {
        matches!(self.status, ExtractorStatus::Disabled)
    }
}

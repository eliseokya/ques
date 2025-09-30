//! Traits and common types for feature extractors
//!
//! Defines the interface for RPC-based feature extraction that produces
//! the same output schema as the full dataplane.

use async_trait::async_trait;
use std::collections::HashMap;

use qenus_dataplane::{Feature, FeatureData, FeatureType};
use crate::{Chain, Result};

/// Trait for feature extractors that work via RPC
#[async_trait]
pub trait BetaFeatureExtractor: Send + Sync {
    /// Get the name of this extractor
    fn name(&self) -> &'static str;

    /// Get the feature type this extractor produces
    fn feature_type(&self) -> FeatureType;

    /// Get the chains this extractor supports
    fn supported_chains(&self) -> Vec<Chain>;

    /// Check if this extractor supports the given chain
    fn supports_chain(&self, chain: Chain) -> bool {
        self.supported_chains().contains(&chain)
    }

    /// Extract features for a specific block
    async fn extract_for_block(
        &self,
        chain: Chain,
        block_number: u64,
        context: &ExtractionContext,
    ) -> Result<Vec<Feature>>;

    /// Extract features for the latest block
    async fn extract_latest(
        &self,
        chain: Chain,
        context: &ExtractionContext,
    ) -> Result<Vec<Feature>>;

    /// Get extractor configuration
    fn config(&self) -> ExtractorConfig;

    /// Update extractor configuration
    async fn update_config(&mut self, config: ExtractorConfig) -> Result<()>;
}

/// Context provided to extractors for additional data and configuration
#[derive(Debug, Clone)]
pub struct ExtractionContext {
    /// The chain being processed
    pub chain: Chain,
    
    /// Current block number
    pub block_number: u64,
    
    /// RPC provider client (opaque handle)
    pub provider_handle: String,
    
    /// External data cache (price feeds, etc.)
    pub external_data: HashMap<String, f64>,
    
    /// Configuration overrides
    pub config_overrides: HashMap<String, String>,
    
    /// Cache for intermediate calculations
    pub cache: Option<HashMap<String, String>>,
}

impl ExtractionContext {
    /// Create a new extraction context
    pub fn new(chain: Chain, block_number: u64, provider_handle: String) -> Self {
        Self {
            chain,
            block_number,
            provider_handle,
            external_data: HashMap::new(),
            config_overrides: HashMap::new(),
            cache: None,
        }
    }

    /// Add external data (e.g., token prices from oracle)
    pub fn with_external_data(mut self, key: String, value: f64) -> Self {
        self.external_data.insert(key, value);
        self
    }

    /// Add configuration override
    pub fn with_config_override(mut self, key: String, value: String) -> Self {
        self.config_overrides.insert(key, value);
        self
    }

    /// Enable caching
    pub fn with_cache(mut self) -> Self {
        self.cache = Some(HashMap::new());
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

    /// Get cached value
    pub fn get_cached(&self, key: &str) -> Option<&String> {
        self.cache.as_ref().and_then(|cache| cache.get(key))
    }

    /// Set cached value
    pub fn set_cached(&mut self, key: String, value: String) {
        if let Some(cache) = &mut self.cache {
            cache.insert(key, value);
        }
    }
}

/// Configuration for feature extractors
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    /// Whether the extractor is enabled
    pub enabled: bool,
    
    /// Update frequency in seconds
    pub update_frequency_seconds: u64,
    
    /// Processing timeout in seconds
    pub timeout_seconds: u64,
    
    /// Minimum confidence threshold
    pub min_confidence: f64,
    
    /// Batch size for processing
    pub batch_size: usize,
    
    /// Extractor-specific configuration
    pub custom_config: HashMap<String, String>,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_frequency_seconds: 1,
            timeout_seconds: 30,
            min_confidence: 0.8,
            batch_size: 100,
            custom_config: HashMap::new(),
        }
    }
}

/// Result from feature extraction
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Extracted features
    pub features: Vec<Feature>,
    
    /// Extraction metadata
    pub metadata: ExtractionMetadata,
    
    /// Any warnings during extraction
    pub warnings: Vec<String>,
}

/// Metadata about the extraction process
#[derive(Debug, Clone)]
pub struct ExtractionMetadata {
    /// Extractor name
    pub extractor: String,
    
    /// Chain processed
    pub chain: Chain,
    
    /// Block number processed
    pub block_number: u64,
    
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
    
    /// Number of features extracted
    pub feature_count: usize,
    
    /// Success status
    pub success: bool,
    
    /// Provider used
    pub provider: String,
}

impl ExtractionResult {
    /// Create a new extraction result
    pub fn new(
        extractor: String,
        chain: Chain,
        block_number: u64,
        features: Vec<Feature>,
        processing_time_ms: f64,
        provider: String,
    ) -> Self {
        let feature_count = features.len();
        
        Self {
            features,
            metadata: ExtractionMetadata {
                extractor,
                chain,
                block_number,
                processing_time_ms,
                feature_count,
                success: true,
                provider,
            },
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Check if extraction was successful
    pub fn is_success(&self) -> bool {
        self.metadata.success
    }

    /// Get feature count
    pub fn feature_count(&self) -> usize {
        self.metadata.feature_count
    }
}

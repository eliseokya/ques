//! Canonical bridge extractor
//!
//! Extracts state from canonical L1-L2 bridges (Arbitrum, Optimism, Base).

use async_trait::async_trait;
use tracing::debug;

use qenus_dataplane::{Feature, FeatureType};

use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    Chain, Result,
};

/// Canonical bridge extractor
pub struct CanonicalBridgeExtractor {
    config: ExtractorConfig,
}

impl CanonicalBridgeExtractor {
    pub fn new(config: ExtractorConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl BetaFeatureExtractor for CanonicalBridgeExtractor {
    fn name(&self) -> &'static str {
        "canonical_bridge"
    }

    fn feature_type(&self) -> FeatureType {
        FeatureType::Bridge
    }

    fn supported_chains(&self) -> Vec<Chain> {
        vec![Chain::Ethereum, Chain::Arbitrum, Chain::Optimism, Chain::Base]
    }

    async fn extract_for_block(
        &self,
        chain: Chain,
        block_number: u64,
        _context: &ExtractionContext,
    ) -> Result<Vec<Feature>> {
        debug!(chain = %chain, block = block_number, "Extracting canonical bridge features (placeholder)");
        // TODO: Implement bridge extraction
        Ok(Vec::new())
    }

    async fn extract_latest(&self, chain: Chain, context: &ExtractionContext) -> Result<Vec<Feature>> {
        self.extract_for_block(chain, context.block_number, context).await
    }

    fn config(&self) -> ExtractorConfig {
        self.config.clone()
    }

    async fn update_config(&mut self, config: ExtractorConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }
}

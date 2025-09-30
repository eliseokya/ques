//! Across Protocol bridge extractor

use async_trait::async_trait;
use tracing::debug;
use qenus_dataplane::{Feature, FeatureType};
use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    Chain, Result,
};

pub struct AcrossBridgeExtractor {
    config: ExtractorConfig,
}

impl AcrossBridgeExtractor {
    pub fn new(config: ExtractorConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl BetaFeatureExtractor for AcrossBridgeExtractor {
    fn name(&self) -> &'static str { "across_bridge" }
    fn feature_type(&self) -> FeatureType { FeatureType::Bridge }
    fn supported_chains(&self) -> Vec<Chain> {
        vec![Chain::Ethereum, Chain::Arbitrum, Chain::Optimism, Chain::Base]
    }
    async fn extract_for_block(&self, chain: Chain, block_number: u64, _context: &ExtractionContext) -> Result<Vec<Feature>> {
        debug!(chain = %chain, block = block_number, "Extracting Across bridge features (placeholder)");
        Ok(Vec::new())
    }
    async fn extract_latest(&self, chain: Chain, context: &ExtractionContext) -> Result<Vec<Feature>> {
        self.extract_for_block(chain, context.block_number, context).await
    }
    fn config(&self) -> ExtractorConfig { self.config.clone() }
    async fn update_config(&mut self, config: ExtractorConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }
}

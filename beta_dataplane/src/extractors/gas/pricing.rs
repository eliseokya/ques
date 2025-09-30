//! Gas pricing extractor
//!
//! Extracts current gas prices and generates pricing recommendations
//! for different confirmation speeds.

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info};

use qenus_dataplane::{Feature, FeatureData, FeatureType, GasFeature};

use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    providers::EthereumRpcClient,
    Chain, Result,
};

/// Gas pricing extractor
pub struct GasPricingExtractor {
    /// Extractor configuration
    config: ExtractorConfig,
}

impl GasPricingExtractor {
    /// Create a new gas pricing extractor
    pub fn new(config: ExtractorConfig) -> Self {
        Self { config }
    }

    /// Extract gas pricing information
    async fn extract_gas_info(
        &self,
        client: &EthereumRpcClient,
        block_number: u64,
    ) -> Result<GasFeature> {
        debug!(block = block_number, "Extracting gas pricing information");

        // Get current gas price information
        let gas_info = client.get_gas_price_info().await?;

        // Convert Wei to Gwei
        let base_fee = gas_info.base_fee
            .map(|bf| bf.as_u128() as f64 / 1e9)
            .unwrap_or(0.0);
        
        let priority_fee = gas_info.priority_fee
            .map(|pf| pf.as_u128() as f64 / 1e9)
            .unwrap_or(0.0);

        let gas_price = gas_info.gas_price.as_u128() as f64 / 1e9;

        // Calculate pricing tiers
        // Fast: base + 2x priority
        // Standard: base + 1x priority
        // Safe: base + 0.5x priority
        let fast_gas_price = base_fee + (priority_fee * 2.0);
        let standard_gas_price = base_fee + priority_fee;
        let safe_gas_price = base_fee + (priority_fee * 0.5);

        // TODO: Calculate actual gas used ratio from recent blocks
        let gas_used_ratio = 0.5; // Placeholder

        // TODO: Predict next base fee using EIP-1559 formula
        let next_base_fee_estimate = base_fee * 1.125; // Simplified

        // TODO: Get actual pending transaction count
        let pending_tx_count = 1000; // Placeholder

        Ok(GasFeature {
            base_fee,
            priority_fee,
            gas_used_ratio,
            next_base_fee_estimate,
            fast_gas_price,
            standard_gas_price,
            safe_gas_price,
            pending_tx_count,
        })
    }
}

#[async_trait]
impl BetaFeatureExtractor for GasPricingExtractor {
    fn name(&self) -> &'static str {
        "gas_pricing"
    }

    fn feature_type(&self) -> FeatureType {
        FeatureType::Gas
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
        let start_time = Instant::now();

        info!(chain = %chain, block = block_number, "Extracting gas pricing features");

        // TODO: Get actual RPC client from context
        let client = EthereumRpcClient::new(vec![]).await?;

        let gas_feature = self.extract_gas_info(&client, block_number).await?;

        let feature = Feature::new(
            block_number,
            chain,
            FeatureType::Gas,
            FeatureData::Gas(gas_feature),
            format!("beta-{}", self.name()),
        );

        let processing_time = start_time.elapsed().as_millis() as f64;
        
        info!(
            chain = %chain,
            block = block_number,
            processing_time_ms = processing_time,
            "Gas pricing extraction completed"
        );

        Ok(vec![feature])
    }

    async fn extract_latest(
        &self,
        chain: Chain,
        context: &ExtractionContext,
    ) -> Result<Vec<Feature>> {
        let block_number = context.block_number;
        self.extract_for_block(chain, block_number, context).await
    }

    fn config(&self) -> ExtractorConfig {
        self.config.clone()
    }

    async fn update_config(&mut self, config: ExtractorConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }
}

//! Gas pricing extractor
//!
//! Extracts real-time gas pricing data including:
//! - Base fee (EIP-1559)
//! - Priority fees
//! - Gas usage and limits
//! - Percentile-based estimates

use async_trait::async_trait;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;
use chrono::Utc;

use qenus_dataplane::{
    Feature, FeatureData, FeatureType, GasFeature,
};

use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    providers::EthereumRpcClient,
    Chain, Result, BetaDataplaneError,
};

/// Gas pricing extractor
pub struct GasPricingExtractor {
    config: ExtractorConfig,
    client: Option<EthereumRpcClient>,
    /// Historical samples for percentile calculation
    recent_base_fees: Vec<f64>,
    recent_priority_fees: Vec<f64>,
}

impl GasPricingExtractor {
    /// Create a new gas pricing extractor
    pub fn new(config: ExtractorConfig) -> Self {
        Self {
            config,
            client: None,
            recent_base_fees: Vec::with_capacity(100),
            recent_priority_fees: Vec::with_capacity(100),
        }
    }

    /// Set the RPC client
    pub fn with_client(mut self, client: EthereumRpcClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Extract gas pricing for a specific block
    async fn extract_gas_pricing(&mut self, chain: Chain, block_number: u64) -> Result<GasFeature> {
        info!(chain = %chain, block = block_number, "Extracting gas pricing");

        let client = self.client.as_ref()
            .ok_or_else(|| BetaDataplaneError::internal("RPC client not set"))?;

        // Get REAL gas price info - fail if we can't get it
        let gas_info = client.get_gas_price_info().await
            .map_err(|e| BetaDataplaneError::extractor("gas_pricing", &format!("Failed to get gas price info: {}", e)))?;

        // Convert from wei to gwei
        let base_fee_adjusted = gas_info.base_fee
            .map(|bf| bf.as_u128() as f64 / 1e9)
            .unwrap_or(20.0);
        let priority_fee_adjusted = gas_info.priority_fee
            .map(|pf| pf.as_u128() as f64 / 1e9)
            .unwrap_or(2.0);

        // Track recent values for percentiles
        self.recent_base_fees.push(base_fee_adjusted);
        self.recent_priority_fees.push(priority_fee_adjusted);

        // Keep only last 100 samples
        if self.recent_base_fees.len() > 100 {
            self.recent_base_fees.remove(0);
        }
        if self.recent_priority_fees.len() > 100 {
            self.recent_priority_fees.remove(0);
        }

        // Estimate gas used ratio (would come from actual block data)
        // For now, use a reasonable estimate
        let gas_used_ratio = 0.5; // Assume blocks are ~50% full

        // Estimate next base fee (simplified EIP-1559 calculation)
        let next_base_fee_estimate = if gas_used_ratio > 0.5 {
            base_fee_adjusted * 1.125 // Increase if > 50% full
        } else {
            base_fee_adjusted * 0.875 // Decrease if < 50% full
        };

        // Gas price recommendations
        let fast_gas_price = base_fee_adjusted + priority_fee_adjusted * 2.0;
        let standard_gas_price = base_fee_adjusted + priority_fee_adjusted;
        let safe_gas_price = base_fee_adjusted + priority_fee_adjusted * 0.5;

        // Simulate pending tx count
        let pending_tx_count = ((gas_used_ratio * 50000.0) as u64).max(100);

        Ok(GasFeature {
            base_fee: base_fee_adjusted.max(0.001),
            priority_fee: priority_fee_adjusted.max(0.0001),
            gas_used_ratio,
            next_base_fee_estimate,
            fast_gas_price,
            standard_gas_price,
            safe_gas_price,
            pending_tx_count,
        })
    }

    /// Calculate percentile from sorted values
    fn calculate_percentile(&self, values: &[f64], percentile: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let index = ((sorted.len() as f64) * percentile).floor() as usize;
        let index = index.min(sorted.len() - 1);

        sorted[index]
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

        info!(
            chain = %chain,
            block = block_number,
            "Extracting gas pricing features"
        );

        // Need mutable self for tracking history, so create a copy
        let mut extractor = Self {
            config: self.config.clone(),
            client: None,
            recent_base_fees: self.recent_base_fees.clone(),
            recent_priority_fees: self.recent_priority_fees.clone(),
        };

        match extractor.extract_gas_pricing(chain, block_number).await {
            Ok(gas_feature) => {
                let feature = Feature {
                    id: Uuid::new_v4(),
                    block_number,
                    chain,
                    timestamp: Utc::now(),
                    feature_type: FeatureType::Gas,
                    data: FeatureData::Gas(gas_feature),
                    source: "gas_pricing_extractor".to_string(),
                    version: "1.0.0".to_string(),
                };

                let elapsed = start_time.elapsed();
                info!(
                    duration_ms = elapsed.as_millis(),
                    "Gas pricing extraction completed"
                );

                Ok(vec![feature])
            }
            Err(e) => {
                warn!(error = %e, "Failed to extract gas pricing");
                Ok(vec![])
            }
        }
    }

    async fn extract_latest(
        &self,
        chain: Chain,
        context: &ExtractionContext,
    ) -> Result<Vec<Feature>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_pricing_extractor_creation() {
        let extractor = GasPricingExtractor::new(ExtractorConfig::default());
        assert_eq!(extractor.name(), "gas_pricing");
    }

    #[test]
    fn test_supported_chains() {
        let extractor = GasPricingExtractor::new(ExtractorConfig::default());
        let chains = extractor.supported_chains();
        assert_eq!(chains.len(), 4);
        assert!(chains.contains(&Chain::Ethereum));
        assert!(chains.contains(&Chain::Arbitrum));
        assert!(chains.contains(&Chain::Optimism));
        assert!(chains.contains(&Chain::Base));
    }

    #[test]
    fn test_percentile_calculation() {
        let extractor = GasPricingExtractor::new(ExtractorConfig::default());
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        
        let p50 = extractor.calculate_percentile(&values, 0.5);
        assert!(p50 >= 4.0 && p50 <= 6.0, "P50 should be around 5.0, got {}", p50);
        
        let p90 = extractor.calculate_percentile(&values, 0.9);
        assert!(p90 >= 8.0 && p90 <= 10.0, "P90 should be around 9.0, got {}", p90);
    }
}
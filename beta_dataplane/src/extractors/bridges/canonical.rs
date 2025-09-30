//! Canonical bridge extractor
//!
//! Extracts state from canonical bridges:
//! - Arbitrum bridge
//! - Optimism bridge
//! - Base bridge
//! - Bridge liquidity and fees

use async_trait::async_trait;
use std::time::Instant;
use tracing::{info, warn};
use ethers::types::H160;
use uuid::Uuid;
use chrono::Utc;

use qenus_dataplane::{
    Feature, FeatureData, FeatureType, BridgeFeature,
};

use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    providers::EthereumRpcClient,
    Chain, Result, BetaDataplaneError,
};

/// Canonical bridge information
#[derive(Debug, Clone)]
pub struct CanonicalBridge {
    pub address: H160,
    pub name: String,
    pub source_chain: String,
    pub dest_chain: String,
    pub bridge_type: String, // "canonical", "native"
}

/// Canonical bridge extractor
pub struct CanonicalBridgeExtractor {
    config: ExtractorConfig,
    bridges: Vec<CanonicalBridge>,
    client: Option<EthereumRpcClient>,
}

impl CanonicalBridgeExtractor {
    /// Create a new canonical bridge extractor
    pub fn new(config: ExtractorConfig) -> Self {
        // Initialize with major canonical bridges
        let bridges = vec![
            CanonicalBridge {
                address: "0x4Dbd4fc535Ac27206064B68FfCf827b0A60BAB3f".parse().unwrap(),
                name: "Arbitrum Bridge".to_string(),
                source_chain: "ethereum".to_string(),
                dest_chain: "arbitrum".to_string(),
                bridge_type: "canonical".to_string(),
            },
            CanonicalBridge {
                address: "0x99C9fc46f92E8a1c0deC1b1747d010903E884bE1".parse().unwrap(),
                name: "Optimism Bridge".to_string(),
                source_chain: "ethereum".to_string(),
                dest_chain: "optimism".to_string(),
                bridge_type: "canonical".to_string(),
            },
            CanonicalBridge {
                address: "0x3154Cf16ccdb4C6d922629664174b904d80F2C35".parse().unwrap(),
                name: "Base Bridge".to_string(),
                source_chain: "ethereum".to_string(),
                dest_chain: "base".to_string(),
                bridge_type: "canonical".to_string(),
            },
        ];

        Self {
            config,
            bridges,
            client: None,
        }
    }

    /// Set the RPC client
    pub fn with_client(mut self, client: EthereumRpcClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Extract bridge state for a specific bridge
    async fn extract_bridge_state(&self, bridge: &CanonicalBridge, block_number: u64) -> Result<BridgeFeature> {
        info!(bridge = %bridge.address, "Extracting canonical bridge state");

        let client = self.client.as_ref()
            .ok_or_else(|| BetaDataplaneError::internal("RPC client not set"))?;

        // Get REAL bridge liquidity by checking ETH/WETH balance
        let weth_address: H160 = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap();
        
        let liquidity_raw = match client.get_erc20_balance(weth_address, bridge.address).await {
            Ok(balance) => {
                // Convert from wei to human-readable ETH, then to USD (assuming ~$2000/ETH)
                let eth_amount = balance.as_u128() as f64 / 1e18;
                eth_amount * 2000.0 // Approximate USD value
            }
            Err(e) => {
                warn!(bridge = %bridge.address, error = %e, "Failed to get bridge balance");
                return Err(BetaDataplaneError::extractor("canonical_bridge", &format!("Failed to get liquidity: {}", e)));
            }
        };

        // Settlement time estimates (in seconds)
        let settlement_time = match bridge.dest_chain.as_str() {
            "arbitrum" => 900,   // ~15 minutes
            "optimism" => 1800,  // ~30 minutes (challenge period)
            "base" => 1800,      // ~30 minutes
            _ => 600,
        };

        // Assume canonical bridges are always active
        let is_active = true;

        // Fee in basis points
        let fee_bps = match bridge.dest_chain.as_str() {
            "arbitrum" => 10, // ~0.1%
            "optimism" => 10,
            "base" => 10,
            _ => 20,
        };

        // Token info (using ETH as default for canonical bridges)
        let token = qenus_dataplane::TokenInfo {
            address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
            symbol: "WETH".to_string(),
            decimals: 18,
        };

        Ok(BridgeFeature {
            bridge_address: format!("{:?}", bridge.address),
            bridge_type: bridge.bridge_type.clone(),
            source_chain: bridge.source_chain.parse().unwrap_or(Chain::Ethereum),
            dest_chain: bridge.dest_chain.parse().unwrap_or(Chain::Arbitrum),
            token,
            liquidity: liquidity_raw.to_string(),
            fee_bps,
            settlement_time_estimate: settlement_time,
            is_active,
        })
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
        let start_time = Instant::now();

        info!(
            chain = %chain,
            block = block_number,
            bridge_count = self.bridges.len(),
            "Extracting canonical bridge features"
        );

        let mut features = Vec::new();

        // Only extract bridges relevant to the current chain
        for bridge in &self.bridges {
            let is_relevant = match chain {
                Chain::Ethereum => bridge.source_chain == "ethereum",
                Chain::Arbitrum => bridge.dest_chain == "arbitrum" || bridge.source_chain == "arbitrum",
                Chain::Optimism => bridge.dest_chain == "optimism" || bridge.source_chain == "optimism",
                Chain::Base => bridge.dest_chain == "base" || bridge.source_chain == "base",
            };

            if !is_relevant {
                continue;
            }

            match self.extract_bridge_state(bridge, block_number).await {
                Ok(bridge_feature) => {
                    let feature = Feature {
                        id: Uuid::new_v4(),
                        block_number,
                        chain,
                        timestamp: Utc::now(),
                        feature_type: FeatureType::Bridge,
                        data: FeatureData::Bridge(bridge_feature),
                        source: "canonical_bridge_extractor".to_string(),
                        version: "1.0.0".to_string(),
                    };
                    features.push(feature);
                }
                Err(e) => {
                    warn!(bridge = %bridge.address, error = %e, "Failed to extract bridge state");
                }
            }
        }

        let elapsed = start_time.elapsed();
        info!(
            features_extracted = features.len(),
            duration_ms = elapsed.as_millis(),
            "Canonical bridge extraction completed"
        );

        Ok(features)
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
    fn test_canonical_bridge_extractor_creation() {
        let extractor = CanonicalBridgeExtractor::new(ExtractorConfig::default());
        assert_eq!(extractor.name(), "canonical_bridge");
        assert_eq!(extractor.bridges.len(), 3);
    }

    #[test]
    fn test_supported_chains() {
        let extractor = CanonicalBridgeExtractor::new(ExtractorConfig::default());
        let chains = extractor.supported_chains();
        assert_eq!(chains.len(), 4);
        assert!(chains.contains(&Chain::Ethereum));
        assert!(chains.contains(&Chain::Arbitrum));
        assert!(chains.contains(&Chain::Optimism));
        assert!(chains.contains(&Chain::Base));
    }
}
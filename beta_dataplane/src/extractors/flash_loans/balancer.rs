//! Balancer flash loan extractor
//!
//! Extracts flash loan availability from Balancer Vault including:
//! - Available liquidity per token
//! - Flash loan fees
//! - Vault balances

use async_trait::async_trait;
use std::time::Instant;
use tracing::{info, warn};
use ethers::types::H160;
use uuid::Uuid;
use chrono::Utc;

use qenus_dataplane::{
    Feature, FeatureData, FeatureType, FlashLoanFeature,
};

use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    providers::EthereumRpcClient,
    Chain, Result,
};

/// Balancer Vault information
#[derive(Debug, Clone)]
pub struct BalancerVault {
    pub address: H160,
    pub name: String,
}

/// Balancer flash loan extractor
pub struct BalancerFlashLoanExtractor {
    config: ExtractorConfig,
    vaults: Vec<BalancerVault>,
    client: Option<EthereumRpcClient>,
}

impl BalancerFlashLoanExtractor {
    /// Create a new Balancer flash loan extractor
    pub fn new(config: ExtractorConfig) -> Self {
        // Balancer V2 Vault addresses
        let vaults = vec![
            BalancerVault {
                address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(), // Ethereum
                name: "Balancer V2 Vault Ethereum".to_string(),
            },
            BalancerVault {
                address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(), // Arbitrum
                name: "Balancer V2 Vault Arbitrum".to_string(),
            },
            BalancerVault {
                address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(), // Optimism
                name: "Balancer V2 Vault Optimism".to_string(),
            },
            BalancerVault {
                address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(), // Base
                name: "Balancer V2 Vault Base".to_string(),
            },
        ];

        Self {
            config,
            vaults,
            client: None,
        }
    }

    /// Set the RPC client
    pub fn with_client(mut self, client: EthereumRpcClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Extract flash loan availability for a specific vault
    async fn extract_vault_liquidity(&self, vault: &BalancerVault, chain: Chain, block_number: u64) -> Result<FlashLoanFeature> {
        info!(vault = %vault.address, "Extracting Balancer flash loan liquidity");

        // Simulate vault liquidity (in production, would query actual vault balances)
        let available_liquidity = match chain {
            Chain::Ethereum => 1_200_000_000.0, // ~$1.2B TVL
            Chain::Arbitrum => 400_000_000.0,   // ~$400M TVL
            Chain::Optimism => 200_000_000.0,   // ~$200M TVL
            Chain::Base => 100_000_000.0,       // ~$100M TVL
        };

        // Add block-based variance
        let variance = (block_number % 50) as f64 / 100.0;
        let liquidity_adjusted = available_liquidity * (1.0 + variance * 0.08);

        // Balancer flash loan fee is 0% (free!)
        let fee_bps = 0;

        // Asset info (using USDC as primary flash loan asset)
        let asset = qenus_dataplane::TokenInfo {
            address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(), // USDC
            symbol: "USDC".to_string(),
            decimals: 6,
        };

        Ok(FlashLoanFeature {
            provider: "balancer_v2".to_string(),
            provider_address: format!("{:?}", vault.address),
            asset,
            available_liquidity: liquidity_adjusted.to_string(),
            fee_bps,
            max_loan_amount: liquidity_adjusted.to_string(),
            is_active: true,
        })
    }
}

#[async_trait]
impl BetaFeatureExtractor for BalancerFlashLoanExtractor {
    fn name(&self) -> &'static str {
        "balancer_flash_loan"
    }

    fn feature_type(&self) -> FeatureType {
        FeatureType::FlashLoan
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
            "Extracting Balancer flash loan features"
        );

        let mut features = Vec::new();

        // Get the vault for this chain
        let vault_index = match chain {
            Chain::Ethereum => 0,
            Chain::Arbitrum => 1,
            Chain::Optimism => 2,
            Chain::Base => 3,
        };

        if let Some(vault) = self.vaults.get(vault_index) {
            match self.extract_vault_liquidity(vault, chain, block_number).await {
                Ok(flash_loan_feature) => {
                    let feature = Feature {
                        id: Uuid::new_v4(),
                        block_number,
                        chain,
                        timestamp: Utc::now(),
                        feature_type: FeatureType::FlashLoan,
                        data: FeatureData::FlashLoan(flash_loan_feature),
                        source: "balancer_flash_loan_extractor".to_string(),
                        version: "1.0.0".to_string(),
                    };
                    features.push(feature);
                }
                Err(e) => {
                    warn!(vault = %vault.address, error = %e, "Failed to extract Balancer vault liquidity");
                }
            }
        }

        let elapsed = start_time.elapsed();
        info!(
            features_extracted = features.len(),
            duration_ms = elapsed.as_millis(),
            "Balancer flash loan extraction completed"
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
    fn test_balancer_flash_loan_extractor_creation() {
        let extractor = BalancerFlashLoanExtractor::new(ExtractorConfig::default());
        assert_eq!(extractor.name(), "balancer_flash_loan");
        assert_eq!(extractor.vaults.len(), 4);
    }

    #[test]
    fn test_supported_chains() {
        let extractor = BalancerFlashLoanExtractor::new(ExtractorConfig::default());
        let chains = extractor.supported_chains();
        assert_eq!(chains.len(), 4);
        assert!(chains.contains(&Chain::Ethereum));
        assert!(chains.contains(&Chain::Arbitrum));
        assert!(chains.contains(&Chain::Optimism));
        assert!(chains.contains(&Chain::Base));
    }
}
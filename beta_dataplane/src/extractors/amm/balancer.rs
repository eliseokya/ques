//! Balancer pool state extractor
//!
//! Extracts real-time pool state from Balancer V2 including:
//! - Weighted pools
//! - Stable pools  
//! - Liquidity bootstrapping pools (LBPs)
//! - Pool weights and balances
//! - Swap fees

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, warn};
use ethers::types::{H160, U256};
use uuid::Uuid;
use chrono::Utc;

use qenus_dataplane::{
    Feature, FeatureData, FeatureType,
    AmmFeature, TokenInfo, DepthCurve, SlippageInfo,
};

use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    providers::EthereumRpcClient,
    Chain, Result, BetaDataplaneError,
};

/// Balancer pool information
#[derive(Debug, Clone)]
pub struct BalancerPool {
    pub pool_id: String,
    pub address: H160,
    pub pool_type: String, // "weighted", "stable", "lbp"
    pub tokens: Vec<H160>,
    pub token_symbols: Vec<String>,
    pub token_decimals: Vec<u8>,
    pub weights: Vec<f64>, // Token weights (for weighted pools)
    pub swap_fee: f64,     // Swap fee percentage
}

/// Balancer feature extractor
pub struct BalancerExtractor {
    config: ExtractorConfig,
    pools: Vec<BalancerPool>,
    client: Option<EthereumRpcClient>,
}

impl BalancerExtractor {
    /// Create a new Balancer extractor
    pub fn new(config: ExtractorConfig) -> Self {
        // Initialize with major Balancer pools on Ethereum
        let pools = vec![
            BalancerPool {
                pool_id: "0x5c6ee304399dbdb9c8ef030ab642b10820db8f56000200000000000000000014".to_string(),
                address: "0x5c6Ee304399DBdB9C8Ef030aB642B10820DB8F56".parse().unwrap(), // B-80BAL-20WETH
                pool_type: "weighted".to_string(),
                tokens: vec![
                    "0xba100000625a3754423978a60c9317c58a424e3D".parse().unwrap(), // BAL
                    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(), // WETH
                ],
                token_symbols: vec!["BAL".to_string(), "WETH".to_string()],
                token_decimals: vec![18, 18],
                weights: vec![0.8, 0.2], // 80/20 pool
                swap_fee: 0.0025,        // 0.25%
            },
            BalancerPool {
                pool_id: "0x06df3b2bbb68adc8b0e302443692037ed9f91b42000000000000000000000063".to_string(),
                address: "0x06Df3b2bbB68adc8B0e302443692037ED9f91b42".parse().unwrap(), // Stable Pool USD
                pool_type: "stable".to_string(),
                tokens: vec![
                    "0x6B175474E89094C44Da98b954EedeAC495271d0F".parse().unwrap(), // DAI
                    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(), // USDC
                    "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().unwrap(), // USDT
                ],
                token_symbols: vec!["DAI".to_string(), "USDC".to_string(), "USDT".to_string()],
                token_decimals: vec![18, 6, 6],
                weights: vec![0.33, 0.33, 0.34], // Equal weights for stable
                swap_fee: 0.0001,                 // 0.01%
            },
        ];

        Self {
            config,
            pools,
            client: None,
        }
    }

    /// Set the RPC client
    pub fn with_client(mut self, client: EthereumRpcClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Extract pool state for a specific pool
    async fn extract_pool_state(&self, pool: &BalancerPool, block_number: u64) -> Result<AmmFeature> {
        info!(pool = %pool.address, "Extracting Balancer pool state");

        let client = self.client.as_ref()
            .ok_or_else(|| BetaDataplaneError::internal("RPC client not set"))?;

        // Get real pool data from Balancer Vault
        let vault_address: H160 = "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap();
        
        // Convert pool_id string to bytes32
        let pool_id_bytes = hex::decode(pool.pool_id.trim_start_matches("0x"))
            .map_err(|e| BetaDataplaneError::internal(format!("Invalid pool ID: {}", e)))?;
        let mut pool_id: [u8; 32] = [0; 32];
        pool_id.copy_from_slice(&pool_id_bytes[..32]);

        // Make real contract call to get pool tokens and balances
        let pool_tokens = client.get_balancer_pool_tokens(vault_address, pool_id).await?;

        // Calculate total liquidity from REAL balances using actual token decimals
        let mut total_liquidity = 0.0;
        for (idx, balance) in pool_tokens.balances.iter().enumerate() {
            if let Some(decimals) = pool.token_decimals.get(idx) {
                let balance_readable = balance.as_u128() as f64 / 10f64.powi(*decimals as i32);
                total_liquidity += balance_readable;
            }
        }
        
        // For weighted pools, calculate price from reserves and weights
        let mid_price = if pool.tokens.len() >= 2 && pool_tokens.balances.len() >= 2 {
            let balance0 = pool_tokens.balances[0].as_u128() as f64 / 10f64.powi(pool.token_decimals[0] as i32);
            let balance1 = pool_tokens.balances[1].as_u128() as f64 / 10f64.powi(pool.token_decimals[1] as i32);
            let weight0 = pool.weights[0];
            let weight1 = pool.weights[1];
            
            // Price = (balance1/weight1) / (balance0/weight0)
            (balance1 / weight1) / (balance0 / weight0)
        } else {
            1.0
        };

        let liquidity = total_liquidity.to_string();

        // Build token info
        let token0_info = TokenInfo {
            address: format!("{:?}", pool.tokens[0]),
            symbol: pool.token_symbols[0].clone(),
            decimals: pool.token_decimals[0],
        };

        let token1_info = if pool.tokens.len() > 1 {
            TokenInfo {
                address: format!("{:?}", pool.tokens[1]),
                symbol: pool.token_symbols[1].clone(),
                decimals: pool.token_decimals[1],
            }
        } else {
            token0_info.clone()
        };

        // Build reserves map from REAL balances
        let mut reserves = HashMap::new();
        for (idx, symbol) in pool.token_symbols.iter().enumerate() {
            if let Some(balance) = pool_tokens.balances.get(idx) {
                reserves.insert(symbol.clone(), balance.to_string());
            }
        }

        // Calculate depth curve
        let depth = self.calculate_depth_curve(mid_price, liquidity.parse::<f64>().unwrap_or(0.0), &pool.weights);

        Ok(AmmFeature {
            pool_address: format!("{:?}", pool.address),
            pool_type: format!("balancer_{}", pool.pool_type),
            token0: token0_info,
            token1: token1_info,
            fee_tier: Some((pool.swap_fee * 10000.0) as u32), // Convert to bps
            reserves,
            mid_price,
            liquidity: liquidity.to_string(),
            depth,
            volume_24h: None,
            fees_24h: None,
        })
    }

    /// Calculate depth curve considering pool weights
    fn calculate_depth_curve(&self, mid_price: f64, liquidity: f64, weights: &[f64]) -> DepthCurve {
        let mut sizes = HashMap::new();

        let trade_sizes = vec![
            ("10k", 10_000.0),
            ("100k", 100_000.0),
            ("1m", 1_000_000.0),
            ("10m", 10_000_000.0),
        ];

        for (size_label, trade_size_usd) in trade_sizes {
            // Balancer uses weighted constant product formula
            // Slippage depends on pool weights and liquidity
            let slippage_bps = if liquidity > 0.0 {
                let weight_factor = weights.get(0).unwrap_or(&0.5);
                let base_slippage = (trade_size_usd / liquidity) * 8000.0 / weight_factor;
                base_slippage.min(1000.0) // Cap at 10%
            } else {
                1000.0
            };

            sizes.insert(
                size_label.to_string(),
                SlippageInfo {
                    slippage_bps,
                    price_impact: slippage_bps / 10000.0,
                },
            );
        }

        DepthCurve { sizes }
    }
}

#[async_trait]
impl BetaFeatureExtractor for BalancerExtractor {
    fn name(&self) -> &'static str {
        "balancer"
    }

    fn feature_type(&self) -> FeatureType {
        FeatureType::Amm
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
            pool_count = self.pools.len(),
            "Extracting Balancer pool features"
        );

        let mut features = Vec::new();

        for pool in &self.pools {
            match self.extract_pool_state(pool, block_number).await {
                Ok(amm_feature) => {
                    let feature = Feature {
                        id: Uuid::new_v4(),
                        block_number,
                        chain,
                        timestamp: Utc::now(),
                        feature_type: FeatureType::Amm,
                        data: FeatureData::Amm(amm_feature),
                        source: "balancer_extractor".to_string(),
                        version: "1.0.0".to_string(),
                    };
                    features.push(feature);
                }
                Err(e) => {
                    warn!(pool = %pool.address, error = %e, "Failed to extract Balancer pool");
                }
            }
        }

        let elapsed = start_time.elapsed();
        info!(
            features_extracted = features.len(),
            duration_ms = elapsed.as_millis(),
            "Balancer extraction completed"
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
    fn test_balancer_extractor_creation() {
        let extractor = BalancerExtractor::new(ExtractorConfig::default());
        assert_eq!(extractor.name(), "balancer");
        assert_eq!(extractor.pools.len(), 2);
    }

    #[test]
    fn test_supported_chains() {
        let extractor = BalancerExtractor::new(ExtractorConfig::default());
        let chains = extractor.supported_chains();
        assert!(chains.contains(&Chain::Ethereum));
        assert!(chains.contains(&Chain::Arbitrum));
    }
}
//! Uniswap V3 pool state extractor
//!
//! Extracts real-time pool state including:
//! - Current price (from slot0)
//! - Liquidity and reserves
//! - Tick data and price ranges
//! - Slippage depth curves

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, warn};
use ethers::types::{H160, U256};

use qenus_dataplane::{
    Feature, FeatureData, FeatureType,
    AmmFeature, TokenInfo, DepthCurve, SlippageInfo,
};

use crate::{
    extractors::traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    providers::EthereumRpcClient,
    Chain, Result,
};

/// Uniswap V3 feature extractor
pub struct UniswapV3Extractor {
    /// Extractor configuration
    config: ExtractorConfig,
    
    /// Known Uniswap V3 pools to monitor
    pools: Vec<UniswapV3Pool>,
    
    /// RPC client for contract calls
    client: Option<EthereumRpcClient>,
}

/// Uniswap V3 pool information
#[derive(Debug, Clone)]
pub struct UniswapV3Pool {
    /// Pool address
    pub address: H160,
    
    /// Token0 address
    pub token0: H160,
    
    /// Token1 address
    pub token1: H160,
    
    /// Fee tier (in hundredths of a bip, e.g., 3000 = 0.3%)
    pub fee: u32,
    
    /// Token0 symbol
    pub token0_symbol: String,
    
    /// Token1 symbol
    pub token1_symbol: String,
    
    /// Token0 decimals
    pub token0_decimals: u8,
    
    /// Token1 decimals
    pub token1_decimals: u8,
}

impl UniswapV3Extractor {
    /// Create a new Uniswap V3 extractor
    pub fn new(config: ExtractorConfig) -> Self {
        // Initialize with major Uniswap V3 pools on Ethereum
        let pools = vec![
            UniswapV3Pool {
                address: "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".parse().unwrap(), // USDC/WETH 0.3%
                token0: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(), // USDC
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(), // WETH
                fee: 3000,
                token0_symbol: "USDC".to_string(),
                token1_symbol: "WETH".to_string(),
                token0_decimals: 6,
                token1_decimals: 18,
            },
            UniswapV3Pool {
                address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".parse().unwrap(), // USDC/WETH 0.05%
                token0: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(), // USDC
                token1: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(), // WETH
                fee: 500,
                token0_symbol: "USDC".to_string(),
                token1_symbol: "WETH".to_string(),
                token0_decimals: 6,
                token1_decimals: 18,
            },
        ];

        Self { 
            config,
            pools,
            client: None,
        }
    }

    /// Set the RPC client for this extractor
    pub fn set_client(&mut self, client: EthereumRpcClient) {
        self.client = Some(client);
    }

    /// Extract pool state from RPC
    async fn extract_pool_state(
        &self,
        pool: &UniswapV3Pool,
        block_number: u64,
    ) -> Result<AmmFeature> {
        debug!(
            pool_address = %pool.address,
            block = block_number,
            "Extracting Uniswap V3 pool state"
        );

        let client = self.client.as_ref()
            .ok_or_else(|| crate::BetaDataplaneError::internal("RPC client not set"))?;

        // Get slot0 data (actual contract call)
        let slot0 = client.get_uniswap_v3_slot0(pool.address).await?;
        
        info!(
            pool = %pool.address,
            sqrt_price_x96 = %slot0.sqrt_price_x96,
            tick = slot0.tick,
            "Retrieved slot0 from Uniswap V3 pool"
        );
        
        // Calculate mid-price from sqrtPriceX96
        let mid_price = self.calculate_price_from_sqrt_price_x96(
            slot0.sqrt_price_x96,
            pool.token0_decimals,
            pool.token1_decimals,
        );

        // Get pool liquidity (actual contract call)
        let liquidity = client.get_uniswap_v3_liquidity(pool.address).await?;
        
        info!(
            pool = %pool.address,
            liquidity = %liquidity,
            mid_price = mid_price,
            "Calculated pool metrics"
        );

        // Calculate reserves from sqrtPrice and liquidity
        let (reserve0, reserve1) = self.calculate_reserves(
            slot0.sqrt_price_x96,
            liquidity,
            pool.token0_decimals,
            pool.token1_decimals,
        );

        // Calculate depth curve
        let depth = self.calculate_depth_curve(mid_price, liquidity);

        // Create token info
        let token0_info = TokenInfo {
            address: format!("{:?}", pool.token0),
            symbol: pool.token0_symbol.clone(),
            decimals: pool.token0_decimals,
        };

        let token1_info = TokenInfo {
            address: format!("{:?}", pool.token1),
            symbol: pool.token1_symbol.clone(),
            decimals: pool.token1_decimals,
        };

        // Build reserves map
        let mut reserves = HashMap::new();
        reserves.insert(pool.token0_symbol.clone(), reserve0.to_string());
        reserves.insert(pool.token1_symbol.clone(), reserve1.to_string());

        Ok(AmmFeature {
            pool_address: format!("{:?}", pool.address),
            pool_type: format!("uniswap_v3_{}_bps", pool.fee),
            token0: token0_info,
            token1: token1_info,
            fee_tier: Some(pool.fee),
            reserves,
            mid_price,
            liquidity: liquidity.to_string(),
            depth,
            volume_24h: None, // TODO: Implement volume tracking
            fees_24h: None,   // TODO: Implement fee tracking
        })
    }

    /// Calculate price from sqrtPriceX96
    fn calculate_price_from_sqrt_price_x96(
        &self,
        sqrt_price_x96: U256,
        token0_decimals: u8,
        token1_decimals: u8,
    ) -> f64 {
        // sqrtPrice = sqrt(price) * 2^96
        // price = (sqrtPrice / 2^96)^2
        
        let q96 = U256::from(2).pow(U256::from(96));
        let sqrt_price = sqrt_price_x96.as_u128() as f64;
        let q96_f64 = q96.as_u128() as f64;
        
        let price_raw = (sqrt_price / q96_f64).powi(2);
        
        // Adjust for token decimals
        let decimal_adjustment = 10f64.powi(token0_decimals as i32 - token1_decimals as i32);
        
        price_raw * decimal_adjustment
    }

    /// Calculate reserves from sqrtPrice and liquidity
    fn calculate_reserves(
        &self,
        sqrt_price_x96: U256,
        liquidity: U256,
        _token0_decimals: u8,
        _token1_decimals: u8,
    ) -> (U256, U256) {
        // For Uniswap V3, reserves are calculated from liquidity and price
        // reserve0 = liquidity / sqrt(price)
        // reserve1 = liquidity * sqrt(price)
        
        let q96 = U256::from(2).pow(U256::from(96));
        
        // Simplified calculation - in production, use exact Uniswap V3 math
        let reserve0 = if sqrt_price_x96 > U256::zero() {
            liquidity.saturating_mul(q96) / sqrt_price_x96
        } else {
            U256::zero()
        };
        
        let reserve1 = if !q96.is_zero() {
            liquidity.saturating_mul(sqrt_price_x96) / q96
        } else {
            U256::zero()
        };
        
        (reserve0, reserve1)
    }

    /// Calculate slippage depth curve for different trade sizes
    fn calculate_depth_curve(&self, mid_price: f64, liquidity: U256) -> DepthCurve {
        let mut sizes = HashMap::new();
        
        // Calculate slippage for standard trade sizes
        let trade_sizes = vec![
            ("100k", 100_000.0),
            ("1m", 1_000_000.0),
            ("10m", 10_000_000.0),
        ];

        for (size_label, trade_size_usd) in trade_sizes {
            // Simplified slippage calculation
            // In production, would use actual Uniswap V3 tick math
            let liquidity_f64 = liquidity.as_u128() as f64;
            
            // Estimate slippage based on trade size vs liquidity
            let liquidity_usd = liquidity_f64 * mid_price / 1e18; // Rough estimate
            let slippage_bps = if liquidity_usd > 0.0 {
                (trade_size_usd / liquidity_usd) * 10000.0
            } else {
                10000.0 // 100% if no liquidity
            };
            
            let price_impact = slippage_bps / 10000.0;
            
            sizes.insert(
                size_label.to_string(),
                SlippageInfo {
                    slippage_bps,
                    price_impact,
                },
            );
        }

        DepthCurve { sizes }
    }
}

#[async_trait]
impl BetaFeatureExtractor for UniswapV3Extractor {
    fn name(&self) -> &'static str {
        "uniswap_v3"
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
            "Extracting Uniswap V3 features"
        );

        let mut features = Vec::new();

        for pool in &self.pools {
            match self.extract_pool_state(pool, block_number).await {
                Ok(amm_feature) => {
                    let feature = Feature::new(
                        block_number,
                        chain,
                        FeatureType::Amm,
                        FeatureData::Amm(amm_feature),
                        format!("beta-{}", self.name()),
                    );
                    features.push(feature);
                }
                Err(e) => {
                    warn!(
                        pool_address = %pool.address,
                        error = %e,
                        "Failed to extract pool state"
                    );
                }
            }
        }

        let processing_time = start_time.elapsed().as_millis() as f64;
        
        info!(
            chain = %chain,
            block = block_number,
            features_extracted = features.len(),
            processing_time_ms = processing_time,
            "Uniswap V3 extraction completed"
        );

        Ok(features)
    }

    async fn extract_latest(
        &self,
        chain: Chain,
        context: &ExtractionContext,
    ) -> Result<Vec<Feature>> {
        // Get latest block number from context
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

impl UniswapV3Extractor {
    /// Add a pool to monitor
    pub fn add_pool(&mut self, pool: UniswapV3Pool) {
        self.pools.push(pool);
    }

    /// Get monitored pools
    pub fn get_pools(&self) -> &[UniswapV3Pool] {
        &self.pools
    }

    /// Get pool count
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}
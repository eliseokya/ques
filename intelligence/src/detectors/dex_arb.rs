//! DEX arbitrage detector

use std::sync::Arc;
use chrono::Utc;
use tracing::info;
use qenus_dataplane::Chain;

use crate::error::Result;
use crate::state::MarketState;
use crate::types::{Candidate, StrategyConfig};

/// DEX arbitrage detector: Uniswap → Curve → Balancer (same chain)
pub struct DexArbDetector {
    config: StrategyConfig,
    market_state: Arc<MarketState>,
}

impl DexArbDetector {
    /// Create a new DEX arbitrage detector
    pub fn new(config: StrategyConfig, market_state: Arc<MarketState>) -> Self {
        Self {
            config,
            market_state,
        }
    }
    
    /// Detect DEX arbitrage opportunities
    pub async fn detect(&self) -> Result<Vec<Candidate>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }
        
        let mut candidates = Vec::new();
        
        // For each approved asset
        for asset in &self.config.approved_assets {
            // For each approved chain
            for chain in &self.config.approved_chains {
                // Skip if sequencer is not healthy
                if !self.market_state.is_sequencer_healthy(*chain).await {
                    continue;
                }
                
                // Get all AMM pools on this chain
                let pools = self.market_state.get_amm_pools(*chain).await;
                
                // Find pools with our asset
                let relevant_pools: Vec<_> = pools.iter()
                    .filter(|pool| {
                        pool.token0_symbol == *asset || pool.token1_symbol == *asset
                    })
                    .collect();
                
                // Compare prices across pools
                if relevant_pools.len() >= 2 {
                    for i in 0..relevant_pools.len() {
                        for j in (i + 1)..relevant_pools.len() {
                            let pool_a = relevant_pools[i];
                            let pool_b = relevant_pools[j];
                            
                            // Skip if same pool type
                            if pool_a.pool_type == pool_b.pool_type {
                                continue;
                            }
                            
                            let price_a = pool_a.mid_price;
                            let price_b = pool_b.mid_price;
                            
                            // Calculate spread
                            let spread_bps = ((price_b - price_a) / price_a * 10000.0).abs();
                            
                            // Check if spread exceeds minimum threshold
                            if spread_bps >= self.config.min_profit_bps {
                                // Get swap fees
                                let fee_a = pool_a.fee_tier.unwrap_or(30);
                                let fee_b = pool_b.fee_tier.unwrap_or(30);
                                let total_fees_bps = fee_a + fee_b;
                                
                                // Net spread after swap fees
                                let net_spread_bps = spread_bps - total_fees_bps as f64;
                                
                                if net_spread_bps >= self.config.min_profit_bps {
                                    info!(
                                        "DEX arb: {} {:?} {}@{} -> {}@{} spread={:.2}bps net={:.2}bps",
                                        asset, chain, pool_a.pool_type, price_a,
                                        pool_b.pool_type, price_b, spread_bps, net_spread_bps
                                    );
                                    
                                    candidates.push(Candidate {
                                        strategy: "dex_arb".to_string(),
                                        asset: asset.clone(),
                                        spread_bps: net_spread_bps,
                                        legs: vec![
                                            (format!("{} on {:?}", pool_a.pool_type, chain), "buy".to_string()),
                                            (format!("{} on {:?}", pool_b.pool_type, chain), "sell".to_string()),
                                        ],
                                        detected_at: Utc::now(),
                                        confidence: 0.9,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(candidates)
    }
}


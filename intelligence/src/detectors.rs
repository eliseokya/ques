//! Candidate detection logic
//!
//! Detectors scan MarketState and identify arbitrage opportunities
//! based on configured strategies from the business module.

use std::sync::Arc;
use chrono::Utc;
use tracing::{debug, info, warn};
use qenus_dataplane::Chain;

use crate::error::{IntelligenceError, Result};
use crate::state::MarketState;
use crate::types::{Candidate, StrategyConfig};

/// Triangle arbitrage detector: L2 → Bridge → L1 → Bridge → L2
pub struct TriangleArbDetector {
    config: StrategyConfig,
    market_state: Arc<MarketState>,
}

impl TriangleArbDetector {
    /// Create a new triangle arbitrage detector
    pub fn new(config: StrategyConfig, market_state: Arc<MarketState>) -> Self {
        Self {
            config,
            market_state,
        }
    }
    
    /// Detect triangle arbitrage opportunities
    pub async fn detect(&self) -> Result<Vec<Candidate>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }
        
        let mut candidates = Vec::new();
        
        // For each approved asset
        for asset in &self.config.approved_assets {
            // For each pair of approved chains
            let chains: Vec<Chain> = self.config.approved_chains.clone();
            
            for i in 0..chains.len() {
                for j in 0..chains.len() {
                    if i == j {
                        continue;
                    }
                    
                    let chain_a = chains[i];
                    let chain_b = chains[j];
                    
                    // Skip if sequencers are not healthy
                    if !self.market_state.is_sequencer_healthy(chain_a).await {
                        continue;
                    }
                    if !self.market_state.is_sequencer_healthy(chain_b).await {
                        continue;
                    }
                    
                    // Get prices on both chains
                    let price_a = self.market_state.get_price(chain_a, asset).await;
                    let price_b = self.market_state.get_price(chain_b, asset).await;
                    
                    if let (Some(p_a), Some(p_b)) = (price_a, price_b) {
                        // Calculate spread
                        let spread_bps = ((p_b - p_a) / p_a * 10000.0).abs();
                        
                        // Check if spread exceeds minimum threshold
                        if spread_bps >= self.config.min_profit_bps {
                            // Get bridge fees
                            let bridge_fee_ab = self.market_state
                                .get_bridge_fee(chain_a, chain_b, asset)
                                .await
                                .unwrap_or(100);
                            
                            let bridge_fee_ba = self.market_state
                                .get_bridge_fee(chain_b, chain_a, asset)
                                .await
                                .unwrap_or(100);
                            
                            let total_bridge_fees_bps = bridge_fee_ab + bridge_fee_ba;
                            let net_spread_bps = spread_bps - total_bridge_fees_bps as f64;
                            
                            if net_spread_bps >= self.config.min_profit_bps {
                                info!(
                                    "Triangle arb: {} {:?}@{} -> {:?}@{} spread={:.2}bps net={:.2}bps",
                                    asset, chain_a, p_a, chain_b, p_b, spread_bps, net_spread_bps
                                );
                                
                                candidates.push(Candidate {
                                    strategy: "triangle_arb".to_string(),
                                    asset: asset.clone(),
                                    spread_bps: net_spread_bps,
                                    legs: vec![
                                        (format!("{:?}", chain_a), "buy".to_string()),
                                        (format!("{:?}->{:?}", chain_a, chain_b), "bridge".to_string()),
                                        (format!("{:?}", chain_b), "sell".to_string()),
                                    ],
                                    detected_at: Utc::now(),
                                    confidence: 0.8,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(candidates)
    }
}

pub mod dex_arb;
pub mod manager;

pub use dex_arb::DexArbDetector;
pub use manager::DetectorManager;


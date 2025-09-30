//! Bridge cost and latency models

use std::sync::Arc;
use qenus_dataplane::Chain;
use crate::state::MarketState;
use crate::Result;

/// Bridge simulator
pub struct BridgeSimulator {
    market_state: Arc<MarketState>,
}

impl BridgeSimulator {
    pub fn new(market_state: Arc<MarketState>) -> Self {
        Self { market_state }
    }
    
    /// Estimate bridge fee in basis points
    pub async fn estimate_bridge_fee(&self, from_chain: Chain, to_chain: Chain, asset: &str) -> u32 {
        if let Some(fee_bps) = self.market_state.get_bridge_fee(from_chain, to_chain, asset).await {
            fee_bps
        } else {
            // Fallback: canonical bridges typically 0.1%
            10
        }
    }
    
    /// Estimate bridge settlement time in seconds
    pub fn estimate_settlement_time(&self, from_chain: Chain, to_chain: Chain) -> u64 {
        match (from_chain, to_chain) {
            (Chain::Ethereum, _) | (_, Chain::Ethereum) => {
                // L1 → L2 or L2 → L1
                if matches!(from_chain, Chain::Ethereum) {
                    600 // L1 → L2: ~10 minutes
                } else {
                    3600 // L2 → L1: ~1 hour (challenge period)
                }
            }
            _ => 300, // L2 → L2 via L1: ~5 minutes
        }
    }
    
    /// Calculate bridge cost including fees and gas
    pub async fn calculate_total_bridge_cost(
        &self,
        from_chain: Chain,
        to_chain: Chain,
        asset: &str,
        amount_usd: f64,
        eth_price: f64,
    ) -> Result<(f64, u32)> {
        let fee_bps = self.estimate_bridge_fee(from_chain, to_chain, asset).await;
        let fee_usd = amount_usd * (fee_bps as f64 / 10000.0);
        
        Ok((fee_usd, fee_bps))
    }
}


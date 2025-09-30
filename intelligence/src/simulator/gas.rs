//! Gas cost estimation using beta_dataplane gas features

use std::sync::Arc;
use qenus_dataplane::Chain;
use crate::state::MarketState;

/// Gas estimator
pub struct GasEstimator {
    market_state: Arc<MarketState>,
}

impl GasEstimator {
    pub fn new(market_state: Arc<MarketState>) -> Self {
        Self { market_state }
    }
    
    /// Estimate gas cost for a swap transaction
    pub async fn estimate_swap_gas(&self, chain: Chain, eth_price: f64) -> f64 {
        if let Some(gas_price_gwei) = self.market_state.get_gas_price(chain).await {
            let gas_units = match chain {
                Chain::Ethereum => 150_000.0, // L1 swap
                _ => 150_000.0, // L2 swap (cheaper per unit)
            };
            
            let gas_cost_eth = (gas_price_gwei * gas_units) / 1e9;
            gas_cost_eth * eth_price
        } else {
            // Fallback estimates
            self.fallback_swap_gas(chain)
        }
    }
    
    /// Estimate gas cost for a bridge transaction
    pub async fn estimate_bridge_gas(&self, eth_price: f64) -> f64 {
        if let Some(gas_price_gwei) = self.market_state.get_gas_price(Chain::Ethereum).await {
            let gas_units = 300_000.0; // Bridges cost more
            let gas_cost_eth = (gas_price_gwei * gas_units) / 1e9;
            gas_cost_eth * eth_price
        } else {
            100.0 // Fallback: $100 for bridge
        }
    }
    
    /// Estimate gas for flash loan
    pub async fn estimate_flashloan_gas(&self, chain: Chain, eth_price: f64) -> f64 {
        if let Some(gas_price_gwei) = self.market_state.get_gas_price(chain).await {
            let gas_units = 200_000.0; // Flash loan overhead
            let gas_cost_eth = (gas_price_gwei * gas_units) / 1e9;
            gas_cost_eth * eth_price
        } else {
            self.fallback_swap_gas(chain) * 1.5 // 50% more than swap
        }
    }
    
    fn fallback_swap_gas(&self, chain: Chain) -> f64 {
        match chain {
            Chain::Ethereum => 50.0,
            Chain::Arbitrum => 0.5,
            Chain::Optimism => 0.5,
            Chain::Base => 0.3,
        }
    }
}


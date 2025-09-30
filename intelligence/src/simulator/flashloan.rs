//! Flash loan cost models

use std::sync::Arc;
use qenus_dataplane::Chain;
use crate::state::MarketState;
use crate::Result;

/// Flash loan simulator
pub struct FlashLoanSimulator {
    market_state: Arc<MarketState>,
}

impl FlashLoanSimulator {
    pub fn new(market_state: Arc<MarketState>) -> Self {
        Self { market_state }
    }
    
    /// Check if flash loan is needed for trade size
    pub fn needs_flashloan(&self, trade_size_usd: f64, available_capital_usd: f64) -> bool {
        trade_size_usd > available_capital_usd
    }
    
    /// Get flash loan availability
    pub async fn get_flashloan_liquidity(&self, chain: Chain, asset: &str) -> Option<f64> {
        self.market_state.get_flashloan_liquidity(chain, asset)
            .await
            .and_then(|liq_str| liq_str.parse::<f64>().ok())
    }
    
    /// Estimate flash loan fee
    pub fn estimate_flashloan_fee(&self, provider: &str, amount_usd: f64) -> f64 {
        let fee_bps = match provider {
            "aave_v3" => 5.0,      // 0.05%
            "balancer" => 0.0,     // 0%
            "dydx" => 0.0,         // 0%
            _ => 5.0,
        };
        
        amount_usd * (fee_bps / 10000.0)
    }
    
    /// Find best flash loan provider
    pub async fn find_best_provider(&self, chain: Chain, asset: &str, amount_needed: f64) -> Option<(String, f64)> {
        // Check Aave V3
        if let Some(aave_liquidity) = self.get_flashloan_liquidity(chain, asset).await {
            if aave_liquidity >= amount_needed {
                let fee = self.estimate_flashloan_fee("aave_v3", amount_needed);
                return Some(("aave_v3".to_string(), fee));
            }
        }
        
        // Balancer has 0% fees but may have less liquidity
        // TODO: Check Balancer liquidity from market state
        
        None
    }
}


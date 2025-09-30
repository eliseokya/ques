//! Trade evaluator - ties all simulation components together

use std::sync::Arc;
use tracing::{debug, warn};
use qenus_dataplane::Chain;

use crate::{Candidate, EvaluationResult, CostBreakdown, SimulatedStep, Result, IntelligenceError};
use crate::state::MarketState;
use super::{gas::GasEstimator, bridge::BridgeSimulator, flashloan::FlashLoanSimulator};

/// Trade simulator - evaluates candidates using market state
pub struct TradeSimulator {
    market_state: Arc<MarketState>,
    gas_estimator: GasEstimator,
    bridge_simulator: BridgeSimulator,
    flashloan_simulator: FlashLoanSimulator,
}

impl TradeSimulator {
    /// Create a new trade simulator
    pub fn new(market_state: Arc<MarketState>) -> Self {
        Self {
            gas_estimator: GasEstimator::new(market_state.clone()),
            bridge_simulator: BridgeSimulator::new(market_state.clone()),
            flashloan_simulator: FlashLoanSimulator::new(market_state.clone()),
            market_state,
        }
    }

    /// Evaluate a candidate and produce detailed simulation results
    pub async fn evaluate(&self, candidate: &Candidate) -> Result<EvaluationResult> {
        debug!("Simulating candidate: {} on {}", candidate.strategy, candidate.asset);
        
        // Get ETH price for gas calculations
        let eth_price = self.get_eth_price().await.unwrap_or(3000.0);
        
        // Route to appropriate simulation strategy
        match candidate.strategy.as_str() {
            "triangle_arb" => self.simulate_triangle_arb(candidate, eth_price).await,
            "dex_arb" => self.simulate_dex_arb(candidate, eth_price).await,
            _ => Err(IntelligenceError::Simulation {
                message: format!("Unknown strategy: {}", candidate.strategy),
            }),
        }
    }
    
    /// Get current ETH price from market state
    async fn get_eth_price(&self) -> Option<f64> {
        for chain in &[Chain::Ethereum, Chain::Arbitrum, Chain::Optimism, Chain::Base] {
            if let Some(price) = self.market_state.get_price(*chain, "WETH").await {
                return Some(price);
            }
        }
        None
    }
    
    /// Simulate triangle arbitrage
    async fn simulate_triangle_arb(&self, candidate: &Candidate, eth_price: f64) -> Result<EvaluationResult> {
        let optimal_size_usd = self.estimate_optimal_size(candidate).await?;
        
        let mut execution_path = Vec::new();
        let mut costs = CostBreakdown {
            gas_usd: 0.0,
            protocol_fees_usd: 0.0,
            bridge_fees_usd: 0.0,
            flashloan_fees_usd: 0.0,
            slippage_usd: 0.0,
            total_usd: 0.0,
        };
        
        // Step 1: Swap on source chain
        let swap1_gas = self.gas_estimator.estimate_swap_gas(Chain::Arbitrum, eth_price).await;
        costs.gas_usd += swap1_gas;
        
        let swap1_slippage_bps = 5.0;
        let swap1_fee_bps = 5.0;
        
        costs.slippage_usd += optimal_size_usd * swap1_slippage_bps / 10000.0;
        costs.protocol_fees_usd += optimal_size_usd * swap1_fee_bps / 10000.0;
        
        execution_path.push(SimulatedStep {
            step: 1,
            action: "swap_buy".to_string(),
            domain: "Arbitrum".to_string(),
            protocol: "uniswap_v3".to_string(),
            amount_in: optimal_size_usd,
            amount_out: optimal_size_usd * (1.0 - (swap1_slippage_bps + swap1_fee_bps) / 10000.0),
            slippage_bps: swap1_slippage_bps,
            cost_usd: swap1_gas + costs.slippage_usd + costs.protocol_fees_usd,
        });
        
        // Step 2: Bridge
        let (bridge_fee_usd, _) = self.bridge_simulator.calculate_total_bridge_cost(
            Chain::Arbitrum, Chain::Ethereum, &candidate.asset, optimal_size_usd, eth_price
        ).await?;
        
        costs.bridge_fees_usd += bridge_fee_usd;
        costs.gas_usd += self.gas_estimator.estimate_bridge_gas(eth_price).await;
        
        execution_path.push(SimulatedStep {
            step: 2,
            action: "bridge".to_string(),
            domain: "Arbitrum -> Ethereum".to_string(),
            protocol: "canonical_bridge".to_string(),
            amount_in: execution_path[0].amount_out,
            amount_out: execution_path[0].amount_out - bridge_fee_usd,
            slippage_bps: 0.0,
            cost_usd: bridge_fee_usd,
        });
        
        // Step 3: Swap on destination chain
        let swap2_gas = self.gas_estimator.estimate_swap_gas(Chain::Ethereum, eth_price).await;
        costs.gas_usd += swap2_gas;
        
        let swap2_slippage_bps = 5.0;
        let swap2_fee_bps = 5.0;
        
        costs.slippage_usd += optimal_size_usd * swap2_slippage_bps / 10000.0;
        costs.protocol_fees_usd += optimal_size_usd * swap2_fee_bps / 10000.0;
        
        let amount_out = execution_path[1].amount_out * (1.0 + candidate.spread_bps / 10000.0);
        
        execution_path.push(SimulatedStep {
            step: 3,
            action: "swap_sell".to_string(),
            domain: "Ethereum".to_string(),
            protocol: "curve".to_string(),
            amount_in: execution_path[1].amount_out,
            amount_out,
            slippage_bps: swap2_slippage_bps,
            cost_usd: swap2_gas + costs.slippage_usd + costs.protocol_fees_usd,
        });
        
        // Calculate PnL
        costs.total_usd = costs.gas_usd + costs.protocol_fees_usd + 
                          costs.bridge_fees_usd + costs.flashloan_fees_usd + costs.slippage_usd;
        
        let net_pnl_usd = execution_path.last().unwrap().amount_out - optimal_size_usd - costs.total_usd;
        let net_bps = (net_pnl_usd / optimal_size_usd) * 10000.0;
        
        let success_prob = self.estimate_success_probability(candidate, &costs).await;
        
        Ok(EvaluationResult {
            net_pnl_usd,
            net_bps,
            optimal_size_usd,
            success_prob,
            costs,
            execution_path,
        })
    }
    
    /// Simulate DEX arbitrage
    async fn simulate_dex_arb(&self, candidate: &Candidate, eth_price: f64) -> Result<EvaluationResult> {
        let optimal_size_usd = self.estimate_optimal_size(candidate).await?;
        
        let mut execution_path = Vec::new();
        let mut costs = CostBreakdown {
            gas_usd: 0.0,
            protocol_fees_usd: 0.0,
            bridge_fees_usd: 0.0,
            flashloan_fees_usd: 0.0,
            slippage_usd: 0.0,
            total_usd: 0.0,
        };
        
        // Check if flash loan is needed
        let use_flashloan = optimal_size_usd > 50000.0;
        if use_flashloan {
            costs.flashloan_fees_usd = self.flashloan_simulator.estimate_flashloan_fee("aave_v3", optimal_size_usd);
            costs.gas_usd += self.gas_estimator.estimate_flashloan_gas(Chain::Ethereum, eth_price).await;
        }
        
        // Swap 1: Buy
        let swap1_gas = self.gas_estimator.estimate_swap_gas(Chain::Ethereum, eth_price).await;
        costs.gas_usd += swap1_gas;
        
        let swap1_slippage_bps = 3.0;
        let swap1_fee_bps = 5.0;
        
        costs.slippage_usd += optimal_size_usd * swap1_slippage_bps / 10000.0;
        costs.protocol_fees_usd += optimal_size_usd * swap1_fee_bps / 10000.0;
        
        execution_path.push(SimulatedStep {
            step: 1,
            action: "swap_buy".to_string(),
            domain: "Ethereum".to_string(),
            protocol: "uniswap_v3".to_string(),
            amount_in: optimal_size_usd,
            amount_out: optimal_size_usd * (1.0 - (swap1_slippage_bps + swap1_fee_bps) / 10000.0),
            slippage_bps: swap1_slippage_bps,
            cost_usd: swap1_gas + costs.slippage_usd + costs.protocol_fees_usd,
        });
        
        // Swap 2: Sell
        let swap2_gas = self.gas_estimator.estimate_swap_gas(Chain::Ethereum, eth_price).await;
        costs.gas_usd += swap2_gas;
        
        let swap2_slippage_bps = 3.0;
        let swap2_fee_bps = 4.0;
        
        costs.slippage_usd += optimal_size_usd * swap2_slippage_bps / 10000.0;
        costs.protocol_fees_usd += optimal_size_usd * swap2_fee_bps / 10000.0;
        
        let amount_out = execution_path[0].amount_out * (1.0 + candidate.spread_bps / 10000.0);
        
        execution_path.push(SimulatedStep {
            step: 2,
            action: "swap_sell".to_string(),
            domain: "Ethereum".to_string(),
            protocol: "curve".to_string(),
            amount_in: execution_path[0].amount_out,
            amount_out,
            slippage_bps: swap2_slippage_bps,
            cost_usd: swap2_gas + costs.slippage_usd + costs.protocol_fees_usd,
        });
        
        // Calculate PnL
        costs.total_usd = costs.gas_usd + costs.protocol_fees_usd + 
                          costs.bridge_fees_usd + costs.flashloan_fees_usd + costs.slippage_usd;
        
        let net_pnl_usd = execution_path.last().unwrap().amount_out - optimal_size_usd - costs.total_usd;
        let net_bps = (net_pnl_usd / optimal_size_usd) * 10000.0;
        
        let success_prob = self.estimate_success_probability(candidate, &costs).await;
        
        Ok(EvaluationResult {
            net_pnl_usd,
            net_bps,
            optimal_size_usd,
            success_prob,
            costs,
            execution_path,
        })
    }
    
    /// Estimate optimal trade size
    async fn estimate_optimal_size(&self, candidate: &Candidate) -> Result<f64> {
        // TODO: Use beta_dataplane depth curves
        if candidate.spread_bps > 50.0 {
            Ok(500_000.0)
        } else if candidate.spread_bps > 20.0 {
            Ok(250_000.0)
        } else {
            Ok(100_000.0)
        }
    }
    
    /// Estimate success probability
    async fn estimate_success_probability(&self, candidate: &Candidate, costs: &CostBreakdown) -> f64 {
        let mut prob = candidate.confidence;
        
        let cost_ratio = costs.total_usd / (costs.total_usd + 100.0);
        prob *= 1.0 - (cost_ratio * 0.2);
        
        if candidate.strategy == "triangle_arb" {
            prob *= 0.9;
        }
        
        prob.max(0.5).min(0.99)
    }
}

impl Default for TradeSimulator {
    fn default() -> Self {
        Self::new(Arc::new(MarketState::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    #[tokio::test]
    async fn test_dex_arb_simulation() {
        let market_state = Arc::new(MarketState::new(30));
        let simulator = TradeSimulator::new(market_state);
        
        let candidate = Candidate {
            strategy: "dex_arb".to_string(),
            asset: "USDC".to_string(),
            spread_bps: 15.0,
            legs: vec![
                ("Ethereum".to_string(), "buy".to_string()),
                ("Ethereum".to_string(), "sell".to_string()),
            ],
            detected_at: Utc::now(),
            confidence: 0.9,
        };
        
        let result = simulator.evaluate(&candidate).await.unwrap();
        
        assert_eq!(result.execution_path.len(), 2);
        assert!(result.success_prob > 0.0 && result.success_prob < 1.0);
        assert!(result.optimal_size_usd > 0.0);
    }
}


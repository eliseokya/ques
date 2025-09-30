//! Trade decision engine
//!
//! Applies risk policies and selects best opportunities to execute.
//! This is the "risk management brain" that filters simulation results.

use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::{
    EvaluationResult, Candidate, StrategyConfig, RiskLimits, Result, IntelligenceError,
};
use crate::state::MarketState;

/// Decision made by the engine
#[derive(Debug, Clone)]
pub struct TradeDecision {
    /// Whether to execute this trade
    pub should_execute: bool,
    
    /// The evaluation result that passed all checks
    pub evaluation: EvaluationResult,
    
    /// The original candidate
    pub candidate: Candidate,
    
    /// Decision score (higher = better)
    pub score: f64,
    
    /// Why this decision was made
    pub reasoning: Vec<String>,
    
    /// Any warnings
    pub warnings: Vec<String>,
}

/// Position tracker for exposure management
#[derive(Debug, Clone)]
pub struct PositionTracker {
    /// Current positions by asset (USD value)
    positions: HashMap<String, f64>,
    
    /// Max position per asset
    max_position_per_asset: f64,
}

impl PositionTracker {
    pub fn new(max_position_per_asset: f64) -> Self {
        Self {
            positions: HashMap::new(),
            max_position_per_asset,
        }
    }
    
    /// Check if we can take this position
    pub fn can_take_position(&self, asset: &str, size_usd: f64) -> bool {
        let current = self.positions.get(asset).copied().unwrap_or(0.0);
        current + size_usd <= self.max_position_per_asset
    }
    
    /// Record a new position
    pub fn add_position(&mut self, asset: &str, size_usd: f64) {
        *self.positions.entry(asset.to_string()).or_insert(0.0) += size_usd;
    }
    
    /// Get current position
    pub fn get_position(&self, asset: &str) -> f64 {
        self.positions.get(asset).copied().unwrap_or(0.0)
    }
}

/// Decision engine - applies risk policies and selects best trades
pub struct DecisionEngine {
    market_state: Arc<MarketState>,
    position_tracker: Arc<tokio::sync::RwLock<PositionTracker>>,
}

impl DecisionEngine {
    /// Create a new decision engine
    pub fn new(market_state: Arc<MarketState>, max_position_per_asset: f64) -> Self {
        Self {
            market_state,
            position_tracker: Arc::new(tokio::sync::RwLock::new(
                PositionTracker::new(max_position_per_asset)
            )),
        }
    }

    /// Evaluate whether to execute a trade
    pub async fn decide(
        &self,
        candidate: Candidate,
        evaluation: EvaluationResult,
        strategy_config: &StrategyConfig,
    ) -> Result<TradeDecision> {
        let mut reasoning = Vec::new();
        let mut warnings = Vec::new();
        let mut should_execute = true;
        
        debug!("Evaluating decision for {} on {}", candidate.strategy, candidate.asset);
        
        // 1. Check minimum profit threshold
        if evaluation.net_pnl_usd < strategy_config.min_profit_usd {
            reasoning.push(format!(
                "❌ PnL ${:.2} < min ${:.2}",
                evaluation.net_pnl_usd,
                strategy_config.min_profit_usd
            ));
            should_execute = false;
        } else {
            reasoning.push(format!(
                "✅ PnL ${:.2} >= min ${:.2}",
                evaluation.net_pnl_usd,
                strategy_config.min_profit_usd
            ));
        }
        
        // 2. Check minimum profit in basis points
        if evaluation.net_bps < strategy_config.min_profit_bps {
            reasoning.push(format!(
                "❌ Net spread {:.2}bps < min {:.2}bps",
                evaluation.net_bps,
                strategy_config.min_profit_bps
            ));
            should_execute = false;
        } else {
            reasoning.push(format!(
                "✅ Net spread {:.2}bps >= min {:.2}bps",
                evaluation.net_bps,
                strategy_config.min_profit_bps
            ));
        }
        
        // 3. Check slippage limit
        let total_slippage_bps: f64 = evaluation.execution_path
            .iter()
            .map(|step| step.slippage_bps)
            .sum();
        
        if total_slippage_bps > strategy_config.risk_limits.max_slippage_bps {
            reasoning.push(format!(
                "❌ Slippage {:.2}bps > max {:.2}bps",
                total_slippage_bps,
                strategy_config.risk_limits.max_slippage_bps
            ));
            should_execute = false;
        } else {
            reasoning.push(format!(
                "✅ Slippage {:.2}bps <= max {:.2}bps",
                total_slippage_bps,
                strategy_config.risk_limits.max_slippage_bps
            ));
        }
        
        // 4. Check gas cost as percentage of profit
        let gas_pct = if evaluation.net_pnl_usd > 0.0 {
            (evaluation.costs.gas_usd / evaluation.net_pnl_usd) * 100.0
        } else {
            100.0 // If no profit, gas is 100% of "profit"
        };
        
        if gas_pct > strategy_config.risk_limits.max_gas_pct {
            reasoning.push(format!(
                "❌ Gas {:.1}% of profit > max {:.1}%",
                gas_pct,
                strategy_config.risk_limits.max_gas_pct
            ));
            should_execute = false;
        } else {
            reasoning.push(format!(
                "✅ Gas {:.1}% of profit <= max {:.1}%",
                gas_pct,
                strategy_config.risk_limits.max_gas_pct
            ));
        }
        
        // 5. Check success probability
        if evaluation.success_prob < strategy_config.risk_limits.min_success_prob {
            reasoning.push(format!(
                "❌ Success prob {:.2} < min {:.2}",
                evaluation.success_prob,
                strategy_config.risk_limits.min_success_prob
            ));
            should_execute = false;
        } else {
            reasoning.push(format!(
                "✅ Success prob {:.2} >= min {:.2}",
                evaluation.success_prob,
                strategy_config.risk_limits.min_success_prob
            ));
        }
        
        // 6. Check position limits
        let position_tracker = self.position_tracker.read().await;
        let current_position = position_tracker.get_position(&candidate.asset);
        let can_take = position_tracker.can_take_position(&candidate.asset, evaluation.optimal_size_usd);
        drop(position_tracker);
        
        if !can_take {
            reasoning.push(format!(
                "❌ Position limit: current ${:.0} + ${:.0} > max ${:.0}",
                current_position,
                evaluation.optimal_size_usd,
                strategy_config.max_position_usd
            ));
            should_execute = false;
        } else {
            reasoning.push(format!(
                "✅ Position ok: ${:.0} + ${:.0} <= ${:.0}",
                current_position,
                evaluation.optimal_size_usd,
                strategy_config.max_position_usd
            ));
        }
        
        // 7. Check sequencer health for involved chains
        let chains_involved = self.extract_chains(&candidate);
        for chain in chains_involved {
            if !self.market_state.is_sequencer_healthy(chain).await {
                reasoning.push(format!(
                    "❌ Sequencer unhealthy on {:?}",
                    chain
                ));
                should_execute = false;
            }
        }
        
        // 8. Check if asset is approved
        if !strategy_config.approved_assets.contains(&candidate.asset) {
            reasoning.push(format!(
                "❌ Asset {} not in approved list",
                candidate.asset
            ));
            should_execute = false;
        }
        
        // Calculate decision score (for ranking multiple opportunities)
        let score = self.calculate_score(&evaluation, strategy_config);
        
        // Add warnings for marginal cases
        if evaluation.net_pnl_usd < strategy_config.min_profit_usd * 1.5 {
            warnings.push("Low profit margin".to_string());
        }
        
        if gas_pct > strategy_config.risk_limits.max_gas_pct * 0.7 {
            warnings.push("High gas cost relative to profit".to_string());
        }
        
        if should_execute {
            info!(
                "✅ APPROVED: {} on {} - PnL: ${:.2} ({:.2}bps), Score: {:.2}",
                candidate.strategy,
                candidate.asset,
                evaluation.net_pnl_usd,
                evaluation.net_bps,
                score
            );
        } else {
            let rejections: Vec<String> = reasoning.iter()
                .filter(|r| r.starts_with("❌"))
                .map(|s| s.to_string())
                .collect();
            info!(
                "❌ REJECTED: {} on {} - {}",
                candidate.strategy,
                candidate.asset,
                rejections.join(", ")
            );
        }
        
        Ok(TradeDecision {
            should_execute,
            evaluation,
            candidate,
            score,
            reasoning,
            warnings,
        })
    }
    
    /// Select best trades from a list of decisions
    pub async fn select_best(
        &self,
        mut decisions: Vec<TradeDecision>,
        max_concurrent: usize,
    ) -> Result<Vec<TradeDecision>> {
        // Filter to only executable decisions
        decisions.retain(|d| d.should_execute);
        
        if decisions.is_empty() {
            return Ok(Vec::new());
        }
        
        // Sort by score (highest first)
        decisions.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Take top N, respecting position limits
        let mut selected = Vec::new();
        let mut position_tracker = self.position_tracker.write().await;
        
        for decision in decisions {
            if selected.len() >= max_concurrent {
                break;
            }
            
            // Double-check position limits
            if position_tracker.can_take_position(
                &decision.candidate.asset,
                decision.evaluation.optimal_size_usd
            ) {
                // Reserve the position
                position_tracker.add_position(
                    &decision.candidate.asset,
                    decision.evaluation.optimal_size_usd
                );
                
                selected.push(decision);
            } else {
                warn!(
                    "Skipping {} - position limit reached for {}",
                    decision.candidate.strategy,
                    decision.candidate.asset
                );
            }
        }
        
        let total_evaluated = selected.len();
        
        info!(
            "Selected {} trades from candidates (max_concurrent: {})",
            selected.len(),
            max_concurrent
        );
        
        Ok(selected)
    }
    
    /// Release a position after trade completion
    pub async fn release_position(&self, asset: &str, size_usd: f64) {
        let mut tracker = self.position_tracker.write().await;
        if let Some(current) = tracker.positions.get_mut(asset) {
            *current = (*current - size_usd).max(0.0);
        }
    }
    
    /// Extract chains involved in a candidate
    fn extract_chains(&self, candidate: &Candidate) -> Vec<qenus_dataplane::Chain> {
        // Parse chain names from legs
        let mut chains = Vec::new();
        
        for (domain, _) in &candidate.legs {
            if domain.contains("Ethereum") {
                chains.push(qenus_dataplane::Chain::Ethereum);
            }
            if domain.contains("Arbitrum") {
                chains.push(qenus_dataplane::Chain::Arbitrum);
            }
            if domain.contains("Optimism") {
                chains.push(qenus_dataplane::Chain::Optimism);
            }
            if domain.contains("Base") {
                chains.push(qenus_dataplane::Chain::Base);
            }
        }
        
        // Remove duplicates manually since Chain doesn't implement Ord
        let mut unique_chains = Vec::new();
        for chain in chains {
            if !unique_chains.contains(&chain) {
                unique_chains.push(chain);
            }
        }
        chains = unique_chains;
        chains
    }
    
    /// Calculate decision score for ranking
    fn calculate_score(&self, evaluation: &EvaluationResult, _config: &StrategyConfig) -> f64 {
        // Score = PnL * Success Probability * Risk-Adjusted Return
        let base_score = evaluation.net_pnl_usd * evaluation.success_prob;
        
        // Adjust for risk (higher bps = better)
        let risk_adjustment = (evaluation.net_bps / 10.0).min(10.0); // Cap at 10x
        
        // Penalize high costs
        let cost_ratio = evaluation.costs.total_usd / (evaluation.optimal_size_usd + 1.0);
        let cost_penalty = 1.0 - cost_ratio.min(0.5); // Max 50% penalty
        
        base_score * risk_adjustment * cost_penalty
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new(Arc::new(MarketState::default()), 5_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::{CostBreakdown, SimulatedStep};
    
    fn create_test_evaluation(net_pnl_usd: f64, net_bps: f64) -> EvaluationResult {
        EvaluationResult {
            net_pnl_usd,
            net_bps,
            optimal_size_usd: 100_000.0,
            success_prob: 0.85,
            costs: CostBreakdown {
                gas_usd: 50.0,
                protocol_fees_usd: 100.0,
                bridge_fees_usd: 0.0,
                flashloan_fees_usd: 0.0,
                slippage_usd: 50.0,
                total_usd: 200.0,
            },
            execution_path: vec![
                SimulatedStep {
                    step: 1,
                    action: "swap".to_string(),
                    domain: "Ethereum".to_string(),
                    protocol: "uniswap_v3".to_string(),
                    amount_in: 100_000.0,
                    amount_out: 100_500.0,
                    slippage_bps: 5.0,
                    cost_usd: 100.0,
                },
            ],
        }
    }
    
    #[tokio::test]
    async fn test_decision_approval() {
        let market_state = Arc::new(MarketState::new(30));
        let engine = DecisionEngine::new(market_state, 5_000_000.0);
        
        let candidate = Candidate {
            strategy: "dex_arb".to_string(),
            asset: "USDC".to_string(),
            spread_bps: 15.0,
            legs: vec![],
            detected_at: Utc::now(),
            confidence: 0.9,
        };
        
        let evaluation = create_test_evaluation(600.0, 12.0);
        
        let config = StrategyConfig {
            name: "test".to_string(),
            enabled: true,
            min_profit_usd: 500.0,
            min_profit_bps: 10.0,
            max_position_usd: 1_000_000.0,
            approved_assets: vec!["USDC".to_string()],
            approved_chains: vec![qenus_dataplane::Chain::Ethereum],
            risk_limits: RiskLimits::default(),
        };
        
        let decision = engine.decide(candidate, evaluation, &config).await.unwrap();
        
        assert!(decision.should_execute);
        assert!(decision.score > 0.0);
    }
    
    #[tokio::test]
    async fn test_decision_rejection_low_profit() {
        let market_state = Arc::new(MarketState::new(30));
        let engine = DecisionEngine::new(market_state, 5_000_000.0);
        
        let candidate = Candidate {
            strategy: "dex_arb".to_string(),
            asset: "USDC".to_string(),
            spread_bps: 3.0,
            legs: vec![],
            detected_at: Utc::now(),
            confidence: 0.9,
        };
        
        let evaluation = create_test_evaluation(100.0, 2.0); // Too low
        
        let config = StrategyConfig {
            name: "test".to_string(),
            enabled: true,
            min_profit_usd: 500.0,
            min_profit_bps: 10.0,
            max_position_usd: 1_000_000.0,
            approved_assets: vec!["USDC".to_string()],
            approved_chains: vec![qenus_dataplane::Chain::Ethereum],
            risk_limits: RiskLimits::default(),
        };
        
        let decision = engine.decide(candidate, evaluation, &config).await.unwrap();
        
        assert!(!decision.should_execute);
        assert!(decision.reasoning.iter().any(|r| r.contains("PnL")));
    }
}

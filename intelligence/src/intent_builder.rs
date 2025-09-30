//! Trade intent builder - converts decisions into executable intents

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{Utc, Duration};
use tracing::debug;

use crate::{
    TradeIntent, TradeLeg, TradeAction, TradeMetadata, MarketSnapshot, RiskFactor, RiskSeverity,
    TradeDecision, Result, IntelligenceError,
};
use crate::state::MarketState;

/// Intent builder
pub struct IntentBuilder {
    market_state: Arc<MarketState>,
}

impl IntentBuilder {
    pub fn new(market_state: Arc<MarketState>) -> Self {
        Self { market_state }
    }

    /// Build trade intent from approved decision
    pub async fn build(&self, decision: &TradeDecision) -> Result<TradeIntent> {
        if !decision.should_execute {
            return Err(IntelligenceError::Internal(
                "Cannot build intent for rejected decision".to_string()
            ));
        }
        
        let intent_id = Uuid::new_v4();
        let legs = self.build_legs(&decision).await?;
        let ttl_seconds = self.calculate_ttl(&decision);
        let market_snapshot = self.create_market_snapshot().await?;
        let risk_factors = self.identify_risk_factors(&decision);
        
        let metadata = TradeMetadata {
            detected_at: decision.candidate.detected_at,
            detector: decision.candidate.strategy.clone(),
            market_snapshot,
            risk_factors,
        };
        
        Ok(TradeIntent {
            intent_id,
            strategy: decision.candidate.strategy.clone(),
            asset: decision.candidate.asset.clone(),
            size_usd: decision.evaluation.optimal_size_usd,
            expected_pnl_usd: decision.evaluation.net_pnl_usd,
            net_bps: decision.evaluation.net_bps,
            success_prob: decision.evaluation.success_prob,
            legs,
            ttl_seconds,
            created_at: Utc::now(),
            metadata,
        })
    }
    
    async fn build_legs(&self, decision: &TradeDecision) -> Result<Vec<TradeLeg>> {
        let mut legs = Vec::new();
        let now = Utc::now();
        
        for step in &decision.evaluation.execution_path {
            let chain = if step.domain.contains("Ethereum") {
                qenus_dataplane::Chain::Ethereum
            } else if step.domain.contains("Arbitrum") {
                qenus_dataplane::Chain::Arbitrum
            } else if step.domain.contains("Optimism") {
                qenus_dataplane::Chain::Optimism
            } else {
                qenus_dataplane::Chain::Base
            };
            
            let action = if step.action.contains("bridge") {
                TradeAction::Bridge
            } else {
                TradeAction::Swap
            };
            
            let deadline = now + Duration::seconds(if matches!(action, TradeAction::Bridge) { 300 } else { 30 });
            let min_amount_out = step.amount_out * (1.0 - (step.slippage_bps + 10.0) / 10000.0);
            let max_fee_bps = if step.protocol.contains("curve") { 10 } else { 30 };
            
            legs.push(TradeLeg {
                domain: chain,
                action,
                protocol: step.protocol.clone(),
                asset_in: decision.candidate.asset.clone(),
                asset_out: decision.candidate.asset.clone(),
                amount_in: format!("{:.6}", step.amount_in),
                min_amount_out: format!("{:.6}", min_amount_out),
                max_fee_bps,
                deadline,
                expected_out: format!("{:.6}", step.amount_out),
            });
        }
        
        Ok(legs)
    }
    
    fn calculate_ttl(&self, decision: &TradeDecision) -> u64 {
        match decision.candidate.strategy.as_str() {
            "dex_arb" => 30,
            "triangle_arb" => 120,
            _ => 60,
        }
    }
    
    async fn create_market_snapshot(&self) -> Result<MarketSnapshot> {
        let mut gas_prices = HashMap::new();
        let mut sequencer_health = HashMap::new();
        
        for chain in &[qenus_dataplane::Chain::Ethereum, qenus_dataplane::Chain::Arbitrum] {
            if let Some(gas_price) = self.market_state.get_gas_price(*chain).await {
                gas_prices.insert(format!("{:?}", chain), gas_price);
            }
            
            let health = if self.market_state.is_sequencer_healthy(*chain).await {
                "healthy"
            } else {
                "unhealthy"
            };
            sequencer_health.insert(format!("{:?}", chain), health.to_string());
        }
        
        Ok(MarketSnapshot {
            gas_prices,
            sequencer_health,
            volatility: 0.5,
        })
    }
    
    fn identify_risk_factors(&self, decision: &TradeDecision) -> Vec<RiskFactor> {
        let mut risks = Vec::new();
        
        let gas_pct = if decision.evaluation.net_pnl_usd > 0.0 {
            (decision.evaluation.costs.gas_usd / decision.evaluation.net_pnl_usd) * 100.0
        } else {
            0.0
        };
        
        if gas_pct > 30.0 {
            risks.push(RiskFactor {
                factor: "high_gas_cost".to_string(),
                severity: RiskSeverity::Medium,
                message: format!("Gas is {:.1}% of profit", gas_pct),
            });
        }
        
        if decision.evaluation.success_prob < 0.85 {
            risks.push(RiskFactor {
                factor: "low_success_prob".to_string(),
                severity: RiskSeverity::Low,
                message: format!("Success probability: {:.2}", decision.evaluation.success_prob),
            });
        }
        
        risks
    }
}

impl Default for IntentBuilder {
    fn default() -> Self {
        Self::new(Arc::new(MarketState::default()))
    }
}

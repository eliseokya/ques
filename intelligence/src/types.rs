//! Core types for the Intelligence layer

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use qenus_dataplane::Chain;

/// Trade intent - output of Intelligence layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeIntent {
    /// Unique intent ID for correlation
    pub intent_id: Uuid,
    
    /// Strategy name
    pub strategy: String,
    
    /// Asset being traded
    pub asset: String,
    
    /// Trade size in USD
    pub size_usd: f64,
    
    /// Expected profit in USD
    pub expected_pnl_usd: f64,
    
    /// Net profit in basis points
    pub net_bps: f64,
    
    /// Success probability (0.0 to 1.0)
    pub success_prob: f64,
    
    /// Execution legs
    pub legs: Vec<TradeLeg>,
    
    /// Time-to-live in seconds
    pub ttl_seconds: u64,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Metadata
    pub metadata: TradeMetadata,
}

/// Single leg of a multi-step trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLeg {
    /// Execution domain (chain)
    pub domain: Chain,
    
    /// Action type
    pub action: TradeAction,
    
    /// Protocol/venue
    pub protocol: String,
    
    /// Input asset
    pub asset_in: String,
    
    /// Output asset
    pub asset_out: String,
    
    /// Amount in
    pub amount_in: String,
    
    /// Minimum amount out (slippage protection)
    pub min_amount_out: String,
    
    /// Maximum fee in basis points
    pub max_fee_bps: u32,
    
    /// Deadline timestamp
    pub deadline: DateTime<Utc>,
    
    /// Expected output
    pub expected_out: String,
}

/// Trade action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAction {
    /// Swap on DEX
    Swap,
    
    /// Bridge to another chain
    Bridge,
    
    /// Take flash loan
    FlashLoan,
    
    /// Repay flash loan
    FlashRepay,
    
    /// Add liquidity
    AddLiquidity,
    
    /// Remove liquidity
    RemoveLiquidity,
}

/// Trade metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeMetadata {
    /// Detected at timestamp
    pub detected_at: DateTime<Utc>,
    
    /// Detector that found this
    pub detector: String,
    
    /// Market conditions snapshot
    pub market_snapshot: MarketSnapshot,
    
    /// Risk factors
    pub risk_factors: Vec<RiskFactor>,
}

/// Market conditions snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// Gas prices by chain
    pub gas_prices: std::collections::HashMap<String, f64>,
    
    /// Sequencer health by chain
    pub sequencer_health: std::collections::HashMap<String, String>,
    
    /// Market volatility indicator
    pub volatility: f64,
}

/// Risk factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor: String,
    pub severity: RiskSeverity,
    pub message: String,
}

/// Risk severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Arbitrage candidate detected by detectors
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Strategy type
    pub strategy: String,
    
    /// Primary asset
    pub asset: String,
    
    /// Spread in basis points
    pub spread_bps: f64,
    
    /// Execution path (domain hops)
    pub legs: Vec<(String, String)>, // (domain, action)
    
    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
    
    /// Confidence score
    pub confidence: f64,
}

/// Evaluation result from simulator
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// Net profit in USD
    pub net_pnl_usd: f64,
    
    /// Net profit in basis points
    pub net_bps: f64,
    
    /// Optimal trade size
    pub optimal_size_usd: f64,
    
    /// Success probability
    pub success_prob: f64,
    
    /// Breakdown of costs
    pub costs: CostBreakdown,
    
    /// Simulated execution path
    pub execution_path: Vec<SimulatedStep>,
}

/// Cost breakdown
#[derive(Debug, Clone)]
pub struct CostBreakdown {
    /// Gas costs in USD
    pub gas_usd: f64,
    
    /// DEX/Protocol fees in USD
    pub protocol_fees_usd: f64,
    
    /// Bridge fees in USD
    pub bridge_fees_usd: f64,
    
    /// Flash loan fees in USD
    pub flashloan_fees_usd: f64,
    
    /// Slippage cost in USD
    pub slippage_usd: f64,
    
    /// Total costs in USD
    pub total_usd: f64,
}

/// Simulated execution step
#[derive(Debug, Clone)]
pub struct SimulatedStep {
    pub step: usize,
    pub action: String,
    pub domain: String,
    pub protocol: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub slippage_bps: f64,
    pub cost_usd: f64,
}

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Strategy name
    pub name: String,
    
    /// Whether strategy is enabled
    pub enabled: bool,
    
    /// Minimum profit threshold in USD
    pub min_profit_usd: f64,
    
    /// Minimum profit in basis points
    pub min_profit_bps: f64,
    
    /// Maximum position size in USD
    pub max_position_usd: f64,
    
    /// Approved assets
    pub approved_assets: Vec<String>,
    
    /// Approved chains
    pub approved_chains: Vec<Chain>,
    
    /// Risk limits
    pub risk_limits: RiskLimits,
}

/// Risk limits per strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimits {
    /// Maximum slippage tolerance in bps
    pub max_slippage_bps: f64,
    
    /// Maximum gas cost as % of profit
    pub max_gas_pct: f64,
    
    /// Maximum bridge latency in seconds
    pub max_bridge_latency_secs: u64,
    
    /// Minimum success probability
    pub min_success_prob: f64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_slippage_bps: 100.0,   // 1% max slippage
            max_gas_pct: 50.0,          // Gas can be up to 50% of profit
            max_bridge_latency_secs: 300, // 5 min max
            min_success_prob: 0.8,      // 80% min success probability
        }
    }
}


//! Core data types and schemas for the dataplane

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::Chain;

/// Block identifier - can be number, hash, or latest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockId {
    Number(u64),
    Hash(String),
    Latest,
}

/// Normalized block data across all chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub chain: Chain,
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: DateTime<Utc>,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee_per_gas: Option<u64>,
    pub transactions: Vec<Transaction>,
    pub logs: Vec<Log>,
}

/// Normalized transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: String, // Use string to avoid precision loss
    pub gas: u64,
    pub gas_price: Option<u64>,
    pub max_fee_per_gas: Option<u64>,
    pub max_priority_fee_per_gas: Option<u64>,
    pub nonce: u64,
    pub transaction_index: u64,
    pub input: String,
    pub status: Option<u64>,
}

/// Normalized log/event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_number: u64,
    pub transaction_hash: String,
    pub transaction_index: u64,
    pub log_index: u64,
    pub removed: bool,
}

/// Unified feature schema - the core data structure consumed by Intelligence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    /// Unique identifier for this feature event
    pub id: Uuid,
    
    /// Block information
    pub block_number: u64,
    pub chain: Chain,
    pub timestamp: DateTime<Utc>,
    
    /// Feature type and data
    pub feature_type: FeatureType,
    pub data: FeatureData,
    
    /// Metadata
    pub source: String, // Which observer/extractor generated this
    pub version: String, // Schema version for evolution
}

/// Types of features we extract
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FeatureType {
    Amm,
    Bridge,
    Gas,
    FlashLoan,
    SequencerHealth,
}

/// Feature data payload - extensible union type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureData {
    Amm(AmmFeature),
    Bridge(BridgeFeature),
    Gas(GasFeature),
    FlashLoan(FlashLoanFeature),
    SequencerHealth(SequencerHealthFeature),
}

/// AMM pool state and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmFeature {
    pub pool_address: String,
    pub pool_type: String, // "uniswap_v3", "curve", "balancer", etc.
    pub token0: TokenInfo,
    pub token1: TokenInfo,
    pub fee_tier: Option<u32>, // Fee in basis points
    pub reserves: HashMap<String, String>, // token -> amount (string for precision)
    pub mid_price: f64,
    pub liquidity: String, // Total liquidity (string for precision)
    pub depth: DepthCurve,
    pub volume_24h: Option<String>,
    pub fees_24h: Option<String>,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

/// Slippage depth curve for different trade sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthCurve {
    pub sizes: HashMap<String, SlippageInfo>, // "100k", "1m", "10m" -> slippage
}

/// Slippage information for a given trade size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlippageInfo {
    pub slippage_bps: f64, // Slippage in basis points
    pub price_impact: f64, // Price impact percentage
}

/// Bridge state and fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeFeature {
    pub bridge_address: String,
    pub bridge_type: String, // "canonical", "hop", "across", "stargate"
    pub source_chain: Chain,
    pub dest_chain: Chain,
    pub token: TokenInfo,
    pub liquidity: String,
    pub fee_bps: u32,
    pub settlement_time_estimate: u64, // seconds
    pub is_active: bool,
}

/// Gas pricing model and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasFeature {
    pub base_fee: f64, // gwei
    pub priority_fee: f64, // gwei
    pub gas_used_ratio: f64, // 0.0 to 1.0
    pub next_base_fee_estimate: f64, // gwei
    pub fast_gas_price: f64, // gwei for fast confirmation
    pub standard_gas_price: f64, // gwei for standard confirmation
    pub safe_gas_price: f64, // gwei for safe confirmation
    pub pending_tx_count: u64,
}

/// Flash loan availability and pricing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanFeature {
    pub provider: String, // "aave_v3", "balancer", "dydx"
    pub provider_address: String,
    pub asset: TokenInfo,
    pub available_liquidity: String,
    pub fee_bps: u32,
    pub max_loan_amount: String,
    pub is_active: bool,
}

/// L2 sequencer health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerHealthFeature {
    pub sequencer_address: String,
    pub status: SequencerStatus,
    pub block_interval_avg: f64, // seconds
    pub block_interval_variance: f64,
    pub uptime_percentage: f64, // 0.0 to 100.0
    pub last_block_time: DateTime<Utc>,
    pub pending_tx_count: u64,
}

/// Sequencer operational status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SequencerStatus {
    Healthy,
    Degraded,
    Down,
    Unknown,
}

/// Configuration for RPC providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub max_requests_per_second: u32,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub is_websocket: bool,
}

/// Health check result for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub component: String,
    pub status: ComponentStatus,
    pub last_check: DateTime<Utc>,
    pub message: Option<String>,
    pub metrics: HashMap<String, f64>,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Metrics for monitoring and observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub component: String,
    pub timestamp: DateTime<Utc>,
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, Vec<f64>>,
}

impl Feature {
    /// Create a new feature with generated ID and current timestamp
    pub fn new(
        block_number: u64,
        chain: Chain,
        feature_type: FeatureType,
        data: FeatureData,
        source: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            block_number,
            chain,
            timestamp: Utc::now(),
            feature_type,
            data,
            source,
            version: "1.0".to_string(),
        }
    }

    /// Get the feature type as a string
    pub fn type_name(&self) -> &'static str {
        match self.feature_type {
            FeatureType::Amm => "amm",
            FeatureType::Bridge => "bridge",
            FeatureType::Gas => "gas",
            FeatureType::FlashLoan => "flash_loan",
            FeatureType::SequencerHealth => "sequencer_health",
        }
    }

    /// Validate the feature data consistency
    pub fn validate(&self) -> crate::Result<()> {
        // Basic validation - can be extended
        if self.block_number == 0 {
            return Err(crate::DataplaneError::schema_validation(
                "Block number cannot be zero",
            ));
        }

        // Type-specific validation
        match &self.data {
            FeatureData::Amm(amm) => {
                if amm.reserves.is_empty() {
                    return Err(crate::DataplaneError::schema_validation(
                        "AMM reserves cannot be empty",
                    ));
                }
            }
            FeatureData::Bridge(bridge) => {
                if bridge.fee_bps > 10000 {
                    return Err(crate::DataplaneError::schema_validation(
                        "Bridge fee cannot exceed 100%",
                    ));
                }
            }
            FeatureData::Gas(gas) => {
                if gas.base_fee < 0.0 {
                    return Err(crate::DataplaneError::schema_validation(
                        "Base fee cannot be negative",
                    ));
                }
            }
            FeatureData::FlashLoan(flash_loan) => {
                if flash_loan.fee_bps > 10000 {
                    return Err(crate::DataplaneError::schema_validation(
                        "Flash loan fee cannot exceed 100%",
                    ));
                }
            }
            FeatureData::SequencerHealth(health) => {
                if health.uptime_percentage > 100.0 {
                    return Err(crate::DataplaneError::schema_validation(
                        "Uptime percentage cannot exceed 100%",
                    ));
                }
            }
        }

        Ok(())
    }
}

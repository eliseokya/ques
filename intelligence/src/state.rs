//! Market state management
//!
//! Maintains a rolling view of market conditions across all chains by consuming
//! beta_dataplane features via Kafka or gRPC.
//! This is the Intelligence layer's memory of the market.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use qenus_dataplane::{Feature, FeatureData, Chain};
use tracing::debug;

use crate::error::{IntelligenceError, Result};

/// Market state manager - maintains rolling state from beta_dataplane features
pub struct MarketState {
    /// AMM pool states by chain and pool address
    amm_state: Arc<RwLock<HashMap<(Chain, String), AmmState>>>,
    
    /// Bridge states by chain pair
    bridge_state: Arc<RwLock<HashMap<(Chain, Chain), Vec<BridgeState>>>>,
    
    /// Gas states by chain
    gas_state: Arc<RwLock<HashMap<Chain, GasState>>>,
    
    /// Flash loan availability by chain and provider
    flashloan_state: Arc<RwLock<HashMap<(Chain, String), FlashLoanState>>>,
    
    /// Sequencer health by chain
    sequencer_state: Arc<RwLock<HashMap<Chain, SequencerState>>>,
    
    /// Time-to-live for cached states
    state_ttl: Duration,
    
    /// Last update time
    last_update: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

/// AMM pool state derived from beta_dataplane
#[derive(Debug, Clone)]
pub struct AmmState {
    pub pool_address: String,
    pub pool_type: String,
    pub token0_symbol: String,
    pub token1_symbol: String,
    pub mid_price: f64,
    pub liquidity: String,
    pub fee_tier: Option<u32>,
    /// Depth curve for slippage calculation
    pub depth: HashMap<String, (f64, f64)>, // size -> (slippage_bps, price_impact)
    pub last_update: DateTime<Utc>,
}

/// Bridge state derived from beta_dataplane
#[derive(Debug, Clone)]
pub struct BridgeState {
    pub bridge_address: String,
    pub bridge_type: String,
    pub token_symbol: String,
    pub liquidity: String,
    pub fee_bps: u32,
    pub settlement_time_secs: u64,
    pub is_active: bool,
    pub last_update: DateTime<Utc>,
}

/// Gas state derived from beta_dataplane
#[derive(Debug, Clone)]
pub struct GasState {
    pub base_fee: f64,
    pub priority_fee: f64,
    pub fast_gas_price: f64,
    pub standard_gas_price: f64,
    pub gas_used_ratio: f64,
    pub pending_tx_count: u64,
    pub last_update: DateTime<Utc>,
}

/// Flash loan state derived from beta_dataplane
#[derive(Debug, Clone)]
pub struct FlashLoanState {
    pub provider: String,
    pub provider_address: String,
    pub asset_symbol: String,
    pub available_liquidity: String,
    pub fee_bps: u32,
    pub max_loan_amount: String,
    pub is_active: bool,
    pub last_update: DateTime<Utc>,
}

/// Sequencer state derived from beta_dataplane
#[derive(Debug, Clone)]
pub struct SequencerState {
    pub status: String, // "healthy", "degraded", "down"
    pub block_interval_avg: f64,
    pub uptime_percentage: f64,
    pub pending_tx_count: u64,
    pub last_update: DateTime<Utc>,
}

impl MarketState {
    /// Create a new market state manager
    pub fn new(state_ttl_secs: i64) -> Self {
        Self {
            amm_state: Arc::new(RwLock::new(HashMap::new())),
            bridge_state: Arc::new(RwLock::new(HashMap::new())),
            gas_state: Arc::new(RwLock::new(HashMap::new())),
            flashloan_state: Arc::new(RwLock::new(HashMap::new())),
            sequencer_state: Arc::new(RwLock::new(HashMap::new())),
            state_ttl: Duration::seconds(state_ttl_secs),
            last_update: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Ingest a feature from beta_dataplane
    pub async fn ingest_feature(&self, feature: Feature) -> Result<()> {
        // Validate feature
        feature.validate().map_err(|e| {
            IntelligenceError::DataIngestion {
                message: format!("Feature validation failed: {}", e),
            }
        })?;
        
        debug!(
            "Ingesting feature: type={}, chain={:?}, block={}",
            feature.type_name(),
            feature.chain,
            feature.block_number
        );
        
        // Save metadata before moving feature.data
        let chain = feature.chain;
        let type_name = feature.type_name().to_string();
        let timestamp = feature.timestamp;
        
        // Route to appropriate state handler
        match feature.data {
            FeatureData::Amm(amm_data) => {
                self.update_amm_state(chain, amm_data, timestamp).await?;
            }
            FeatureData::Bridge(bridge_data) => {
                self.update_bridge_state(chain, bridge_data, timestamp).await?;
            }
            FeatureData::Gas(gas_data) => {
                self.update_gas_state(chain, gas_data, timestamp).await?;
            }
            FeatureData::FlashLoan(flashloan_data) => {
                self.update_flashloan_state(chain, flashloan_data, timestamp).await?;
            }
            FeatureData::SequencerHealth(seq_data) => {
                self.update_sequencer_state(chain, seq_data, timestamp).await?;
            }
        }
        
        // Update last update time
        let key = format!("{:?}_{}", chain, type_name);
        let mut last_update = self.last_update.write().await;
        last_update.insert(key, timestamp);
        
        Ok(())
    }
    
    /// Update AMM state
    async fn update_amm_state(
        &self,
        chain: Chain,
        amm_data: qenus_dataplane::AmmFeature,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let mut amm_state = self.amm_state.write().await;
        
        let state = AmmState {
            pool_address: amm_data.pool_address.clone(),
            pool_type: amm_data.pool_type,
            token0_symbol: amm_data.token0.symbol,
            token1_symbol: amm_data.token1.symbol,
            mid_price: amm_data.mid_price,
            liquidity: amm_data.liquidity,
            fee_tier: amm_data.fee_tier,
            depth: amm_data.depth.sizes.into_iter().map(|(k, v)| {
                (k, (v.slippage_bps, v.price_impact))
            }).collect(),
            last_update: timestamp,
        };
        
        amm_state.insert((chain, amm_data.pool_address), state);
        Ok(())
    }
    
    /// Update bridge state
    async fn update_bridge_state(
        &self,
        chain: Chain,
        bridge_data: qenus_dataplane::BridgeFeature,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let mut bridge_state = self.bridge_state.write().await;
        
        let state = BridgeState {
            bridge_address: bridge_data.bridge_address,
            bridge_type: bridge_data.bridge_type,
            token_symbol: bridge_data.token.symbol,
            liquidity: bridge_data.liquidity,
            fee_bps: bridge_data.fee_bps,
            settlement_time_secs: bridge_data.settlement_time_estimate,
            is_active: bridge_data.is_active,
            last_update: timestamp,
        };
        
        let key = (bridge_data.source_chain, bridge_data.dest_chain);
        bridge_state.entry(key).or_insert_with(Vec::new).push(state);
        
        Ok(())
    }
    
    /// Update gas state
    async fn update_gas_state(
        &self,
        chain: Chain,
        gas_data: qenus_dataplane::GasFeature,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let mut gas_state = self.gas_state.write().await;
        
        let state = GasState {
            base_fee: gas_data.base_fee,
            priority_fee: gas_data.priority_fee,
            fast_gas_price: gas_data.fast_gas_price,
            standard_gas_price: gas_data.standard_gas_price,
            gas_used_ratio: gas_data.gas_used_ratio,
            pending_tx_count: gas_data.pending_tx_count,
            last_update: timestamp,
        };
        
        gas_state.insert(chain, state);
        Ok(())
    }
    
    /// Update flash loan state
    async fn update_flashloan_state(
        &self,
        chain: Chain,
        flashloan_data: qenus_dataplane::FlashLoanFeature,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let mut flashloan_state = self.flashloan_state.write().await;
        
        let state = FlashLoanState {
            provider: flashloan_data.provider.clone(),
            provider_address: flashloan_data.provider_address,
            asset_symbol: flashloan_data.asset.symbol,
            available_liquidity: flashloan_data.available_liquidity,
            fee_bps: flashloan_data.fee_bps,
            max_loan_amount: flashloan_data.max_loan_amount,
            is_active: flashloan_data.is_active,
            last_update: timestamp,
        };
        
        flashloan_state.insert((chain, flashloan_data.provider), state);
        Ok(())
    }
    
    /// Update sequencer state
    async fn update_sequencer_state(
        &self,
        chain: Chain,
        seq_data: qenus_dataplane::SequencerHealthFeature,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let mut sequencer_state = self.sequencer_state.write().await;
        
        let status = match seq_data.status {
            qenus_dataplane::SequencerStatus::Healthy => "healthy",
            qenus_dataplane::SequencerStatus::Degraded => "degraded",
            qenus_dataplane::SequencerStatus::Down => "down",
            qenus_dataplane::SequencerStatus::Unknown => "unknown",
        };
        
        let state = SequencerState {
            status: status.to_string(),
            block_interval_avg: seq_data.block_interval_avg,
            uptime_percentage: seq_data.uptime_percentage,
            pending_tx_count: seq_data.pending_tx_count,
            last_update: timestamp,
        };
        
        sequencer_state.insert(chain, state);
        Ok(())
    }
    
    /// Get price for an asset on a specific chain (from AMM state)
    pub async fn get_price(&self, chain: Chain, asset: &str) -> Option<f64> {
        let amm_state = self.amm_state.read().await;
        
        // Find any pool containing this asset and return mid price
        for ((pool_chain, _), state) in amm_state.iter() {
            if *pool_chain == chain {
                if state.token0_symbol == asset || state.token1_symbol == asset {
                    if !self.is_stale(&state.last_update) {
                        return Some(state.mid_price);
                    }
                }
            }
        }
        
        None
    }
    
    /// Get slippage for a trade size
    pub async fn get_slippage(&self, chain: Chain, pool_address: &str, size_usd: &str) -> Option<f64> {
        let amm_state = self.amm_state.read().await;
        
        if let Some(state) = amm_state.get(&(chain, pool_address.to_string())) {
            if !self.is_stale(&state.last_update) {
                return state.depth.get(size_usd).map(|(slippage_bps, _)| *slippage_bps);
            }
        }
        
        None
    }
    
    /// Get gas price for a chain
    pub async fn get_gas_price(&self, chain: Chain) -> Option<f64> {
        let gas_state = self.gas_state.read().await;
        
        if let Some(state) = gas_state.get(&chain) {
            if !self.is_stale(&state.last_update) {
                return Some(state.fast_gas_price);
            }
        }
        
        None
    }
    
    /// Get bridge fee between chains
    pub async fn get_bridge_fee(&self, from_chain: Chain, to_chain: Chain, _asset: &str) -> Option<u32> {
        let bridge_state = self.bridge_state.read().await;
        
        if let Some(bridges) = bridge_state.get(&(from_chain, to_chain)) {
            // Return the best (lowest) fee from active bridges
            bridges.iter()
                .filter(|b| b.is_active && !self.is_stale(&b.last_update))
                .map(|b| b.fee_bps)
                .min()
        } else {
            None
        }
    }
    
    /// Get flash loan liquidity
    pub async fn get_flashloan_liquidity(&self, chain: Chain, asset: &str) -> Option<String> {
        let flashloan_state = self.flashloan_state.read().await;
        
        // Find any active provider with this asset
        for ((fl_chain, _), state) in flashloan_state.iter() {
            if *fl_chain == chain && state.asset_symbol == asset {
                if state.is_active && !self.is_stale(&state.last_update) {
                    return Some(state.available_liquidity.clone());
                }
            }
        }
        
        None
    }
    
    /// Check if sequencer is healthy
    pub async fn is_sequencer_healthy(&self, chain: Chain) -> bool {
        let sequencer_state = self.sequencer_state.read().await;
        
        if let Some(state) = sequencer_state.get(&chain) {
            if !self.is_stale(&state.last_update) {
                return state.status == "healthy";
            }
        }
        
        // Default to unhealthy if no recent data
        false
    }
    
    /// Get all AMM pools for a chain
    pub async fn get_amm_pools(&self, chain: Chain) -> Vec<AmmState> {
        let amm_state = self.amm_state.read().await;
        
        amm_state.iter()
            .filter(|((pool_chain, _), state)| {
                *pool_chain == chain && !self.is_stale(&state.last_update)
            })
            .map(|(_, state)| state.clone())
            .collect()
    }
    
    /// Get all bridges between two chains
    pub async fn get_bridges(&self, from_chain: Chain, to_chain: Chain) -> Vec<BridgeState> {
        let bridge_state = self.bridge_state.read().await;
        
        if let Some(bridges) = bridge_state.get(&(from_chain, to_chain)) {
            bridges.iter()
                .filter(|b| !self.is_stale(&b.last_update))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Check if state is stale
    fn is_stale(&self, last_update: &DateTime<Utc>) -> bool {
        Utc::now() - *last_update > self.state_ttl
    }
    
    /// Check if feed is stale for a specific chain/type
    pub async fn is_feed_stale(&self, chain: Chain, feature_type: &str) -> bool {
        let last_update = self.last_update.read().await;
        
        if let Some(timestamp) = last_update.get(&format!("{:?}_{}", chain, feature_type)) {
            self.is_stale(timestamp)
        } else {
            true // No data yet = stale
        }
    }
    
    /// Get state statistics for monitoring
    pub async fn get_stats(&self) -> MarketStateStats {
        let amm_state = self.amm_state.read().await;
        let bridge_state = self.bridge_state.read().await;
        let gas_state = self.gas_state.read().await;
        let flashloan_state = self.flashloan_state.read().await;
        let sequencer_state = self.sequencer_state.read().await;
        
        MarketStateStats {
            total_amm_pools: amm_state.len(),
            total_bridges: bridge_state.values().map(|v| v.len()).sum(),
            total_gas_states: gas_state.len(),
            total_flashloan_providers: flashloan_state.len(),
            total_sequencers: sequencer_state.len(),
        }
    }
}

impl Default for MarketState {
    fn default() -> Self {
        Self::new(30) // 30 seconds default TTL
    }
}

/// Statistics about the market state
#[derive(Debug, Clone)]
pub struct MarketStateStats {
    pub total_amm_pools: usize,
    pub total_bridges: usize,
    pub total_gas_states: usize,
    pub total_flashloan_providers: usize,
    pub total_sequencers: usize,
}

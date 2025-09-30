//! Market state management
//!
//! Maintains rolling market state from dataplane features.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use qenus_dataplane::{Feature, Chain};

use crate::{Result, IntelligenceError};

/// Market state aggregator
pub struct MarketState {
    /// Latest features by type and chain
    features: Arc<RwLock<HashMap<String, Feature>>>,
    
    /// Last update timestamp
    last_update: Arc<RwLock<DateTime<Utc>>>,
}

impl MarketState {
    /// Create a new market state
    pub fn new() -> Self {
        Self {
            features: Arc::new(RwLock::new(HashMap::new())),
            last_update: Arc::new(RwLock::new(Utc::now())),
        }
    }

    /// Update state with a new feature
    pub async fn update(&self, feature: Feature) {
        let key = format!("{}_{:?}", feature.chain, feature.feature_type);
        let mut features = self.features.write().await;
        features.insert(key, feature);
        
        let mut last_update = self.last_update.write().await;
        *last_update = Utc::now();
    }

    /// Get price for an asset on a chain
    pub async fn get_price(&self, chain: Chain, asset: &str) -> Option<f64> {
        // TODO: Implement price lookup from AMM features
        None
    }
}

impl Default for MarketState {
    fn default() -> Self {
        Self::new()
    }
}


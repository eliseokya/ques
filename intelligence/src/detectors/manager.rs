//! Detector manager - orchestrates all detectors

use std::sync::Arc;
use tracing::{info, warn};

use crate::error::Result;
use crate::state::MarketState;
use crate::types::{Candidate, StrategyConfig};
use crate::detectors::{TriangleArbDetector, dex_arb::DexArbDetector};

/// Detector manager - orchestrates all detectors
pub struct DetectorManager {
    triangle_detector: Option<TriangleArbDetector>,
    dex_detector: Option<DexArbDetector>,
}

impl DetectorManager {
    /// Create a new detector manager
    pub fn new(
        triangle_config: Option<StrategyConfig>,
        dex_config: Option<StrategyConfig>,
        market_state: Arc<MarketState>,
    ) -> Self {
        Self {
            triangle_detector: triangle_config.map(|cfg| {
                TriangleArbDetector::new(cfg, market_state.clone())
            }),
            dex_detector: dex_config.map(|cfg| {
                DexArbDetector::new(cfg, market_state)
            }),
        }
    }
    
    /// Run all enabled detectors
    pub async fn detect_all(&self) -> Result<Vec<Candidate>> {
        let mut all_candidates = Vec::new();
        
        // Run triangle arb detector
        if let Some(detector) = &self.triangle_detector {
            match detector.detect().await {
                Ok(candidates) => all_candidates.extend(candidates),
                Err(e) => warn!("Triangle arb detector failed: {}", e),
            }
        }
        
        // Run DEX arb detector
        if let Some(detector) = &self.dex_detector {
            match detector.detect().await {
                Ok(candidates) => all_candidates.extend(candidates),
                Err(e) => warn!("DEX arb detector failed: {}", e),
            }
        }
        
        info!("Detected {} total candidates", all_candidates.len());
        Ok(all_candidates)
    }
}


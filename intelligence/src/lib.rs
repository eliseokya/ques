//! Qenus Intelligence Layer
//!
//! The brain of the arbitrage system - converts raw on-chain data into
//! executable trade intents with risk management and profitability analysis.
//!
//! ## Inputs:
//! 1. **beta_dataplane**: Live market data (prices, liquidity, gas, flash loans)
//! 2. **business**: Strategy configs, risk policies, asset lists
//!
//! ## Output:
//! - **TradeIntent**: Executable trade specifications for Orchestration layer

pub mod state;
pub mod detectors;
pub mod simulator;
pub mod decision;
pub mod intent_builder;
pub mod feedback;
pub mod ingestion;
pub mod error;
pub mod types;
pub mod config;

pub use error::{IntelligenceError, Result};
pub use types::*;
pub use state::{MarketState, MarketStateStats, AmmState, BridgeState, GasState, FlashLoanState, SequencerState};
pub use detectors::{TriangleArbDetector, DexArbDetector, DetectorManager};
pub use ingestion::FeatureIngestionManager;
pub use config::{IntelligenceConfig, DataplaneConnectionConfig, DetectionConfig};
pub use simulator::TradeSimulator;

/// Version of the intelligence layer
pub const VERSION: &str = env!("CARGO_PKG_VERSION");


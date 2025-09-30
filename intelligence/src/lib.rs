//! Qenus Intelligence Layer
//!
//! The brain of the arbitrage system - converts raw on-chain data into
//! executable trade intents with risk management and profitability analysis.

pub mod state;
pub mod detectors;
pub mod simulator;
pub mod decision;
pub mod intent_builder;
pub mod feedback;
pub mod error;
pub mod types;
pub mod config;

pub use error::{IntelligenceError, Result};
pub use types::*;

/// Version of the intelligence layer
pub const VERSION: &str = env!("CARGO_PKG_VERSION");


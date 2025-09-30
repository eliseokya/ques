//! Trade simulation and profit calculation

pub mod amm;
pub mod gas;
pub mod bridge;
pub mod flashloan;
pub mod evaluator;

pub use evaluator::TradeSimulator;


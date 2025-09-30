//! Utility functions for beta dataplane
//!
//! Shared utilities for contract interactions, mathematical calculations,
//! data validation, and retry logic.

pub mod contracts;
pub mod math;
pub mod validation;
pub mod retry;

// Re-export commonly used types
pub use contracts::{ContractRegistry, AbiManager, UniswapV3Slot0};
// TODO: Implement remaining utilities
// pub use math::{PriceCalculator, SlippageCalculator, LiquidityCalculator};
// pub use validation::{DataValidator, SchemaValidator};
// pub use retry::{RetryPolicy, ExponentialBackoff};

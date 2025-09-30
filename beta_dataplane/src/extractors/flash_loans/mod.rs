//! Flash loan protocol extractors
//!
//! Extracts flash loan availability and pricing across protocols.

pub mod aave_v3;
pub mod balancer;

// Re-export extractors
pub use aave_v3::AaveV3FlashLoanExtractor;
pub use balancer::BalancerFlashLoanExtractor;

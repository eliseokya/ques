//! Bridge protocol extractors
//!
//! Extracts cross-chain bridge state and liquidity information.

pub mod canonical;
pub mod hop;
pub mod across;

// Re-export extractors
pub use canonical::CanonicalBridgeExtractor;
pub use hop::HopBridgeExtractor;
pub use across::AcrossBridgeExtractor;

//! AMM (Automated Market Maker) feature extractors
//!
//! Extracts real-time pool state, pricing, and liquidity information
//! from major DEX protocols across all supported chains.

pub mod uniswap_v3;
pub mod curve;
pub mod balancer;

// Re-export extractors
pub use uniswap_v3::UniswapV3Extractor;
pub use curve::CurveExtractor;
pub use balancer::BalancerExtractor;

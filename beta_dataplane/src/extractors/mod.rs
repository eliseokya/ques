//! Feature extractors for beta dataplane
//!
//! RPC-based feature extraction that produces the same output schema
//! as the full dataplane for seamless Intelligence layer integration.

pub mod traits;
pub mod amm;
pub mod bridges;
pub mod gas;
pub mod flash_loans;

// Re-export commonly used types
pub use traits::{BetaFeatureExtractor, ExtractionContext, ExtractionResult, ExtractorConfig, ExtractionMetadata};

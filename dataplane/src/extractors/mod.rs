//! Feature extractors for converting raw blockchain data into actionable metrics

// TODO: Implement specific extractors
// pub mod amm;
// pub mod bridge;
// pub mod gas;
// pub mod flash_loan;
// pub mod sequencer;
pub mod traits;

// Re-export commonly used types
pub use traits::{FeatureExtractor, ExtractorContext, ExtractorResult};

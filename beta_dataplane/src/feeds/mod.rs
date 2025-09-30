//! Data feeds for beta dataplane
//!
//! Provides identical output interfaces to the full dataplane,
//! enabling seamless migration and Intelligence layer integration.

pub mod traits;
pub mod kafka;
pub mod grpc;
pub mod parquet;

// Re-export commonly used types (TODO: Implement in Phase 5)
// pub use traits::{BetaDataFeed, BetaFeedConfig, BetaFeedMetrics, BetaFeedHealth};

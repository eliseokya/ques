//! Data feeds for delivering features to the Intelligence layer

// TODO: Implement specific feeds
// pub mod kafka;
// pub mod grpc;
// pub mod parquet;
pub mod traits;

// Re-export commonly used types
pub use traits::{DataFeed, FeedConfig, FeedMetrics, FeedHealth};

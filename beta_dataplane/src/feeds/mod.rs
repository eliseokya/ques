//! Data feeds for beta dataplane
//!
//! Provides identical output interfaces to the full dataplane,
//! enabling seamless migration and Intelligence layer integration.

pub mod traits;
pub mod kafka;
pub mod grpc;
pub mod parquet;
pub mod manager;

pub use traits::{
    BetaDataFeed, BetaFeedConfig, BetaFeedHealth, BetaFeedMetrics,
    CompressionAlgorithm, CompressionConfig, ConnectionStatus, FeedPerformance, FeedStatus,
    RetryConfig,
};
pub use kafka::KafkaFeed;
pub use grpc::GrpcFeed;
pub use parquet::ParquetFeed;
pub use manager::FeedManager;

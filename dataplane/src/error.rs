//! Error types for the dataplane module

use thiserror::Error;

/// Result type alias for dataplane operations
pub type Result<T> = std::result::Result<T, DataplaneError>;

/// Comprehensive error types for dataplane operations
#[derive(Error, Debug)]
pub enum DataplaneError {
    #[error("Invalid chain: {0}")]
    InvalidChain(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    #[error("Ethereum client error: {0}")]
    Ethereum(#[from] ethers::providers::ProviderError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),

    // TODO: Re-enable when Arrow/Parquet are added back
    // #[error("Arrow error: {0}")]
    // Arrow(#[from] arrow::error::ArrowError),

    // #[error("Parquet error: {0}")]
    // Parquet(#[from] parquet::errors::ParquetError),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Observer error: {message}")]
    Observer { message: String },

    #[error("Extractor error: {message}")]
    Extractor { message: String },

    #[error("Feed error: {message}")]
    Feed { message: String },

    #[error("Invalid block number: {0}")]
    InvalidBlockNumber(u64),

    #[error("Invalid transaction hash: {0}")]
    InvalidTransactionHash(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Rate limit exceeded for provider: {provider}")]
    RateLimit { provider: String },

    #[error("Connection timeout for chain: {chain}")]
    ConnectionTimeout { chain: String },

    #[error("Schema validation failed: {message}")]
    SchemaValidation { message: String },

    #[error("Feature extraction failed: {feature_type} - {message}")]
    FeatureExtraction {
        feature_type: String,
        message: String,
    },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl DataplaneError {
    /// Create a new observer error
    pub fn observer<S: Into<String>>(message: S) -> Self {
        Self::Observer {
            message: message.into(),
        }
    }

    /// Create a new extractor error
    pub fn extractor<S: Into<String>>(message: S) -> Self {
        Self::Extractor {
            message: message.into(),
        }
    }

    /// Create a new feed error
    pub fn feed<S: Into<String>>(message: S) -> Self {
        Self::Feed {
            message: message.into(),
        }
    }

    /// Create a new schema validation error
    pub fn schema_validation<S: Into<String>>(message: S) -> Self {
        Self::SchemaValidation {
            message: message.into(),
        }
    }

    /// Create a new feature extraction error
    pub fn feature_extraction<S: Into<String>>(feature_type: S, message: S) -> Self {
        Self::FeatureExtraction {
            feature_type: feature_type.into(),
            message: message.into(),
        }
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal(message.into())
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            DataplaneError::Network(_) => true,
            DataplaneError::WebSocket(_) => true,
            DataplaneError::ConnectionTimeout { .. } => true,
            DataplaneError::RateLimit { .. } => true,
            DataplaneError::Database(_) => true,
            DataplaneError::Redis(_) => true,
            DataplaneError::Kafka(_) => true,
            _ => false,
        }
    }

    /// Get the error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            DataplaneError::InvalidChain(_) => "validation",
            DataplaneError::Config(_) => "config",
            DataplaneError::Network(_) => "network",
            DataplaneError::WebSocket(_) => "network",
            DataplaneError::Ethereum(_) => "blockchain",
            DataplaneError::Serialization(_) => "serialization",
            DataplaneError::Database(_) => "database",
            DataplaneError::Redis(_) => "cache",
            DataplaneError::Kafka(_) => "messaging",
            // TODO: Re-enable when Arrow/Parquet are added back
            // DataplaneError::Arrow(_) => "data_processing",
            // DataplaneError::Parquet(_) => "data_processing",
            DataplaneError::Grpc(_) => "grpc",
            DataplaneError::Io(_) => "io",
            DataplaneError::Observer { .. } => "observer",
            DataplaneError::Extractor { .. } => "extractor",
            DataplaneError::Feed { .. } => "feed",
            DataplaneError::InvalidBlockNumber(_) => "validation",
            DataplaneError::InvalidTransactionHash(_) => "validation",
            DataplaneError::InvalidAddress(_) => "validation",
            DataplaneError::RateLimit { .. } => "rate_limit",
            DataplaneError::ConnectionTimeout { .. } => "timeout",
            DataplaneError::SchemaValidation { .. } => "validation",
            DataplaneError::FeatureExtraction { .. } => "feature_extraction",
            DataplaneError::Internal(_) => "internal",
        }
    }
}

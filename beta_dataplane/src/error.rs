//! Error types for the beta dataplane

use thiserror::Error;

/// Result type alias for beta dataplane operations
pub type Result<T> = std::result::Result<T, BetaDataplaneError>;

/// Comprehensive error types for beta dataplane operations
#[derive(Error, Debug)]
pub enum BetaDataplaneError {
    #[error("Invalid provider: {0}")]
    InvalidProvider(String),

    #[error("Invalid operational mode: {0}")]
    InvalidMode(String),

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

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Dataplane error: {0}")]
    Dataplane(#[from] qenus_dataplane::DataplaneError),

    #[error("Provider error: {provider} - {message}")]
    Provider { provider: String, message: String },

    #[error("Extractor error: {extractor} - {message}")]
    Extractor { extractor: String, message: String },

    #[error("Feed error: {feed} - {message}")]
    Feed { feed: String, message: String },

    #[error("Cache error: {message}")]
    Cache { message: String },

    #[error("Rate limit exceeded for provider: {provider}")]
    RateLimit { provider: String },

    #[error("Connection timeout for provider: {provider}")]
    ConnectionTimeout { provider: String },

    #[error("Provider unavailable: {provider} - {reason}")]
    ProviderUnavailable { provider: String, reason: String },

    #[error("Data validation failed: {message}")]
    DataValidation { message: String },

    #[error("Feature extraction failed: {feature_type} - {message}")]
    FeatureExtraction {
        feature_type: String,
        message: String,
    },

    #[error("Optimization error: {component} - {message}")]
    Optimization { component: String, message: String },

    #[error("Monitoring error: {component} - {message}")]
    Monitoring { component: String, message: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl BetaDataplaneError {
    /// Create a new provider error
    pub fn provider<S: Into<String>>(provider: S, message: S) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Create a new extractor error
    pub fn extractor<S: Into<String>>(extractor: S, message: S) -> Self {
        Self::Extractor {
            extractor: extractor.into(),
            message: message.into(),
        }
    }

    /// Create a new feed error
    pub fn feed<S: Into<String>>(feed: S, message: S) -> Self {
        Self::Feed {
            feed: feed.into(),
            message: message.into(),
        }
    }

    /// Create a new cache error
    pub fn cache<S: Into<String>>(message: S) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// Create a new data validation error
    pub fn data_validation<S: Into<String>>(message: S) -> Self {
        Self::DataValidation {
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

    /// Create a new optimization error
    pub fn optimization<S: Into<String>>(component: S, message: S) -> Self {
        Self::Optimization {
            component: component.into(),
            message: message.into(),
        }
    }

    /// Create a new monitoring error
    pub fn monitoring<S: Into<String>>(component: S, message: S) -> Self {
        Self::Monitoring {
            component: component.into(),
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
            BetaDataplaneError::Network(_) => true,
            BetaDataplaneError::WebSocket(_) => true,
            BetaDataplaneError::Ethereum(_) => true,
            BetaDataplaneError::ConnectionTimeout { .. } => true,
            BetaDataplaneError::ProviderUnavailable { .. } => true,
            BetaDataplaneError::Database(_) => true,
            BetaDataplaneError::Redis(_) => true,
            BetaDataplaneError::Kafka(_) => true,
            BetaDataplaneError::Grpc(_) => true,
            _ => false,
        }
    }

    /// Get the error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            BetaDataplaneError::InvalidProvider(_) => "validation",
            BetaDataplaneError::InvalidMode(_) => "validation",
            BetaDataplaneError::InvalidChain(_) => "validation",
            BetaDataplaneError::Config(_) => "config",
            BetaDataplaneError::Network(_) => "network",
            BetaDataplaneError::WebSocket(_) => "network",
            BetaDataplaneError::Ethereum(_) => "blockchain",
            BetaDataplaneError::Serialization(_) => "serialization",
            BetaDataplaneError::Database(_) => "database",
            BetaDataplaneError::Redis(_) => "cache",
            BetaDataplaneError::Kafka(_) => "messaging",
            BetaDataplaneError::Grpc(_) => "grpc",
            BetaDataplaneError::Io(_) => "io",
            BetaDataplaneError::Dataplane(_) => "dataplane",
            BetaDataplaneError::Provider { .. } => "provider",
            BetaDataplaneError::Extractor { .. } => "extractor",
            BetaDataplaneError::Feed { .. } => "feed",
            BetaDataplaneError::Cache { .. } => "cache",
            BetaDataplaneError::RateLimit { .. } => "rate_limit",
            BetaDataplaneError::ConnectionTimeout { .. } => "timeout",
            BetaDataplaneError::ProviderUnavailable { .. } => "provider_unavailable",
            BetaDataplaneError::DataValidation { .. } => "validation",
            BetaDataplaneError::FeatureExtraction { .. } => "feature_extraction",
            BetaDataplaneError::Optimization { .. } => "optimization",
            BetaDataplaneError::Monitoring { .. } => "monitoring",
            BetaDataplaneError::Internal(_) => "internal",
        }
    }

    /// Get the severity level for this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            BetaDataplaneError::InvalidProvider(_) => ErrorSeverity::High,
            BetaDataplaneError::InvalidMode(_) => ErrorSeverity::High,
            BetaDataplaneError::InvalidChain(_) => ErrorSeverity::High,
            BetaDataplaneError::Config(_) => ErrorSeverity::High,
            BetaDataplaneError::Network(_) => ErrorSeverity::Medium,
            BetaDataplaneError::WebSocket(_) => ErrorSeverity::Medium,
            BetaDataplaneError::Ethereum(_) => ErrorSeverity::Medium,
            BetaDataplaneError::ConnectionTimeout { .. } => ErrorSeverity::Medium,
            BetaDataplaneError::ProviderUnavailable { .. } => ErrorSeverity::Medium,
            BetaDataplaneError::RateLimit { .. } => ErrorSeverity::Low,
            BetaDataplaneError::DataValidation { .. } => ErrorSeverity::Medium,
            BetaDataplaneError::FeatureExtraction { .. } => ErrorSeverity::Medium,
            BetaDataplaneError::Internal(_) => ErrorSeverity::High,
            _ => ErrorSeverity::Low,
        }
    }
}

/// Error severity levels for alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
    /// Check if this severity requires immediate attention
    pub fn requires_immediate_attention(&self) -> bool {
        matches!(self, ErrorSeverity::High | ErrorSeverity::Critical)
    }

    /// Get the alert threshold for this severity
    pub fn alert_threshold(&self) -> u32 {
        match self {
            ErrorSeverity::Low => 100,      // Alert after 100 occurrences
            ErrorSeverity::Medium => 10,    // Alert after 10 occurrences
            ErrorSeverity::High => 1,       // Alert immediately
            ErrorSeverity::Critical => 1,   // Alert immediately
        }
    }
}

//! Error types for the Intelligence layer

use thiserror::Error;

/// Result type alias for Intelligence layer operations
pub type Result<T> = std::result::Result<T, IntelligenceError>;

/// Comprehensive error types for Intelligence layer
#[derive(Error, Debug)]
pub enum IntelligenceError {
    #[error("Invalid strategy: {0}")]
    InvalidStrategy(String),

    #[error("Invalid candidate: {0}")]
    InvalidCandidate(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    // #[error("Kafka error: {0}")]
    // Kafka(#[from] rdkafka::error::KafkaError),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Dataplane error: {0}")]
    Dataplane(#[from] qenus_dataplane::DataplaneError),

    #[error("State error: {message}")]
    State { message: String },

    #[error("Data ingestion error: {message}")]
    DataIngestion { message: String },

    #[error("Detection error: {message}")]
    Detection { message: String },

    #[error("Simulation error: {message}")]
    Simulation { message: String },

    #[error("Decision error: {message}")]
    Decision { message: String },

    #[error("Policy violation: {policy} - {message}")]
    PolicyViolation { policy: String, message: String },

    #[error("Insufficient liquidity: required={required}, available={available}")]
    InsufficientLiquidity { required: f64, available: f64 },

    #[error("Opportunity expired: {message}")]
    OpportunityExpired { message: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntelligenceError {
    /// Create a state error
    pub fn state<S: Into<String>>(message: S) -> Self {
        Self::State {
            message: message.into(),
        }
    }

    /// Create a detection error
    pub fn detection<S: Into<String>>(message: S) -> Self {
        Self::Detection {
            message: message.into(),
        }
    }

    /// Create a simulation error
    pub fn simulation<S: Into<String>>(message: S) -> Self {
        Self::Simulation {
            message: message.into(),
        }
    }

    /// Create a decision error
    pub fn decision<S: Into<String>>(message: S) -> Self {
        Self::Decision {
            message: message.into(),
        }
    }

    /// Create a policy violation error
    pub fn policy_violation<S: Into<String>>(policy: S, message: S) -> Self {
        Self::PolicyViolation {
            policy: policy.into(),
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal(message.into())
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            IntelligenceError::Grpc(_)
                | IntelligenceError::Database(_)
                | IntelligenceError::Redis(_)
        )
    }
}


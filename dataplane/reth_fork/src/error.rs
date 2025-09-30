//! Error types for the Qenus Reth fork

use thiserror::Error;

/// Result type alias for Qenus Reth operations
pub type Result<T> = std::result::Result<T, QenusRethError>;

/// Comprehensive error types for Qenus Reth operations
#[derive(Error, Debug)]
pub enum QenusRethError {
    #[error("Invalid chain configuration: {0}")]
    InvalidChain(String),

    #[error("Reth error: {0}")]
    Reth(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("State access error: {0}")]
    StateAccess(String),

    #[error("Feature extraction error: {0}")]
    FeatureExtraction(String),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Dataplane error: {0}")]
    Dataplane(#[from] qenus_dataplane::DataplaneError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl QenusRethError {
    /// Create a new database error
    pub fn database<S: Into<String>>(message: S) -> Self {
        Self::Database(message.into())
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network(message.into())
    }

    /// Create a new sync error
    pub fn sync<S: Into<String>>(message: S) -> Self {
        Self::Sync(message.into())
    }

    /// Create a new execution error
    pub fn execution<S: Into<String>>(message: S) -> Self {
        Self::Execution(message.into())
    }

    /// Create a new state access error
    pub fn state_access<S: Into<String>>(message: S) -> Self {
        Self::StateAccess(message.into())
    }

    /// Create a new feature extraction error
    pub fn feature_extraction<S: Into<String>>(message: S) -> Self {
        Self::FeatureExtraction(message.into())
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal(message.into())
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            QenusRethError::Network(_) => true,
            QenusRethError::Sync(_) => true,
            QenusRethError::Database(_) => true,
            QenusRethError::Io(_) => true,
            _ => false,
        }
    }

    /// Get the error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            QenusRethError::InvalidChain(_) => "config",
            QenusRethError::Reth(_) => "reth",
            QenusRethError::Database(_) => "database",
            QenusRethError::Network(_) => "network",
            QenusRethError::Sync(_) => "sync",
            QenusRethError::Execution(_) => "execution",
            QenusRethError::StateAccess(_) => "state",
            QenusRethError::FeatureExtraction(_) => "extraction",
            QenusRethError::Config(_) => "config",
            QenusRethError::Io(_) => "io",
            QenusRethError::Serialization(_) => "serialization",
            QenusRethError::Dataplane(_) => "dataplane",
            QenusRethError::Internal(_) => "internal",
        }
    }
}

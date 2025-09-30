//! # Qenus Beta Dataplane
//!
//! Production-ready RPC-based dataplane for immediate revenue generation.
//! Provides the same feature extraction and data feeds as the full dataplane,
//! but uses RPC providers for faster time-to-market.

pub mod config;
pub mod error;
pub mod providers;
pub mod extractors;
pub mod optimization;
pub mod feeds;
pub mod monitoring;
pub mod utils;

// Re-export commonly used types from parent dataplane
pub use qenus_dataplane::{Chain, Feature, FeatureData, FeatureType};

// Re-export beta-specific types
pub use config::BetaDataplaneConfig;
pub use error::{BetaDataplaneError, Result};

/// Current version of the beta dataplane
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Beta dataplane identifier
pub const SYSTEM_NAME: &str = "qenus-beta-dataplane";

/// Supported RPC provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProviderType {
    #[serde(rename = "alchemy")]
    Alchemy,
    #[serde(rename = "infura")]
    Infura,
    #[serde(rename = "quicknode")]
    QuickNode,
    #[serde(rename = "ankr")]
    Ankr,
    #[serde(rename = "llamarpc")]
    LlamaRpc,
    #[serde(rename = "custom")]
    Custom,
}

impl ProviderType {
    /// Get the provider name as a string
    pub fn name(&self) -> &'static str {
        match self {
            ProviderType::Alchemy => "alchemy",
            ProviderType::Infura => "infura",
            ProviderType::QuickNode => "quicknode",
            ProviderType::Ankr => "ankr",
            ProviderType::LlamaRpc => "llamarpc",
            ProviderType::Custom => "custom",
        }
    }

    /// Get typical rate limits for this provider (requests per second)
    pub fn default_rate_limit(&self) -> u32 {
        match self {
            ProviderType::Alchemy => 300,      // Alchemy Growth plan
            ProviderType::Infura => 100,       // Infura standard
            ProviderType::QuickNode => 500,    // QuickNode premium
            ProviderType::Ankr => 200,         // Ankr premium
            ProviderType::LlamaRpc => 50,      // LlamaRPC free tier
            ProviderType::Custom => 100,       // Conservative default
        }
    }

    /// Check if this provider supports WebSocket subscriptions
    pub fn supports_websocket(&self) -> bool {
        match self {
            ProviderType::Alchemy => true,
            ProviderType::Infura => true,
            ProviderType::QuickNode => true,
            ProviderType::Ankr => true,
            ProviderType::LlamaRpc => false,
            ProviderType::Custom => false, // Assume no unless configured
        }
    }

    /// Get typical latency characteristics (milliseconds)
    pub fn typical_latency_ms(&self) -> u64 {
        match self {
            ProviderType::Alchemy => 100,
            ProviderType::Infura => 150,
            ProviderType::QuickNode => 80,
            ProviderType::Ankr => 120,
            ProviderType::LlamaRpc => 200,
            ProviderType::Custom => 150,
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for ProviderType {
    type Err = BetaDataplaneError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "alchemy" => Ok(ProviderType::Alchemy),
            "infura" => Ok(ProviderType::Infura),
            "quicknode" => Ok(ProviderType::QuickNode),
            "ankr" => Ok(ProviderType::Ankr),
            "llamarpc" => Ok(ProviderType::LlamaRpc),
            "custom" => Ok(ProviderType::Custom),
            _ => Err(BetaDataplaneError::InvalidProvider(s.to_string())),
        }
    }
}

/// Beta dataplane operational modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalMode {
    /// Development mode with verbose logging
    Development,
    
    /// Testing mode with validation
    Testing,
    
    /// Production mode optimized for performance
    Production,
    
    /// Dry-run mode for validation without publishing
    DryRun,
}

impl OperationalMode {
    /// Check if this mode allows data publishing
    pub fn allows_publishing(&self) -> bool {
        match self {
            OperationalMode::Development => true,
            OperationalMode::Testing => true,
            OperationalMode::Production => true,
            OperationalMode::DryRun => false,
        }
    }

    /// Get the appropriate log level for this mode
    pub fn log_level(&self) -> &'static str {
        match self {
            OperationalMode::Development => "debug",
            OperationalMode::Testing => "info",
            OperationalMode::Production => "warn",
            OperationalMode::DryRun => "info",
        }
    }

    /// Get the metrics collection interval for this mode
    pub fn metrics_interval_seconds(&self) -> u64 {
        match self {
            OperationalMode::Development => 5,
            OperationalMode::Testing => 10,
            OperationalMode::Production => 15,
            OperationalMode::DryRun => 30,
        }
    }
}

impl std::str::FromStr for OperationalMode {
    type Err = BetaDataplaneError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(OperationalMode::Development),
            "testing" | "test" => Ok(OperationalMode::Testing),
            "production" | "prod" => Ok(OperationalMode::Production),
            "dry-run" | "dryrun" => Ok(OperationalMode::DryRun),
            _ => Err(BetaDataplaneError::InvalidMode(s.to_string())),
        }
    }
}

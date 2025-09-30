//! # Qenus Dataplane
//!
//! The sensory nervous system of Qenus - responsible for continuously ingesting,
//! normalizing, and publishing live on-chain data from Ethereum L1 and selected
//! flash-loan enabled L2 rollups.

pub mod config;
pub mod error;
pub mod types;
pub mod observers;
pub mod extractors;
pub mod feeds;
pub mod utils;

// Re-export commonly used types
pub use config::DataplaneConfig;
pub use error::{DataplaneError, Result};
pub use types::*;

/// Current version of the dataplane
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Supported blockchain networks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Chain {
    #[serde(rename = "ethereum")]
    Ethereum,
    #[serde(rename = "arbitrum")]
    Arbitrum,
    #[serde(rename = "optimism")]
    Optimism,
    #[serde(rename = "base")]
    Base,
}

impl Chain {
    /// Get the chain ID for this network
    pub fn chain_id(&self) -> u64 {
        match self {
            Chain::Ethereum => 1,
            Chain::Arbitrum => 42161,
            Chain::Optimism => 10,
            Chain::Base => 8453,
        }
    }

    /// Get the human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Chain::Ethereum => "ethereum",
            Chain::Arbitrum => "arbitrum",
            Chain::Optimism => "optimism",
            Chain::Base => "base",
        }
    }

    /// Check if this chain supports flash loans
    pub fn supports_flash_loans(&self) -> bool {
        match self {
            Chain::Ethereum => true,
            Chain::Arbitrum => true,
            Chain::Optimism => true,
            Chain::Base => true,
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for Chain {
    type Err = DataplaneError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ethereum" | "eth" | "mainnet" => Ok(Chain::Ethereum),
            "arbitrum" | "arb" => Ok(Chain::Arbitrum),
            "optimism" | "op" => Ok(Chain::Optimism),
            "base" => Ok(Chain::Base),
            _ => Err(DataplaneError::InvalidChain(s.to_string())),
        }
    }
}

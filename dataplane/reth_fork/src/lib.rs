//! # Qenus Reth Fork
//!
//! A lean Ethereum L1 client optimized for real-time feature extraction.
//! Strips unnecessary components from Reth while maintaining block sync,
//! execution engine, and direct state access capabilities.

pub mod config;
pub mod node;
pub mod sync;
pub mod execution;
pub mod state;
pub mod extractors;
pub mod feeds;
pub mod error;

// Re-export commonly used types
pub use config::QenusRethConfig;
pub use error::{QenusRethError, Result};
pub use node::QenusRethNode;

/// Current version of the Qenus Reth fork
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Qenus-specific node type identifier
pub const NODE_TYPE: &str = "qenus-reth";

use qenus_dataplane::Chain;

/// Validate that this is configured for Ethereum mainnet
pub fn validate_ethereum_config() -> Result<()> {
    // Ensure we're only running on Ethereum L1
    let chain = Chain::Ethereum;
    if !chain.supports_flash_loans() {
        return Err(QenusRethError::InvalidChain(
            "Ethereum L1 must support flash loans".to_string()
        ));
    }
    Ok(())
}

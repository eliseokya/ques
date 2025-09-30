//! Chain observers for monitoring blockchain data
//!
//! This module contains observers for different blockchain networks that
//! monitor blocks, transactions, and events in real-time.

pub mod base;
pub mod traits;

// Re-export commonly used types
pub use traits::{ChainObserver, ObserverEvent, ObserverMetrics};

use crate::{Chain, Result};

/// Create an observer for the specified chain
pub async fn create_observer(chain: Chain) -> Result<Box<dyn ChainObserver>> {
    match chain {
        Chain::Ethereum => {
            // TODO: Implement Ethereum observer (via Reth fork)
            todo!("Ethereum observer not yet implemented")
        }
        Chain::Arbitrum => {
            let observer = base::L2Observer::new(chain).await?;
            Ok(Box::new(observer))
        }
        Chain::Optimism => {
            let observer = base::L2Observer::new(chain).await?;
            Ok(Box::new(observer))
        }
        Chain::Base => {
            let observer = base::L2Observer::new(chain).await?;
            Ok(Box::new(observer))
        }
    }
}

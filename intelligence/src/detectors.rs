//! Opportunity detectors
//!
//! Finds arbitrage candidates based on strategy configurations.

use crate::{Candidate, Result};

/// Opportunity detector trait
pub trait OpportunityDetector: Send + Sync {
    /// Detect opportunities
    fn detect(&self) -> Result<Vec<Candidate>>;
}


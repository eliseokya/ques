//! Trade intent builder
//!
//! Converts evaluation results into executable trade intents.

use crate::{TradeIntent, EvaluationResult, Result};

/// Intent builder
pub struct IntentBuilder;

impl IntentBuilder {
    /// Create a new intent builder
    pub fn new() -> Self {
        Self
    }

    /// Build a trade intent from an evaluation
    pub async fn build(&self, evaluation: &EvaluationResult) -> Result<TradeIntent> {
        // TODO: Implement intent building
        todo!("Implement intent building")
    }
}

impl Default for IntentBuilder {
    fn default() -> Self {
        Self::new()
    }
}


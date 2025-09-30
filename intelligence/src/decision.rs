//! Trade decision engine
//!
//! Applies risk policies and selects best opportunities.

use crate::{EvaluationResult, Result};

/// Decision engine
pub struct DecisionEngine;

impl DecisionEngine {
    /// Create a new decision engine
    pub fn new() -> Self {
        Self
    }

    /// Select best trade from evaluated opportunities
    pub async fn select_best(&self, evaluations: Vec<EvaluationResult>) -> Result<Option<EvaluationResult>> {
        // TODO: Implement decision logic
        Ok(evaluations.into_iter().next())
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}


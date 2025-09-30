//! Feedback and learning system
//!
//! Closes the loop by comparing predicted vs actual execution results.
//! Learns from errors and adapts simulation models over time.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use tracing::{info, warn, debug};
use serde::{Deserialize, Serialize};

use crate::{TradeIntent, Result, IntelligenceError};

/// Execution receipt from Orchestration layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    /// Intent ID that was executed
    pub intent_id: Uuid,
    
    /// Whether execution succeeded
    pub success: bool,
    
    /// Actual profit/loss in USD
    pub actual_pnl_usd: f64,
    
    /// Actual costs breakdown
    pub actual_costs: ActualCosts,
    
    /// Actual slippage experienced
    pub actual_slippage_bps: f64,
    
    /// Actual execution time (seconds)
    pub execution_time_secs: f64,
    
    /// Timestamp of completion
    pub completed_at: DateTime<Utc>,
    
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Actual costs from execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualCosts {
    pub gas_usd: f64,
    pub protocol_fees_usd: f64,
    pub bridge_fees_usd: f64,
    pub flashloan_fees_usd: f64,
    pub slippage_usd: f64,
    pub total_usd: f64,
}

/// Prediction error tracking
#[derive(Debug, Clone)]
pub struct PredictionError {
    pub pnl_error_pct: f64,
    pub gas_error_pct: f64,
    pub slippage_error_bps: f64,
    pub time_error_pct: f64,
}

/// Rolling error statistics
#[derive(Debug, Clone)]
pub struct ErrorStats {
    /// Number of samples
    pub sample_count: usize,
    
    /// Mean error
    pub mean: f64,
    
    /// Standard deviation
    pub std_dev: f64,
    
    /// Bias (systematic over/under prediction)
    pub bias: f64,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    /// Total intents tracked
    pub total_intents: usize,
    
    /// Successful executions
    pub successful_executions: usize,
    
    /// Failed executions
    pub failed_executions: usize,
    
    /// Hit rate (successful / total)
    pub hit_rate: f64,
    
    /// Average PnL error
    pub avg_pnl_error_pct: f64,
    
    /// Average gas error
    pub avg_gas_error_pct: f64,
    
    /// Average slippage error
    pub avg_slippage_error_bps: f64,
    
    /// Model accuracy score (0-1)
    pub accuracy_score: f64,
}

/// Feedback processor - learns from execution results
pub struct FeedbackProcessor {
    /// Store intents for comparison
    intents: Arc<RwLock<HashMap<Uuid, TradeIntent>>>,
    
    /// Store receipts
    receipts: Arc<RwLock<HashMap<Uuid, ExecutionReceipt>>>,
    
    /// Error tracking by strategy
    error_stats: Arc<RwLock<HashMap<String, HashMap<String, ErrorStats>>>>,
    
    /// Adjustment factors for models
    adjustments: Arc<RwLock<ModelAdjustments>>,
}

/// Model adjustment factors learned from feedback
#[derive(Debug, Clone)]
pub struct ModelAdjustments {
    /// Gas cost multiplier by chain
    pub gas_multipliers: HashMap<String, f64>,
    
    /// Slippage multiplier by protocol
    pub slippage_multipliers: HashMap<String, f64>,
    
    /// Success probability adjustments by strategy
    pub success_prob_adjustments: HashMap<String, f64>,
    
    /// PnL bias corrections
    pub pnl_bias: HashMap<String, f64>,
}

impl Default for ModelAdjustments {
    fn default() -> Self {
        Self {
            gas_multipliers: HashMap::new(),
            slippage_multipliers: HashMap::new(),
            success_prob_adjustments: HashMap::new(),
            pnl_bias: HashMap::new(),
        }
    }
}

impl FeedbackProcessor {
    /// Create a new feedback processor
    pub fn new() -> Self {
        Self {
            intents: Arc::new(RwLock::new(HashMap::new())),
            receipts: Arc::new(RwLock::new(HashMap::new())),
            error_stats: Arc::new(RwLock::new(HashMap::new())),
            adjustments: Arc::new(RwLock::new(ModelAdjustments::default())),
        }
    }

    /// Register an intent for tracking
    pub async fn register_intent(&self, intent: TradeIntent) {
        let intent_id = intent.intent_id;
        let mut intents = self.intents.write().await;
        intents.insert(intent_id, intent);
        
        debug!("Registered intent {} for feedback tracking", intent_id);
    }

    /// Process execution feedback from Orchestration
    pub async fn process_feedback(&self, receipt: ExecutionReceipt) -> Result<()> {
        let intent_id = receipt.intent_id;
        
        // Retrieve the original intent
        let intents = self.intents.read().await;
        let intent = intents.get(&intent_id).ok_or_else(|| {
            IntelligenceError::Internal(format!("Intent {} not found for feedback", intent_id))
        })?;
        
        // Calculate prediction errors
        let error = self.calculate_error(intent, &receipt);
        
        // Log the comparison
        self.log_comparison(intent, &receipt, &error);
        
        // Update error statistics
        self.update_error_stats(intent, error).await;
        
        // Update model adjustments
        self.update_adjustments(intent, &receipt).await;
        
        // Store receipt (capture success before move)
        let success = receipt.success;
        let mut receipts = self.receipts.write().await;
        receipts.insert(intent_id, receipt);
        
        info!(
            "Processed feedback for intent {} - Success: {}",
            intent_id,
            if success { "✅" } else { "❌" }
        );
        
        Ok(())
    }
    
    /// Calculate prediction errors
    fn calculate_error(&self, intent: &TradeIntent, receipt: &ExecutionReceipt) -> PredictionError {
        // PnL error
        let pnl_error_pct = if intent.expected_pnl_usd != 0.0 {
            ((receipt.actual_pnl_usd - intent.expected_pnl_usd) / intent.expected_pnl_usd) * 100.0
        } else {
            0.0
        };
        
        // Gas error
        let predicted_gas = intent.legs.iter()
            .filter_map(|_leg| {
                // Extract gas cost from metadata if available
                Some(50.0) // Placeholder - would extract from intent
            })
            .sum::<f64>();
        
        let gas_error_pct = if predicted_gas != 0.0 {
            ((receipt.actual_costs.gas_usd - predicted_gas) / predicted_gas) * 100.0
        } else {
            0.0
        };
        
        // Slippage error
        let predicted_slippage_bps = intent.legs.iter()
            .map(|_| 5.0) // Placeholder - would extract from intent
            .sum::<f64>();
        
        let slippage_error_bps = receipt.actual_slippage_bps - predicted_slippage_bps;
        
        // Time error
        let predicted_time_secs = intent.ttl_seconds as f64;
        let time_error_pct = if predicted_time_secs != 0.0 {
            ((receipt.execution_time_secs - predicted_time_secs) / predicted_time_secs) * 100.0
        } else {
            0.0
        };
        
        PredictionError {
            pnl_error_pct,
            gas_error_pct,
            slippage_error_bps,
            time_error_pct,
        }
    }
    
    /// Log comparison for analysis
    fn log_comparison(&self, intent: &TradeIntent, receipt: &ExecutionReceipt, error: &PredictionError) {
        info!("📊 Execution Analysis for {}", intent.intent_id);
        info!("  Strategy: {}", intent.strategy);
        info!("  Asset: {}", intent.asset);
        info!("  Success: {}", if receipt.success { "✅" } else { "❌" });
        
        info!("  PnL:");
        info!("    Predicted: ${:.2}", intent.expected_pnl_usd);
        info!("    Actual:    ${:.2}", receipt.actual_pnl_usd);
        info!("    Error:     {:.1}%", error.pnl_error_pct);
        
        info!("  Gas:");
        info!("    Actual:    ${:.2}", receipt.actual_costs.gas_usd);
        info!("    Error:     {:.1}%", error.gas_error_pct);
        
        info!("  Slippage:");
        info!("    Actual:    {:.2}bps", receipt.actual_slippage_bps);
        info!("    Error:     {:.2}bps", error.slippage_error_bps);
        
        if let Some(err_msg) = &receipt.error_message {
            warn!("  Error: {}", err_msg);
        }
    }
    
    /// Update error statistics
    async fn update_error_stats(&self, intent: &TradeIntent, error: PredictionError) {
        let mut stats = self.error_stats.write().await;
        let strategy_stats = stats.entry(intent.strategy.clone()).or_insert_with(HashMap::new);
        
        // Update PnL error stats
        self.update_stat(strategy_stats, "pnl", error.pnl_error_pct);
        self.update_stat(strategy_stats, "gas", error.gas_error_pct);
        self.update_stat(strategy_stats, "slippage", error.slippage_error_bps);
        self.update_stat(strategy_stats, "time", error.time_error_pct);
    }
    
    /// Update a specific error statistic
    fn update_stat(&self, stats: &mut HashMap<String, ErrorStats>, metric: &str, value: f64) {
        let stat = stats.entry(metric.to_string()).or_insert(ErrorStats {
            sample_count: 0,
            mean: 0.0,
            std_dev: 0.0,
            bias: 0.0,
        });
        
        // Update using exponential moving average
        let alpha = 0.1; // Learning rate
        let new_count = stat.sample_count + 1;
        let new_mean = stat.mean + alpha * (value - stat.mean);
        let new_bias = stat.bias + alpha * (value - stat.bias);
        
        // Update variance (simplified)
        let delta = value - stat.mean;
        let new_variance = (1.0 - alpha) * stat.std_dev.powi(2) + alpha * delta.powi(2);
        let new_std_dev = new_variance.sqrt();
        
        *stat = ErrorStats {
            sample_count: new_count,
            mean: new_mean,
            std_dev: new_std_dev,
            bias: new_bias,
        };
    }
    
    /// Update model adjustments based on learned errors
    async fn update_adjustments(&self, intent: &TradeIntent, receipt: &ExecutionReceipt) {
        let mut adjustments = self.adjustments.write().await;
        
        // Update gas multipliers
        for leg in &intent.legs {
            let chain_key = format!("{:?}", leg.domain);
            let current = adjustments.gas_multipliers.get(&chain_key).copied().unwrap_or(1.0);
            
            // If actual gas is consistently different, adjust multiplier
            let adjustment = if receipt.actual_costs.gas_usd > 0.0 {
                1.0 + (receipt.actual_costs.gas_usd / 50.0 - 1.0) * 0.1 // 10% learning rate
            } else {
                1.0
            };
            
            adjustments.gas_multipliers.insert(chain_key, current * 0.9 + adjustment * 0.1);
        }
        
        // Update success probability adjustments
        let strategy_key = intent.strategy.clone();
        let current_adj = adjustments.success_prob_adjustments.get(&strategy_key).copied().unwrap_or(0.0);
        
        let success_delta = if receipt.success {
            0.01 // Slight increase if successful
        } else {
            -0.02 // Larger decrease if failed
        };
        
        adjustments.success_prob_adjustments.insert(
            strategy_key,
            (current_adj + success_delta).clamp(-0.2, 0.2)
        );
    }
    
    /// Get model performance metrics
    pub async fn get_performance(&self) -> ModelPerformance {
        let receipts = self.receipts.read().await;
        let intents = self.intents.read().await;
        
        let total_intents = intents.len();
        let successful = receipts.values().filter(|r| r.success).count();
        let failed = receipts.values().filter(|r| !r.success).count();
        
        let hit_rate = if total_intents > 0 {
            successful as f64 / total_intents as f64
        } else {
            0.0
        };
        
        // Calculate average errors
        let (pnl_error, gas_error, slippage_error) = receipts.values().fold(
            (0.0, 0.0, 0.0),
            |(pnl_sum, gas_sum, slip_sum), receipt| {
                if let Some(intent) = intents.get(&receipt.intent_id) {
                    let error = self.calculate_error(intent, receipt);
                    (
                        pnl_sum + error.pnl_error_pct.abs(),
                        gas_sum + error.gas_error_pct.abs(),
                        slip_sum + error.slippage_error_bps.abs(),
                    )
                } else {
                    (pnl_sum, gas_sum, slip_sum)
                }
            }
        );
        
        let receipt_count = receipts.len() as f64;
        let avg_pnl_error = if receipt_count > 0.0 { pnl_error / receipt_count } else { 0.0 };
        let avg_gas_error = if receipt_count > 0.0 { gas_error / receipt_count } else { 0.0 };
        let avg_slippage_error = if receipt_count > 0.0 { slippage_error / receipt_count } else { 0.0 };
        
        // Calculate accuracy score (closer to 1.0 is better)
        let accuracy_score = hit_rate * (1.0 - (avg_pnl_error / 100.0).min(0.5));
        
        ModelPerformance {
            total_intents,
            successful_executions: successful,
            failed_executions: failed,
            hit_rate,
            avg_pnl_error_pct: avg_pnl_error,
            avg_gas_error_pct: avg_gas_error,
            avg_slippage_error_bps: avg_slippage_error,
            accuracy_score,
        }
    }
    
    /// Get current model adjustments
    pub async fn get_adjustments(&self) -> ModelAdjustments {
        self.adjustments.read().await.clone()
    }
    
    /// Clear old data (older than retention period)
    pub async fn cleanup(&self, retention_days: i64) {
        let cutoff = Utc::now() - Duration::days(retention_days);
        
        let mut intents = self.intents.write().await;
        let mut receipts = self.receipts.write().await;
        
        // Remove old intents
        intents.retain(|_, intent| intent.created_at > cutoff);
        
        // Remove old receipts
        receipts.retain(|_, receipt| receipt.completed_at > cutoff);
        
        info!("Cleaned up feedback data older than {} days", retention_days);
    }
}

impl Default for FeedbackProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TradeLeg, TradeAction, TradeMetadata, MarketSnapshot, RiskFactor};
    
    fn create_test_intent() -> TradeIntent {
        TradeIntent {
            intent_id: Uuid::new_v4(),
            strategy: "dex_arb".to_string(),
            asset: "USDC".to_string(),
            size_usd: 100_000.0,
            expected_pnl_usd: 600.0,
            net_bps: 12.0,
            success_prob: 0.85,
            legs: vec![],
            ttl_seconds: 30,
            created_at: Utc::now(),
            metadata: TradeMetadata {
                detected_at: Utc::now(),
                detector: "dex_arb".to_string(),
                market_snapshot: MarketSnapshot {
                    gas_prices: HashMap::new(),
                    sequencer_health: HashMap::new(),
                    volatility: 0.5,
                },
                risk_factors: vec![],
            },
        }
    }
    
    fn create_test_receipt(intent_id: Uuid, success: bool, actual_pnl: f64) -> ExecutionReceipt {
        ExecutionReceipt {
            intent_id,
            success,
            actual_pnl_usd: actual_pnl,
            actual_costs: ActualCosts {
                gas_usd: 55.0,
                protocol_fees_usd: 100.0,
                bridge_fees_usd: 0.0,
                flashloan_fees_usd: 0.0,
                slippage_usd: 45.0,
                total_usd: 200.0,
            },
            actual_slippage_bps: 8.0,
            execution_time_secs: 25.0,
            completed_at: Utc::now(),
            error_message: None,
        }
    }
    
    #[tokio::test]
    async fn test_feedback_processing() {
        let processor = FeedbackProcessor::new();
        let intent = create_test_intent();
        let intent_id = intent.intent_id;
        
        // Register intent
        processor.register_intent(intent).await;
        
        // Process feedback
        let receipt = create_test_receipt(intent_id, true, 580.0);
        processor.process_feedback(receipt).await.unwrap();
        
        // Check performance metrics
        let perf = processor.get_performance().await;
        assert_eq!(perf.total_intents, 1);
        assert_eq!(perf.successful_executions, 1);
        assert!(perf.hit_rate > 0.0);
    }
    
    #[tokio::test]
    async fn test_error_tracking() {
        let processor = FeedbackProcessor::new();
        let intent = create_test_intent();
        let intent_id = intent.intent_id;
        
        processor.register_intent(intent.clone()).await;
        
        // Actual PnL less than predicted
        let receipt = create_test_receipt(intent_id, true, 500.0);
        let error = processor.calculate_error(&intent, &receipt);
        
        // Should show negative error (actual < predicted)
        assert!(error.pnl_error_pct < 0.0);
    }
}

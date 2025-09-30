//! Predictive pre-fetching for RPC data
//!
//! Analyzes access patterns and pre-fetches data before it's requested
//! to minimize latency for critical operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

use crate::{Result};

/// Access pattern for a data key
#[derive(Debug, Clone)]
struct AccessPattern {
    /// Access timestamps
    access_times: Vec<Instant>,
    
    /// Average interval between accesses
    avg_interval: Option<Duration>,
    
    /// Confidence score (0.0 to 1.0)
    confidence: f64,
    
    /// Last prediction timestamp
    last_prediction: Option<Instant>,
}

impl AccessPattern {
    /// Create new access pattern
    fn new() -> Self {
        Self {
            access_times: Vec::new(),
            avg_interval: None,
            confidence: 0.0,
            last_prediction: None,
        }
    }

    /// Record an access
    fn record_access(&mut self) {
        let now = Instant::now();
        self.access_times.push(now);
        
        // Keep only recent history (last 10 accesses)
        if self.access_times.len() > 10 {
            self.access_times.remove(0);
        }
        
        // Calculate average interval
        self.calculate_interval();
        
        // Update confidence
        self.update_confidence();
    }

    /// Calculate average interval between accesses
    fn calculate_interval(&mut self) {
        if self.access_times.len() < 2 {
            self.avg_interval = None;
            return;
        }

        let mut intervals = Vec::new();
        for i in 1..self.access_times.len() {
            let interval = self.access_times[i].duration_since(self.access_times[i - 1]);
            intervals.push(interval);
        }

        let total: Duration = intervals.iter().sum();
        self.avg_interval = Some(total / intervals.len() as u32);
    }

    /// Update confidence score based on pattern regularity
    fn update_confidence(&mut self) {
        if self.access_times.len() < 3 {
            self.confidence = 0.0;
            return;
        }

        // Calculate variance in intervals
        if let Some(avg) = self.avg_interval {
            let avg_ms = avg.as_millis() as f64;
            let mut variance_sum = 0.0;
            
            for i in 1..self.access_times.len() {
                let interval = self.access_times[i].duration_since(self.access_times[i - 1]);
                let diff = interval.as_millis() as f64 - avg_ms;
                variance_sum += diff * diff;
            }
            
            let variance = variance_sum / (self.access_times.len() - 1) as f64;
            let std_dev = variance.sqrt();
            
            // Lower variance = higher confidence
            self.confidence = if avg_ms > 0.0 {
                1.0 / (1.0 + std_dev / avg_ms)
            } else {
                0.0
            };
        }
    }

    /// Predict next access time
    fn predict_next_access(&self) -> Option<Instant> {
        if let Some(avg_interval) = self.avg_interval {
            if self.confidence > 0.5 {
                // Predict based on last access + average interval
                self.access_times.last().map(|last| *last + avg_interval)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if we should pre-fetch now
    fn should_prefetch(&self, prefetch_window: Duration) -> bool {
        if let Some(predicted) = self.predict_next_access() {
            // Pre-fetch if predicted access is within the window
            let time_until = predicted.saturating_duration_since(Instant::now());
            time_until <= prefetch_window && self.confidence > 0.7
        } else {
            false
        }
    }
}

/// Data predictor for intelligent pre-fetching
pub struct DataPredictor {
    /// Access patterns for each key
    patterns: Arc<RwLock<HashMap<String, AccessPattern>>>,
    
    /// Pre-fetch window
    prefetch_window: Duration,
    
    /// Minimum confidence for pre-fetching
    min_confidence: f64,
    
    /// Prediction statistics
    stats: Arc<RwLock<PredictionStats>>,
}

/// Prediction statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PredictionStats {
    /// Total predictions made
    pub total_predictions: u64,
    
    /// Successful predictions (data was actually requested)
    pub successful_predictions: u64,
    
    /// Failed predictions (data was not requested)
    pub failed_predictions: u64,
    
    /// Accuracy rate (0.0 to 1.0)
    pub accuracy: f64,
    
    /// Average confidence score
    pub avg_confidence: f64,
}

impl PredictionStats {
    /// Create new prediction stats
    fn new() -> Self {
        Self {
            total_predictions: 0,
            successful_predictions: 0,
            failed_predictions: 0,
            accuracy: 0.0,
            avg_confidence: 0.0,
        }
    }

    /// Update accuracy
    fn update_accuracy(&mut self) {
        if self.total_predictions > 0 {
            self.accuracy = self.successful_predictions as f64 / self.total_predictions as f64;
        }
    }

    /// Record a prediction
    fn record_prediction(&mut self, confidence: f64) {
        self.total_predictions += 1;
        
        // Update average confidence
        let alpha = 0.1;
        self.avg_confidence = alpha * confidence + (1.0 - alpha) * self.avg_confidence;
    }

    /// Record prediction outcome
    fn record_outcome(&mut self, was_used: bool) {
        if was_used {
            self.successful_predictions += 1;
        } else {
            self.failed_predictions += 1;
        }
        self.update_accuracy();
    }
}

impl DataPredictor {
    /// Create a new data predictor
    pub fn new(prefetch_window: Duration, min_confidence: f64) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            prefetch_window,
            min_confidence,
            stats: Arc::new(RwLock::new(PredictionStats::new())),
        }
    }

    /// Record data access
    pub async fn record_access(&self, key: &str) {
        let mut patterns = self.patterns.write().await;
        let pattern = patterns.entry(key.to_string()).or_insert_with(AccessPattern::new);
        pattern.record_access();
        
        debug!(
            key = key,
            confidence = pattern.confidence,
            "Recorded access"
        );
    }

    /// Get keys that should be pre-fetched
    pub async fn get_prefetch_candidates(&self) -> Vec<String> {
        let patterns = self.patterns.read().await;
        let mut candidates = Vec::new();
        
        for (key, pattern) in patterns.iter() {
            if pattern.should_prefetch(self.prefetch_window) && pattern.confidence >= self.min_confidence {
                candidates.push(key.clone());
                debug!(
                    key = key,
                    confidence = pattern.confidence,
                    "Pre-fetch candidate identified"
                );
            }
        }
        
        candidates
    }

    /// Record a prediction
    pub async fn record_prediction(&self, key: &str, confidence: f64) {
        self.stats.write().await.record_prediction(confidence);
        
        // Mark prediction in pattern
        let mut patterns = self.patterns.write().await;
        if let Some(pattern) = patterns.get_mut(key) {
            pattern.last_prediction = Some(Instant::now());
        }
    }

    /// Record prediction outcome
    pub async fn record_outcome(&self, _key: &str, was_used: bool) {
        self.stats.write().await.record_outcome(was_used);
    }

    /// Get prediction statistics
    pub async fn get_stats(&self) -> PredictionStats {
        self.stats.read().await.clone()
    }

    /// Get pattern for a key
    pub async fn get_pattern(&self, key: &str) -> Option<AccessPattern> {
        let patterns = self.patterns.read().await;
        patterns.get(key).cloned()
    }

    /// Clear all patterns
    pub async fn clear_patterns(&self) {
        let mut patterns = self.patterns.write().await;
        let count = patterns.len();
        patterns.clear();
        info!(patterns_cleared = count, "Cleared access patterns");
    }
}

impl Clone for DataPredictor {
    fn clone(&self) -> Self {
        Self {
            patterns: Arc::clone(&self.patterns),
            prefetch_window: self.prefetch_window,
            min_confidence: self.min_confidence,
            stats: Arc::clone(&self.stats),
        }
    }
}

/// Prediction engine that coordinates caching and pre-fetching
pub struct PredictionEngine {
    /// Data predictor
    predictor: DataPredictor,
    
    /// Pre-fetch handler
    prefetch_tx: Option<mpsc::UnboundedSender<String>>,
}

impl PredictionEngine {
    /// Create a new prediction engine
    pub fn new(prefetch_window: Duration, min_confidence: f64) -> Self {
        Self {
            predictor: DataPredictor::new(prefetch_window, min_confidence),
            prefetch_tx: None,
        }
    }

    /// Start prediction engine with pre-fetch handler
    pub fn start<F, Fut>(&mut self, prefetch_handler: F)
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.prefetch_tx = Some(tx);
        
        let predictor = self.predictor.clone();
        let handler = Arc::new(prefetch_handler);
        
        // Start pre-fetch worker
        tokio::spawn(async move {
            while let Some(key) = rx.recv().await {
                handler(key).await;
            }
        });
        
        // Start prediction loop
        let predictor_clone = predictor.clone();
        let tx_clone = self.prefetch_tx.clone().unwrap();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                let candidates = predictor_clone.get_prefetch_candidates().await;
                
                for key in candidates {
                    if let Err(_) = tx_clone.send(key.clone()) {
                        break; // Receiver dropped
                    }
                }
            }
        });
        
        info!("Prediction engine started");
    }

    /// Record data access
    pub async fn record_access(&self, key: &str) {
        self.predictor.record_access(key).await;
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PredictionStats {
        self.predictor.get_stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_pattern() {
        let mut pattern = AccessPattern::new();
        
        // Record regular accesses
        for _ in 0..5 {
            pattern.record_access();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Should have some confidence now
        assert!(pattern.confidence > 0.0);
        assert!(pattern.avg_interval.is_some());
    }

    #[tokio::test]
    async fn test_data_predictor() {
        let predictor = DataPredictor::new(Duration::from_secs(1), 0.5);
        
        // Record accesses for a key
        for _ in 0..5 {
            predictor.record_access("test_key").await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Should have a pattern
        let pattern = predictor.get_pattern("test_key").await;
        assert!(pattern.is_some());
    }
}

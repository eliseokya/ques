//! Batch query optimization for RPC calls
//!
//! Groups multiple RPC requests into batches to minimize round-trips
//! and improve overall throughput.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::{Result, BetaDataplaneError};

/// Batch request
#[derive(Debug, Clone)]
pub struct BatchRequest<T> {
    /// Unique request ID
    pub id: String,
    
    /// Request data
    pub data: T,
    
    /// Timestamp when added to batch
    pub timestamp: Instant,
}

/// Batch result
#[derive(Debug, Clone)]
pub struct BatchResult<T> {
    /// Request ID
    pub id: String,
    
    /// Result data
    pub result: std::result::Result<T, String>,
    
    /// Processing time
    pub processing_time: Duration,
}

/// Batch processing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStrategy {
    /// Process when batch is full
    SizeBased,
    
    /// Process after timeout
    TimeBased,
    
    /// Process on both conditions (whichever comes first)
    Hybrid,
}

/// Batch processor for grouping requests
pub struct BatchProcessor<T: Clone + Send + Sync + 'static> {
    /// Pending requests
    pending: Arc<RwLock<Vec<BatchRequest<T>>>>,
    
    /// Batch size threshold
    batch_size: usize,
    
    /// Batch timeout
    batch_timeout: Duration,
    
    /// Processing strategy
    strategy: BatchStrategy,
    
    /// Batch statistics
    stats: Arc<RwLock<BatchStats>>,
}

/// Batch processing statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchStats {
    /// Total batches processed
    pub total_batches: u64,
    
    /// Total requests processed
    pub total_requests: u64,
    
    /// Average batch size
    pub avg_batch_size: f64,
    
    /// Average batch processing time (ms)
    pub avg_processing_time_ms: f64,
    
    /// Requests saved vs individual (estimate)
    pub requests_saved: u64,
}

impl BatchStats {
    /// Create new batch stats
    fn new() -> Self {
        Self {
            total_batches: 0,
            total_requests: 0,
            avg_batch_size: 0.0,
            avg_processing_time_ms: 0.0,
            requests_saved: 0,
        }
    }

    /// Record a batch processed
    fn record_batch(&mut self, batch_size: usize, processing_time: Duration) {
        self.total_batches += 1;
        self.total_requests += batch_size as u64;
        
        // Update average batch size (EMA)
        let alpha = 0.1;
        self.avg_batch_size = alpha * batch_size as f64 + (1.0 - alpha) * self.avg_batch_size;
        
        // Update average processing time
        self.avg_processing_time_ms = alpha * processing_time.as_millis() as f64 
            + (1.0 - alpha) * self.avg_processing_time_ms;
        
        // Estimate requests saved (assuming batching reduces overhead)
        if batch_size > 1 {
            self.requests_saved += (batch_size - 1) as u64;
        }
    }
}

impl<T: Clone + Send + Sync + 'static> BatchProcessor<T> {
    /// Create a new batch processor
    pub fn new(batch_size: usize, batch_timeout: Duration, strategy: BatchStrategy) -> Self {
        Self {
            pending: Arc::new(RwLock::new(Vec::new())),
            batch_size,
            batch_timeout,
            strategy,
            stats: Arc::new(RwLock::new(BatchStats::new())),
        }
    }

    /// Add a request to the batch
    pub async fn add_request(&self, id: String, data: T) {
        let request = BatchRequest {
            id,
            data,
            timestamp: Instant::now(),
        };
        
        self.pending.write().await.push(request);
        
        debug!(
            batch_size = self.pending.read().await.len(),
            threshold = self.batch_size,
            "Request added to batch"
        );
    }

    /// Get pending batch size
    pub async fn pending_size(&self) -> usize {
        self.pending.read().await.len()
    }

    /// Check if batch should be processed
    pub async fn should_process(&self) -> bool {
        let pending = self.pending.read().await;
        
        match self.strategy {
            BatchStrategy::SizeBased => {
                pending.len() >= self.batch_size
            }
            BatchStrategy::TimeBased => {
                if let Some(first) = pending.first() {
                    first.timestamp.elapsed() >= self.batch_timeout
                } else {
                    false
                }
            }
            BatchStrategy::Hybrid => {
                pending.len() >= self.batch_size
                    || (pending.first().map(|r| r.timestamp.elapsed() >= self.batch_timeout).unwrap_or(false))
            }
        }
    }

    /// Get current batch and clear pending
    pub async fn get_batch(&self) -> Vec<BatchRequest<T>> {
        let mut pending = self.pending.write().await;
        std::mem::take(&mut *pending)
    }

    /// Record batch processing
    pub async fn record_batch_processed(&self, batch_size: usize, processing_time: Duration) {
        self.stats.write().await.record_batch(batch_size, processing_time);
    }

    /// Get batch statistics
    pub async fn get_stats(&self) -> BatchStats {
        self.stats.read().await.clone()
    }

    /// Start auto-processing task
    pub fn start_auto_processor<F, Fut>(&self, processor: F)
    where
        F: Fn(Vec<BatchRequest<T>>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let batch_processor = self.clone();
        let processor = Arc::new(processor);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(10));
            
            loop {
                interval.tick().await;
                
                if batch_processor.should_process().await {
                    let batch = batch_processor.get_batch().await;
                    let batch_size = batch.len();
                    
                    if batch_size > 0 {
                        debug!(batch_size = batch_size, "Processing batch");
                        let start = Instant::now();
                        
                        processor(batch).await;
                        
                        let processing_time = start.elapsed();
                        batch_processor.record_batch_processed(batch_size, processing_time).await;
                        
                        debug!(
                            batch_size = batch_size,
                            processing_time_ms = processing_time.as_millis(),
                            "Batch processed"
                        );
                    }
                }
            }
        });
        
        info!("Started auto-processing task");
    }
}

impl<T: Clone + Send + Sync> Clone for BatchProcessor<T> {
    fn clone(&self) -> Self {
        Self {
            pending: Arc::clone(&self.pending),
            batch_size: self.batch_size,
            batch_timeout: self.batch_timeout,
            strategy: self.strategy,
            stats: Arc::clone(&self.stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_processor() {
        let processor = BatchProcessor::<String>::new(
            3, // Batch size
            Duration::from_secs(1),
            BatchStrategy::SizeBased,
        );

        // Add requests
        processor.add_request("req1".to_string(), "data1".to_string()).await;
        processor.add_request("req2".to_string(), "data2".to_string()).await;
        
        // Should not trigger yet
        assert!(!processor.should_process().await);
        
        processor.add_request("req3".to_string(), "data3".to_string()).await;
        
        // Should trigger now
        assert!(processor.should_process().await);
        
        // Get batch
        let batch = processor.get_batch().await;
        assert_eq!(batch.len(), 3);
    }
}

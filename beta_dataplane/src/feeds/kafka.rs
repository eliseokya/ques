//! Kafka feed implementation for real-time feature streaming
//!
//! Publishes features to Kafka topics for consumption by the Intelligence layer.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use qenus_dataplane::Feature;
use crate::feeds::traits::*;
use crate::{Result, BetaDataplaneError};

/// Kafka producer for feature publishing
pub struct KafkaFeed {
    /// Feed name
    name: &'static str,

    /// Kafka broker addresses
    brokers: Vec<String>,

    /// Topic name
    topic: String,

    /// Feed configuration
    config: BetaFeedConfig,

    /// Feed metrics
    metrics: Arc<RwLock<BetaFeedMetrics>>,

    /// Feed health
    health: Arc<RwLock<BetaFeedHealth>>,

    /// Running state
    is_running: Arc<RwLock<bool>>,

    /// Internal queue for batching
    queue: Arc<RwLock<Vec<Feature>>>,

    /// Shutdown sender
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Producer handle (placeholder for actual Kafka producer)
    producer_handle: Option<tokio::task::JoinHandle<()>>,
}

impl KafkaFeed {
    /// Create a new Kafka feed
    pub fn new(
        brokers: Vec<String>,
        topic: String,
        config: BetaFeedConfig,
    ) -> Self {
        let metrics = BetaFeedMetrics::new("kafka".to_string());
        let health = BetaFeedHealth::new("kafka".to_string());

        Self {
            name: "kafka",
            brokers,
            topic,
            config,
            metrics: Arc::new(RwLock::new(metrics)),
            health: Arc::new(RwLock::new(health)),
            is_running: Arc::new(RwLock::new(false)),
            queue: Arc::new(RwLock::new(Vec::new())),
            shutdown_tx: None,
            producer_handle: None,
        }
    }

    /// Create a new Kafka feed from configuration
    pub fn from_config(custom_config: &HashMap<String, String>) -> Result<Self> {
        let brokers = custom_config
            .get("brokers")
            .ok_or_else(|| {
                BetaDataplaneError::InvalidProvider("Kafka brokers not configured".to_string())
            })?
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let topic = custom_config
            .get("topic")
            .ok_or_else(|| {
                BetaDataplaneError::InvalidProvider("Kafka topic not configured".to_string())
            })?
            .clone();

        let mut config = BetaFeedConfig::default();
        config.custom_config = custom_config.clone();

        Ok(Self::new(brokers, topic, config))
    }

    /// Publish a batch of features to Kafka
    async fn publish_batch(&self, features: Vec<Feature>) -> Result<()> {
        if features.is_empty() {
            return Ok(());
        }

        let start = Instant::now();
        let feature_count = features.len();

        // Serialize features
        let serialized: Vec<Vec<u8>> = features
            .iter()
            .map(|f| self.serialize_feature(f))
            .collect::<Result<Vec<_>>>()?;

        let total_bytes: usize = serialized.iter().map(|b| b.len()).sum();

        // Simulate Kafka publish (in production, use rdkafka or similar)
        // TODO: Replace with actual Kafka producer
        debug!(
            "Publishing batch of {} features ({} bytes) to Kafka topic '{}'",
            feature_count, total_bytes, self.topic
        );

        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(5)).await;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.update_publish(feature_count as u64, total_bytes as u64, latency_ms);
        metrics.update_batch(feature_count);

        info!(
            "Published batch of {} features to Kafka (latency: {:.2}ms)",
            feature_count, latency_ms
        );

        Ok(())
    }

    /// Serialize a feature for Kafka
    fn serialize_feature(&self, feature: &Feature) -> Result<Vec<u8>> {
        serde_json::to_vec(feature).map_err(|e| e.into())
    }

    /// Start the background batch processor
    async fn start_batch_processor(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let queue = Arc::clone(&self.queue);
        let metrics = Arc::clone(&self.metrics);
        let health = Arc::clone(&self.health);
        let is_running = Arc::clone(&self.is_running);
        let config = self.config.clone();
        let brokers = self.brokers.clone();
        let topic = self.topic.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(config.batch_timeout_ms));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Process batch
                        let mut queue_lock = queue.write().await;
                        if !queue_lock.is_empty() {
                            let batch = queue_lock.drain(..).collect::<Vec<_>>();
                            drop(queue_lock);

                            // Create temporary feed for batch processing
                            let temp_feed = KafkaFeed {
                                name: "kafka",
                                brokers: brokers.clone(),
                                topic: topic.clone(),
                                config: config.clone(),
                                metrics: Arc::clone(&metrics),
                                health: Arc::clone(&health),
                                is_running: Arc::clone(&is_running),
                                queue: Arc::clone(&queue),
                                shutdown_tx: None,
                                producer_handle: None,
                            };

                            if let Err(e) = temp_feed.publish_batch(batch).await {
                                error!("Failed to publish batch: {}", e);

                                let mut health_lock = health.write().await;
                                health_lock.update_status(
                                    FeedStatus::Degraded,
                                    Some(format!("Batch publish failed: {}", e)),
                                );
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down Kafka batch processor");
                        break;
                    }
                }
            }
        });

        self.producer_handle = Some(handle);
        Ok(())
    }
}

#[async_trait]
impl BetaDataFeed for KafkaFeed {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn start(&mut self) -> Result<()> {
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(BetaDataplaneError::internal("Kafka feed is already running"));
            }
        }

        info!("Starting Kafka feed (brokers: {:?}, topic: {})", self.brokers, self.topic);

        // Update health
        {
            let mut health = self.health.write().await;
            health.update_status(FeedStatus::Connecting, None);
            health.update_connection(ConnectionStatus::Connecting);
        }

        // Start batch processor
        self.start_batch_processor().await?;

        // Update state
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        let mut health = self.health.write().await;
        health.update_status(FeedStatus::Healthy, None);
        health.update_connection(ConnectionStatus::Connected);

        info!("Kafka feed started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        info!("Stopping Kafka feed");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for producer handle to finish
        if let Some(handle) = self.producer_handle.take() {
            let _ = handle.await;
        }

        // Flush remaining queue
        let queue = self.queue.read().await;
        if !queue.is_empty() {
            warn!("Kafka feed stopped with {} features in queue", queue.len());
        }
        drop(queue);

        *is_running = false;

        let mut health = self.health.write().await;
        health.update_status(FeedStatus::Disabled, None);
        health.update_connection(ConnectionStatus::Disconnected);

        info!("Kafka feed stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // Note: This is synchronous, so we can't lock properly
        // In production, consider using AtomicBool
        false // Placeholder
    }

    async fn publish_feature(&self, feature: Feature) -> Result<()> {
        if !*self.is_running.read().await {
            return Err(BetaDataplaneError::internal("Kafka feed is not running"));
        }

        let mut queue = self.queue.write().await;
        queue.push(feature);

        // Update queue metrics
        let queue_size = queue.len();
        drop(queue);

        let mut metrics = self.metrics.write().await;
        metrics.update_queue(queue_size, self.config.max_queue_size);

        // Check if we should flush immediately
        if queue_size >= self.config.batch_size {
            drop(metrics);
            // Trigger immediate batch processing
            let mut queue = self.queue.write().await;
            let batch = queue.drain(..).collect::<Vec<_>>();
            drop(queue);

            self.publish_batch(batch).await?;
        }

        Ok(())
    }

    async fn publish_features(&self, features: Vec<Feature>) -> Result<()> {
        if !*self.is_running.read().await {
            return Err(BetaDataplaneError::internal("Kafka feed is not running"));
        }

        for feature in features {
            self.publish_feature(feature).await?;
        }

        Ok(())
    }

    fn config(&self) -> BetaFeedConfig {
        self.config.clone()
    }

    async fn update_config(&mut self, config: BetaFeedConfig) -> Result<()> {
        info!("Updating Kafka feed configuration");
        self.config = config;
        Ok(())
    }

    fn metrics(&self) -> BetaFeedMetrics {
        // Note: This is synchronous, so we can't lock properly
        // In production, return Arc or use different approach
        BetaFeedMetrics::new("kafka".to_string())
    }

    fn health(&self) -> BetaFeedHealth {
        // Note: This is synchronous, so we can't lock properly
        // In production, return Arc or use different approach
        BetaFeedHealth::new("kafka".to_string())
    }

    async fn flush(&self) -> Result<()> {
        info!("Flushing Kafka feed");

        let mut queue = self.queue.write().await;
        if !queue.is_empty() {
            let batch = queue.drain(..).collect::<Vec<_>>();
            drop(queue);

            self.publish_batch(batch).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kafka_feed_creation() {
        let brokers = vec!["localhost:9092".to_string()];
        let topic = "qenus-features".to_string();
        let config = BetaFeedConfig::default();

        let feed = KafkaFeed::new(brokers, topic, config);
        assert_eq!(feed.name(), "kafka");
    }

    #[tokio::test]
    async fn test_kafka_feed_lifecycle() {
        let brokers = vec!["localhost:9092".to_string()];
        let topic = "test-topic".to_string();
        let config = BetaFeedConfig::default();

        let mut feed = KafkaFeed::new(brokers, topic, config);

        // Start feed
        assert!(feed.start().await.is_ok());

        // Stop feed
        assert!(feed.stop().await.is_ok());
    }
}

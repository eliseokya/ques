//! Feed manager for coordinating multiple data feeds
//!
//! Manages Kafka, gRPC, and Parquet feeds with unified control.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use qenus_dataplane::Feature;
use crate::feeds::*;
use crate::{Result, BetaDataplaneError};

/// Manager for all data feeds
pub struct FeedManager {
    /// Kafka feed
    kafka: Option<Arc<RwLock<KafkaFeed>>>,

    /// gRPC feed
    grpc: Option<Arc<RwLock<GrpcFeed>>>,

    /// Parquet feed
    parquet: Option<Arc<RwLock<ParquetFeed>>>,

    /// Feed configurations
    configs: HashMap<String, BetaFeedConfig>,

    /// Running state
    is_running: Arc<RwLock<bool>>,
}

impl FeedManager {
    /// Create a new feed manager
    pub fn new() -> Self {
        Self {
            kafka: None,
            grpc: None,
            parquet: None,
            configs: HashMap::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Configure Kafka feed
    pub fn with_kafka(
        mut self,
        brokers: Vec<String>,
        topic: String,
        config: BetaFeedConfig,
    ) -> Self {
        let kafka = KafkaFeed::new(brokers, topic, config.clone());
        self.kafka = Some(Arc::new(RwLock::new(kafka)));
        self.configs.insert("kafka".to_string(), config);
        self
    }

    /// Configure gRPC feed
    pub fn with_grpc(
        mut self,
        address: String,
        port: u16,
        config: BetaFeedConfig,
    ) -> Self {
        let grpc = GrpcFeed::new(address, port, config.clone());
        self.grpc = Some(Arc::new(RwLock::new(grpc)));
        self.configs.insert("grpc".to_string(), config);
        self
    }

    /// Configure Parquet feed
    pub fn with_parquet(
        mut self,
        output_dir: std::path::PathBuf,
        file_prefix: String,
        config: BetaFeedConfig,
    ) -> Self {
        let parquet = ParquetFeed::new(output_dir, file_prefix, config.clone());
        self.parquet = Some(Arc::new(RwLock::new(parquet)));
        self.configs.insert("parquet".to_string(), config);
        self
    }

    /// Start all configured feeds
    pub async fn start_all(&mut self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(BetaDataplaneError::internal("Feed manager is already running"));
        }

        info!("Starting all data feeds");

        // Start Kafka
        if let Some(kafka) = &self.kafka {
            let mut kafka_lock = kafka.write().await;
            if let Err(e) = kafka_lock.start().await {
                error!("Failed to start Kafka feed: {}", e);
            } else {
                info!("Kafka feed started");
            }
        }

        // Start gRPC
        if let Some(grpc) = &self.grpc {
            let mut grpc_lock = grpc.write().await;
            if let Err(e) = grpc_lock.start().await {
                error!("Failed to start gRPC feed: {}", e);
            } else {
                info!("gRPC feed started");
            }
        }

        // Start Parquet
        if let Some(parquet) = &self.parquet {
            let mut parquet_lock = parquet.write().await;
            if let Err(e) = parquet_lock.start().await {
                error!("Failed to start Parquet feed: {}", e);
            } else {
                info!("Parquet feed started");
            }
        }

        *is_running = true;
        info!("All data feeds started");
        Ok(())
    }

    /// Stop all feeds
    pub async fn stop_all(&mut self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        info!("Stopping all data feeds");

        // Stop Kafka
        if let Some(kafka) = &self.kafka {
            let mut kafka_lock = kafka.write().await;
            if let Err(e) = kafka_lock.stop().await {
                error!("Failed to stop Kafka feed: {}", e);
            }
        }

        // Stop gRPC
        if let Some(grpc) = &self.grpc {
            let mut grpc_lock = grpc.write().await;
            if let Err(e) = grpc_lock.stop().await {
                error!("Failed to stop gRPC feed: {}", e);
            }
        }

        // Stop Parquet
        if let Some(parquet) = &self.parquet {
            let mut parquet_lock = parquet.write().await;
            if let Err(e) = parquet_lock.stop().await {
                error!("Failed to stop Parquet feed: {}", e);
            }
        }

        *is_running = false;
        info!("All data feeds stopped");
        Ok(())
    }

    /// Publish a feature to all feeds
    pub async fn publish(&self, feature: Feature) -> Result<()> {
        // Publish to Kafka
        if let Some(kafka) = &self.kafka {
            let kafka_lock = kafka.read().await;
            if let Err(e) = kafka_lock.publish_feature(feature.clone()).await {
                warn!("Failed to publish to Kafka: {}", e);
            }
        }

        // Publish to gRPC
        if let Some(grpc) = &self.grpc {
            let grpc_lock = grpc.read().await;
            if let Err(e) = grpc_lock.publish_feature(feature.clone()).await {
                warn!("Failed to publish to gRPC: {}", e);
            }
        }

        // Publish to Parquet
        if let Some(parquet) = &self.parquet {
            let parquet_lock = parquet.read().await;
            if let Err(e) = parquet_lock.publish_feature(feature.clone()).await {
                warn!("Failed to publish to Parquet: {}", e);
            }
        }

        Ok(())
    }

    /// Publish multiple features to all feeds
    pub async fn publish_batch(&self, features: Vec<Feature>) -> Result<()> {
        for feature in features {
            self.publish(feature).await?;
        }
        Ok(())
    }

    /// Flush all feeds
    pub async fn flush_all(&self) -> Result<()> {
        info!("Flushing all data feeds");

        if let Some(kafka) = &self.kafka {
            let kafka_lock = kafka.read().await;
            if let Err(e) = kafka_lock.flush().await {
                error!("Failed to flush Kafka: {}", e);
            }
        }

        if let Some(grpc) = &self.grpc {
            let grpc_lock = grpc.read().await;
            if let Err(e) = grpc_lock.flush().await {
                error!("Failed to flush gRPC: {}", e);
            }
        }

        if let Some(parquet) = &self.parquet {
            let parquet_lock = parquet.read().await;
            if let Err(e) = parquet_lock.flush().await {
                error!("Failed to flush Parquet: {}", e);
            }
        }

        Ok(())
    }

    /// Get aggregated metrics from all feeds
    pub async fn get_metrics(&self) -> HashMap<String, BetaFeedMetrics> {
        let mut metrics = HashMap::new();

        if let Some(kafka) = &self.kafka {
            let kafka_lock = kafka.read().await;
            metrics.insert("kafka".to_string(), kafka_lock.metrics());
        }

        if let Some(grpc) = &self.grpc {
            let grpc_lock = grpc.read().await;
            metrics.insert("grpc".to_string(), grpc_lock.metrics());
        }

        if let Some(parquet) = &self.parquet {
            let parquet_lock = parquet.read().await;
            metrics.insert("parquet".to_string(), parquet_lock.metrics());
        }

        metrics
    }

    /// Get aggregated health from all feeds
    pub async fn get_health(&self) -> HashMap<String, BetaFeedHealth> {
        let mut health = HashMap::new();

        if let Some(kafka) = &self.kafka {
            let kafka_lock = kafka.read().await;
            health.insert("kafka".to_string(), kafka_lock.health());
        }

        if let Some(grpc) = &self.grpc {
            let grpc_lock = grpc.read().await;
            health.insert("grpc".to_string(), grpc_lock.health());
        }

        if let Some(parquet) = &self.parquet {
            let parquet_lock = parquet.read().await;
            health.insert("parquet".to_string(), parquet_lock.health());
        }

        health
    }

    /// Check if all feeds are healthy
    pub async fn is_healthy(&self) -> bool {
        let health = self.get_health().await;
        health.values().all(|h| h.is_healthy() || h.is_disabled())
    }

    /// Get feed by name
    pub fn get_kafka(&self) -> Option<Arc<RwLock<KafkaFeed>>> {
        self.kafka.clone()
    }

    pub fn get_grpc(&self) -> Option<Arc<RwLock<GrpcFeed>>> {
        self.grpc.clone()
    }

    pub fn get_parquet(&self) -> Option<Arc<RwLock<ParquetFeed>>> {
        self.parquet.clone()
    }
}

impl Default for FeedManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feed_manager_creation() {
        let manager = FeedManager::new();
        assert!(manager.kafka.is_none());
        assert!(manager.grpc.is_none());
        assert!(manager.parquet.is_none());
    }

    #[tokio::test]
    async fn test_feed_manager_with_feeds() {
        let manager = FeedManager::new()
            .with_kafka(
                vec!["localhost:9092".to_string()],
                "test-topic".to_string(),
                BetaFeedConfig::default(),
            )
            .with_grpc(
                "127.0.0.1".to_string(),
                50051,
                BetaFeedConfig::default(),
            );

        assert!(manager.kafka.is_some());
        assert!(manager.grpc.is_some());
        assert!(manager.parquet.is_none());
    }
}

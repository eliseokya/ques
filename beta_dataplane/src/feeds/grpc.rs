//! gRPC feed implementation for real-time feature serving
//!
//! Provides a gRPC API for the Intelligence layer to consume features.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use qenus_dataplane::Feature;
use crate::feeds::traits::*;
use crate::{Result, BetaDataplaneError};

/// gRPC server for feature serving
pub struct GrpcFeed {
    /// Feed name
    name: &'static str,

    /// Server address
    address: String,

    /// Port
    port: u16,

    /// Feed configuration
    config: BetaFeedConfig,

    /// Feed metrics
    metrics: Arc<RwLock<BetaFeedMetrics>>,

    /// Feed health
    health: Arc<RwLock<BetaFeedHealth>>,

    /// Running state
    is_running: Arc<RwLock<bool>>,

    /// Feature cache for serving
    feature_cache: Arc<RwLock<Vec<Feature>>>,

    /// Subscribers (stream channels)
    subscribers: Arc<RwLock<Vec<mpsc::Sender<Feature>>>>,

    /// Shutdown sender
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Server handle
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl GrpcFeed {
    /// Create a new gRPC feed
    pub fn new(
        address: String,
        port: u16,
        config: BetaFeedConfig,
    ) -> Self {
        let metrics = BetaFeedMetrics::new("grpc".to_string());
        let health = BetaFeedHealth::new("grpc".to_string());

        Self {
            name: "grpc",
            address,
            port,
            config,
            metrics: Arc::new(RwLock::new(metrics)),
            health: Arc::new(RwLock::new(health)),
            is_running: Arc::new(RwLock::new(false)),
            feature_cache: Arc::new(RwLock::new(Vec::new())),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            shutdown_tx: None,
            server_handle: None,
        }
    }

    /// Create a new gRPC feed from configuration
    pub fn from_config(custom_config: &HashMap<String, String>) -> Result<Self> {
        let address = custom_config
            .get("address")
            .ok_or_else(|| {
                BetaDataplaneError::InvalidProvider("gRPC address not configured".to_string())
            })?
            .clone();

        let port = custom_config
            .get("port")
            .ok_or_else(|| {
                BetaDataplaneError::InvalidProvider("gRPC port not configured".to_string())
            })?
            .parse::<u16>()
            .map_err(|e| {
                BetaDataplaneError::InvalidProvider(format!("Invalid gRPC port: {}", e))
            })?;

        let mut config = BetaFeedConfig::default();
        config.custom_config = custom_config.clone();

        Ok(Self::new(address, port, config))
    }

    /// Get the server endpoint
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Broadcast feature to all subscribers
    async fn broadcast_feature(&self, feature: Feature) -> Result<()> {
        let mut subscribers = self.subscribers.write().await;
        let mut failed_indices = Vec::new();

        for (idx, subscriber) in subscribers.iter().enumerate() {
            if let Err(e) = subscriber.try_send(feature.clone()) {
                warn!("Failed to send feature to subscriber {}: {}", idx, e);
                failed_indices.push(idx);
            }
        }

        // Remove failed subscribers
        for idx in failed_indices.iter().rev() {
            subscribers.swap_remove(*idx);
        }

        Ok(())
    }

    /// Start the gRPC server
    async fn start_server(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let endpoint = self.endpoint();
        let metrics = Arc::clone(&self.metrics);
        let health = Arc::clone(&self.health);
        let feature_cache = Arc::clone(&self.feature_cache);
        let subscribers = Arc::clone(&self.subscribers);

        let handle = tokio::spawn(async move {
            info!("Starting gRPC server on {}", endpoint);

            // Simulate gRPC server (in production, use tonic or similar)
            // TODO: Replace with actual gRPC server implementation
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        // Simulate server health check
                        let mut health_lock = health.write().await;
                        health_lock.update_connection(ConnectionStatus::Connected);
                        drop(health_lock);

                        // Update metrics
                        let cache_size = feature_cache.read().await.len();
                        let subscriber_count = subscribers.read().await.len();

                        let mut metrics_lock = metrics.write().await;
                        metrics_lock.add_custom_metric("cache_size".to_string(), cache_size as f64);
                        metrics_lock.add_custom_metric("subscribers".to_string(), subscriber_count as f64);
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down gRPC server");
                        break;
                    }
                }
            }
        });

        self.server_handle = Some(handle);
        Ok(())
    }

    /// Add a new subscriber
    pub async fn subscribe(&self) -> Result<mpsc::Receiver<Feature>> {
        if !*self.is_running.read().await {
            return Err(BetaDataplaneError::internal("gRPC feed is not running"));
        }

        let (tx, rx) = mpsc::channel(1000);
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(tx);

        info!("New subscriber added (total: {})", subscribers.len());
        Ok(rx)
    }

    /// Get recent features from cache
    pub async fn get_recent_features(&self, limit: usize) -> Vec<Feature> {
        let cache = self.feature_cache.read().await;
        let start = if cache.len() > limit {
            cache.len() - limit
        } else {
            0
        };
        cache[start..].to_vec()
    }
}

#[async_trait]
impl BetaDataFeed for GrpcFeed {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn start(&mut self) -> Result<()> {
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(BetaDataplaneError::internal("gRPC feed is already running"));
            }
        }

        info!("Starting gRPC feed on {}", self.endpoint());

        // Update health
        {
            let mut health = self.health.write().await;
            health.update_status(FeedStatus::Connecting, None);
            health.update_connection(ConnectionStatus::Connecting);
        }

        // Start server
        self.start_server().await?;

        // Update state
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        let mut health = self.health.write().await;
        health.update_status(FeedStatus::Healthy, None);
        health.update_connection(ConnectionStatus::Connected);

        info!("gRPC feed started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        info!("Stopping gRPC feed");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for server handle to finish
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }

        // Clear subscribers
        let mut subscribers = self.subscribers.write().await;
        subscribers.clear();

        *is_running = false;

        let mut health = self.health.write().await;
        health.update_status(FeedStatus::Disabled, None);
        health.update_connection(ConnectionStatus::Disconnected);

        info!("gRPC feed stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        // Note: This is synchronous, so we can't lock properly
        false // Placeholder
    }

    async fn publish_feature(&self, feature: Feature) -> Result<()> {
        if !*self.is_running.read().await {
            return Err(BetaDataplaneError::internal("gRPC feed is not running"));
        }

        let start = Instant::now();

        // Add to cache
        let mut cache = self.feature_cache.write().await;
        cache.push(feature.clone());

        // Limit cache size
        let max_cache_size = 10000;
        let cache_len = cache.len();
        if cache_len > max_cache_size {
            cache.drain(0..cache_len - max_cache_size);
        }
        drop(cache);

        // Broadcast to subscribers
        self.broadcast_feature(feature).await?;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.update_publish(1, 0, latency_ms);

        Ok(())
    }

    async fn publish_features(&self, features: Vec<Feature>) -> Result<()> {
        if !*self.is_running.read().await {
            return Err(BetaDataplaneError::internal("gRPC feed is not running"));
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
        info!("Updating gRPC feed configuration");
        self.config = config;
        Ok(())
    }

    fn metrics(&self) -> BetaFeedMetrics {
        BetaFeedMetrics::new("grpc".to_string())
    }

    fn health(&self) -> BetaFeedHealth {
        BetaFeedHealth::new("grpc".to_string())
    }

    async fn flush(&self) -> Result<()> {
        info!("Flushing gRPC feed (no-op for streaming)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grpc_feed_creation() {
        let address = "127.0.0.1".to_string();
        let port = 50051;
        let config = BetaFeedConfig::default();

        let feed = GrpcFeed::new(address, port, config);
        assert_eq!(feed.name(), "grpc");
        assert_eq!(feed.endpoint(), "127.0.0.1:50051");
    }

    #[tokio::test]
    async fn test_grpc_feed_lifecycle() {
        let address = "127.0.0.1".to_string();
        let port = 50052;
        let config = BetaFeedConfig::default();

        let mut feed = GrpcFeed::new(address, port, config);

        // Start feed
        assert!(feed.start().await.is_ok());

        // Stop feed
        assert!(feed.stop().await.is_ok());
    }
}

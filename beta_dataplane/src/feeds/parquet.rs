//! Parquet feed implementation for feature archival
//!
//! Archives features to Parquet files for backtesting and analysis.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use qenus_dataplane::Feature;
use crate::feeds::traits::*;
use crate::optimization::compression::{DataCompressor, CompressionAlgorithm as OptCompressionAlgorithm, CompressionLevel};
use crate::{Result, BetaDataplaneError};

/// Parquet writer for feature archival
pub struct ParquetFeed {
    /// Feed name
    name: &'static str,

    /// Output directory
    output_dir: PathBuf,

    /// File prefix
    file_prefix: String,

    /// Feed configuration
    config: BetaFeedConfig,

    /// Feed metrics
    metrics: Arc<RwLock<BetaFeedMetrics>>,

    /// Feed health
    health: Arc<RwLock<BetaFeedHealth>>,

    /// Running state
    is_running: Arc<RwLock<bool>>,

    /// Internal buffer for batching
    buffer: Arc<RwLock<Vec<Feature>>>,

    /// Current file path
    current_file: Arc<RwLock<Option<PathBuf>>>,

    /// Compression
    compressor: Arc<DataCompressor>,

    /// Shutdown sender
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Writer handle
    writer_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ParquetFeed {
    /// Create a new Parquet feed
    pub fn new(
        output_dir: PathBuf,
        file_prefix: String,
        config: BetaFeedConfig,
    ) -> Self {
        let metrics = BetaFeedMetrics::new("parquet".to_string());
        let health = BetaFeedHealth::new("parquet".to_string());

        // Create compressor based on config
        let compression_algo = match config.compression.algorithm {
            CompressionAlgorithm::Gzip => OptCompressionAlgorithm::Gzip,
            CompressionAlgorithm::Snappy => OptCompressionAlgorithm::Snappy,
            CompressionAlgorithm::Lz4 => OptCompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd => OptCompressionAlgorithm::Zstd,
            CompressionAlgorithm::None => OptCompressionAlgorithm::None,
        };

        let compression_level = CompressionLevel::new(config.compression.level);

        let compressor = DataCompressor::new(compression_algo, compression_level);

        Self {
            name: "parquet",
            output_dir,
            file_prefix,
            config,
            metrics: Arc::new(RwLock::new(metrics)),
            health: Arc::new(RwLock::new(health)),
            is_running: Arc::new(RwLock::new(false)),
            buffer: Arc::new(RwLock::new(Vec::new())),
            current_file: Arc::new(RwLock::new(None)),
            compressor: Arc::new(compressor),
            shutdown_tx: None,
            writer_handle: None,
        }
    }

    /// Create a new Parquet feed from configuration
    pub fn from_config(custom_config: &HashMap<String, String>) -> Result<Self> {
        let output_dir = custom_config
            .get("output_dir")
            .ok_or_else(|| {
                BetaDataplaneError::InvalidProvider("Parquet output directory not configured".to_string())
            })?
            .into();

        let file_prefix = custom_config
            .get("file_prefix")
            .unwrap_or(&"qenus_features".to_string())
            .clone();

        let mut config = BetaFeedConfig::default();
        config.custom_config = custom_config.clone();

        Ok(Self::new(output_dir, file_prefix, config))
    }

    /// Generate a new file path
    fn generate_file_path(&self) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.parquet", self.file_prefix, timestamp);
        self.output_dir.join(filename)
    }

    /// Write features to Parquet file
    async fn write_parquet(&self, features: Vec<Feature>, file_path: &Path) -> Result<()> {
        let start = Instant::now();

        // Serialize features to JSON (in production, use actual Parquet writer)
        let json_data: Vec<u8> = serde_json::to_vec(&features).map_err(|e| BetaDataplaneError::from(e))?;

        let uncompressed_size = json_data.len();

        // Compress data
        let compressed_data = if self.config.compression.enabled {
            self.compressor.compress(&json_data)?
        } else {
            json_data
        };

        let compressed_size = compressed_data.len();
        let compression_ratio = if self.config.compression.enabled {
            uncompressed_size as f64 / compressed_size as f64
        } else {
            1.0
        };

        // Write to file (simulated)
        // TODO: Replace with actual Parquet writer (arrow/parquet crate)
        tokio::fs::write(file_path, compressed_data).await.map_err(|e| BetaDataplaneError::from(e))?;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.update_publish(features.len() as u64, compressed_size as u64, latency_ms);
        metrics.update_batch(features.len());
        metrics.compression_ratio = Some(compression_ratio);

        info!(
            "Wrote {} features to {} (size: {} bytes, compression: {:.2}x, latency: {:.2}ms)",
            features.len(),
            file_path.display(),
            compressed_size,
            compression_ratio,
            latency_ms
        );

        Ok(())
    }

    /// Start the background writer
    async fn start_writer(&mut self) -> Result<()> {
        // Ensure output directory exists
        tokio::fs::create_dir_all(&self.output_dir).await.map_err(|e| BetaDataplaneError::from(e))?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let buffer = Arc::clone(&self.buffer);
        let metrics = Arc::clone(&self.metrics);
        let health = Arc::clone(&self.health);
        let current_file = Arc::clone(&self.current_file);
        let config = self.config.clone();
        let output_dir = self.output_dir.clone();
        let file_prefix = self.file_prefix.clone();
        let compressor = Arc::clone(&self.compressor);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(config.batch_timeout_ms));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Process buffer
                        let mut buffer_lock = buffer.write().await;
                        if !buffer_lock.is_empty() && buffer_lock.len() >= config.batch_size {
                            let batch = buffer_lock.drain(..).collect::<Vec<_>>();
                            drop(buffer_lock);

                            // Generate file path
                            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                            let filename = format!("{}_{}.parquet", file_prefix, timestamp);
                            let file_path = output_dir.join(filename);

                            // Create temporary feed for writing
                            let temp_feed = ParquetFeed {
                                name: "parquet",
                                output_dir: output_dir.clone(),
                                file_prefix: file_prefix.clone(),
                                config: config.clone(),
                                metrics: Arc::clone(&metrics),
                                health: Arc::clone(&health),
                                is_running: Arc::new(RwLock::new(true)),
                                buffer: Arc::clone(&buffer),
                                current_file: Arc::clone(&current_file),
                                compressor: Arc::clone(&compressor),
                                shutdown_tx: None,
                                writer_handle: None,
                            };

                            if let Err(e) = temp_feed.write_parquet(batch, &file_path).await {
                                error!("Failed to write Parquet file: {}", e);

                                let mut health_lock = health.write().await;
                                health_lock.update_status(
                                    FeedStatus::Degraded,
                                    Some(format!("Write failed: {}", e)),
                                );
                            } else {
                                // Update current file
                                let mut current_file_lock = current_file.write().await;
                                *current_file_lock = Some(file_path);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down Parquet writer");
                        break;
                    }
                }
            }
        });

        self.writer_handle = Some(handle);
        Ok(())
    }
}

#[async_trait]
impl BetaDataFeed for ParquetFeed {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn start(&mut self) -> Result<()> {
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(BetaDataplaneError::internal("Parquet feed is already running"));
            }
        }

        info!("Starting Parquet feed (output: {})", self.output_dir.display());

        // Update health
        {
            let mut health = self.health.write().await;
            health.update_status(FeedStatus::Connecting, None);
        }

        // Start writer
        self.start_writer().await?;

        // Update state
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        let mut health = self.health.write().await;
        health.update_status(FeedStatus::Healthy, None);

        info!("Parquet feed started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        info!("Stopping Parquet feed");

        // Flush remaining buffer
        self.flush().await?;

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Wait for writer handle to finish
        if let Some(handle) = self.writer_handle.take() {
            let _ = handle.await;
        }

        *is_running = false;

        let mut health = self.health.write().await;
        health.update_status(FeedStatus::Disabled, None);

        info!("Parquet feed stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        false // Placeholder
    }

    async fn publish_feature(&self, feature: Feature) -> Result<()> {
        if !*self.is_running.read().await {
            return Err(BetaDataplaneError::internal("Parquet feed is not running"));
        }

        let mut buffer = self.buffer.write().await;
        buffer.push(feature);

        // Update metrics
        let buffer_size = buffer.len();
        drop(buffer);

        let mut metrics = self.metrics.write().await;
        metrics.update_queue(buffer_size, self.config.max_queue_size);

        // Check if we should flush immediately
        if buffer_size >= self.config.batch_size {
            drop(metrics);
            self.flush().await?;
        }

        Ok(())
    }

    async fn publish_features(&self, features: Vec<Feature>) -> Result<()> {
        if !*self.is_running.read().await {
            return Err(BetaDataplaneError::internal("Parquet feed is not running"));
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
        info!("Updating Parquet feed configuration");
        self.config = config;
        Ok(())
    }

    fn metrics(&self) -> BetaFeedMetrics {
        BetaFeedMetrics::new("parquet".to_string())
    }

    fn health(&self) -> BetaFeedHealth {
        BetaFeedHealth::new("parquet".to_string())
    }

    async fn flush(&self) -> Result<()> {
        info!("Flushing Parquet feed");

        let mut buffer = self.buffer.write().await;
        if !buffer.is_empty() {
            let batch = buffer.drain(..).collect::<Vec<_>>();
            drop(buffer);

            let file_path = self.generate_file_path();
            self.write_parquet(batch, &file_path).await?;

            let mut current_file = self.current_file.write().await;
            *current_file = Some(file_path);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parquet_feed_creation() {
        let output_dir = PathBuf::from("/tmp/qenus_test");
        let file_prefix = "test_features".to_string();
        let config = BetaFeedConfig::default();

        let feed = ParquetFeed::new(output_dir, file_prefix, config);
        assert_eq!(feed.name(), "parquet");
    }

    #[test]
    fn test_file_path_generation() {
        let output_dir = PathBuf::from("/tmp/qenus");
        let file_prefix = "features".to_string();
        let config = BetaFeedConfig::default();

        let feed = ParquetFeed::new(output_dir.clone(), file_prefix, config);
        let file_path = feed.generate_file_path();

        assert!(file_path.starts_with(&output_dir));
        assert!(file_path.to_string_lossy().contains("features_"));
        assert!(file_path.extension().unwrap() == "parquet");
    }
}

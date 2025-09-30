//! Configuration management for the dataplane

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::{types::RpcConfig, Chain};

/// Main configuration structure for the dataplane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataplaneConfig {
    /// Global settings
    pub global: GlobalConfig,
    
    /// Chain-specific configurations
    pub chains: HashMap<Chain, ChainConfig>,
    
    /// Observer configurations
    pub observers: ObserverConfig,
    
    /// Feature extractor configurations
    pub extractors: ExtractorConfig,
    
    /// Data feed configurations
    pub feeds: FeedConfig,
    
    /// Monitoring and observability
    pub monitoring: MonitoringConfig,
}

/// Global dataplane settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    
    /// Number of worker threads
    pub worker_threads: usize,
    
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    
    /// Graceful shutdown timeout in seconds
    pub shutdown_timeout_seconds: u64,
    
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
}

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Primary RPC endpoint
    pub primary_rpc: RpcConfig,
    
    /// Fallback RPC endpoints
    pub fallback_rpcs: Vec<RpcConfig>,
    
    /// Block confirmation requirements
    pub confirmations: u64,
    
    /// Maximum block lag before alerting
    pub max_block_lag: u64,
    
    /// Contract addresses to monitor
    pub contracts: ContractConfig,
    
    /// Chain-specific feature flags
    pub features: ChainFeatures,
}

/// Contract addresses for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    /// DEX factory contracts
    pub dex_factories: Vec<String>,
    
    /// Bridge contracts
    pub bridges: Vec<String>,
    
    /// Flash loan providers
    pub flash_loan_providers: Vec<String>,
    
    /// Important token contracts
    pub tokens: Vec<String>,
}

/// Chain-specific feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainFeatures {
    /// Enable AMM monitoring
    pub amm_monitoring: bool,
    
    /// Enable bridge monitoring
    pub bridge_monitoring: bool,
    
    /// Enable gas monitoring
    pub gas_monitoring: bool,
    
    /// Enable flash loan monitoring
    pub flash_loan_monitoring: bool,
    
    /// Enable sequencer health monitoring (L2 only)
    pub sequencer_monitoring: bool,
}

/// Observer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverConfig {
    /// Buffer size for event queues
    pub event_buffer_size: usize,
    
    /// Batch size for processing events
    pub batch_size: usize,
    
    /// Processing interval in milliseconds
    pub processing_interval_ms: u64,
    
    /// Maximum retry attempts for failed requests
    pub max_retries: u32,
    
    /// Retry backoff multiplier
    pub retry_backoff_multiplier: f64,
    
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
}

/// Feature extractor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    /// AMM extractor settings
    pub amm: AmmExtractorConfig,
    
    /// Bridge extractor settings
    pub bridge: BridgeExtractorConfig,
    
    /// Gas extractor settings
    pub gas: GasExtractorConfig,
    
    /// Flash loan extractor settings
    pub flash_loan: FlashLoanExtractorConfig,
    
    /// Sequencer health extractor settings
    pub sequencer: SequencerExtractorConfig,
}

/// AMM extractor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmExtractorConfig {
    /// Minimum liquidity threshold for monitoring
    pub min_liquidity_usd: f64,
    
    /// Depth curve calculation sizes (in USD)
    pub depth_sizes: Vec<u64>,
    
    /// Price impact threshold for alerts
    pub price_impact_threshold: f64,
    
    /// Update frequency in seconds
    pub update_frequency_seconds: u64,
}

/// Bridge extractor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeExtractorConfig {
    /// Minimum bridge liquidity for monitoring
    pub min_liquidity_usd: f64,
    
    /// Maximum acceptable fee in basis points
    pub max_fee_bps: u32,
    
    /// Settlement time estimation window in hours
    pub settlement_window_hours: u64,
}

/// Gas extractor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasExtractorConfig {
    /// Number of blocks for gas price calculation
    pub calculation_window: u64,
    
    /// Percentiles for gas price recommendations
    pub percentiles: Vec<f64>,
    
    /// Base fee prediction model parameters
    pub prediction_params: GasPredictionParams,
}

/// Gas price prediction parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPredictionParams {
    /// EMA alpha for base fee prediction
    pub base_fee_alpha: f64,
    
    /// Target gas used ratio
    pub target_gas_ratio: f64,
    
    /// Maximum base fee change per block
    pub max_change_denominator: u64,
}

/// Flash loan extractor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanExtractorConfig {
    /// Minimum available liquidity for monitoring
    pub min_available_liquidity_usd: f64,
    
    /// Maximum acceptable fee in basis points
    pub max_fee_bps: u32,
    
    /// Update frequency in seconds
    pub update_frequency_seconds: u64,
}

/// Sequencer health extractor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerExtractorConfig {
    /// Block interval monitoring window
    pub monitoring_window_blocks: u64,
    
    /// Uptime calculation window in hours
    pub uptime_window_hours: u64,
    
    /// Alert thresholds
    pub alert_thresholds: SequencerAlertThresholds,
}

/// Sequencer alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerAlertThresholds {
    /// Maximum acceptable block interval variance
    pub max_interval_variance: f64,
    
    /// Minimum acceptable uptime percentage
    pub min_uptime_percentage: f64,
    
    /// Maximum acceptable pending transaction count
    pub max_pending_tx_count: u64,
}

/// Data feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    /// Kafka configuration
    pub kafka: KafkaConfig,
    
    /// gRPC server configuration
    pub grpc: GrpcConfig,
    
    /// Parquet writer configuration
    pub parquet: ParquetConfig,
    
    /// Redis configuration for caching
    pub redis: RedisConfig,
}

/// Kafka/Redpanda configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Bootstrap servers
    pub bootstrap_servers: Vec<String>,
    
    /// Topic configurations
    pub topics: HashMap<String, TopicConfig>,
    
    /// Producer settings
    pub producer: KafkaProducerConfig,
    
    /// Security settings
    pub security: Option<KafkaSecurityConfig>,
}

/// Kafka topic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicConfig {
    /// Number of partitions
    pub partitions: u32,
    
    /// Replication factor
    pub replication_factor: u16,
    
    /// Retention time in hours
    pub retention_hours: u64,
}

/// Kafka producer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaProducerConfig {
    /// Batch size
    pub batch_size: u32,
    
    /// Linger time in milliseconds
    pub linger_ms: u32,
    
    /// Compression type
    pub compression_type: String,
    
    /// Acknowledgment level
    pub acks: String,
    
    /// Request timeout in milliseconds
    pub request_timeout_ms: u32,
}

/// Kafka security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaSecurityConfig {
    /// Security protocol
    pub security_protocol: String,
    
    /// SASL mechanism
    pub sasl_mechanism: Option<String>,
    
    /// SASL username
    pub sasl_username: Option<String>,
    
    /// SASL password
    pub sasl_password: Option<String>,
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// Server bind address
    pub bind_address: String,
    
    /// Server port
    pub port: u16,
    
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    
    /// Maximum concurrent streams
    pub max_concurrent_streams: u32,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_file: String,
    
    /// Private key file path
    pub key_file: String,
    
    /// CA certificate file path
    pub ca_file: Option<String>,
}

/// Parquet writer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParquetConfig {
    /// Output directory
    pub output_dir: String,
    
    /// File rotation interval in hours
    pub rotation_interval_hours: u64,
    
    /// Compression algorithm
    pub compression: String,
    
    /// Row group size
    pub row_group_size: usize,
    
    /// Write batch size
    pub batch_size: usize,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,
    
    /// Connection pool size
    pub pool_size: u32,
    
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    
    /// Default TTL in seconds
    pub default_ttl_seconds: u64,
}

/// Monitoring and observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Metrics configuration
    pub metrics: MetricsConfig,
    
    /// Health check configuration
    pub health_check: HealthCheckConfig,
    
    /// Alerting configuration
    pub alerting: AlertingConfig,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Prometheus exporter bind address
    pub prometheus_bind_address: String,
    
    /// Prometheus exporter port
    pub prometheus_port: u16,
    
    /// Metrics collection interval in seconds
    pub collection_interval_seconds: u64,
    
    /// Histogram buckets for latency metrics
    pub latency_buckets: Vec<f64>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check endpoint bind address
    pub bind_address: String,
    
    /// Health check endpoint port
    pub port: u16,
    
    /// Check interval in seconds
    pub check_interval_seconds: u64,
    
    /// Timeout for individual checks in seconds
    pub check_timeout_seconds: u64,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    
    /// Webhook URLs for alerts
    pub webhook_urls: Vec<String>,
    
    /// Alert severity levels
    pub severity_levels: Vec<String>,
    
    /// Rate limiting for alerts
    pub rate_limit_per_hour: u32,
}

impl DataplaneConfig {
    /// Load configuration from files and environment
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Config::builder()
            // Start with default values
            .add_source(File::with_name("config/default").required(false))
            // Add environment-specific config
            .add_source(File::with_name(&format!(
                "config/{}",
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into())
            )).required(false))
            // Add local config (gitignored)
            .add_source(File::with_name("config/local").required(false))
            // Add environment variables with DATAPLANE_ prefix
            .add_source(Environment::with_prefix("DATAPLANE").separator("__"))
            .build()?;

        config.try_deserialize()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate global settings
        if self.global.worker_threads == 0 {
            return Err(ConfigError::Message("worker_threads must be greater than 0".into()));
        }

        // Validate chain configurations
        for (chain, config) in &self.chains {
            // Validate RPC URLs
            if let Err(_) = Url::parse(&config.primary_rpc.url) {
                return Err(ConfigError::Message(format!(
                    "Invalid primary RPC URL for chain {}: {}",
                    chain, config.primary_rpc.url
                )));
            }

            for (i, fallback) in config.fallback_rpcs.iter().enumerate() {
                if let Err(_) = Url::parse(&fallback.url) {
                    return Err(ConfigError::Message(format!(
                        "Invalid fallback RPC URL {} for chain {}: {}",
                        i, chain, fallback.url
                    )));
                }
            }
        }

        // Validate Kafka configuration
        if self.feeds.kafka.bootstrap_servers.is_empty() {
            return Err(ConfigError::Message("Kafka bootstrap servers cannot be empty".into()));
        }

        // Validate Redis URL
        if let Err(_) = Url::parse(&self.feeds.redis.url) {
            return Err(ConfigError::Message(format!(
                "Invalid Redis URL: {}",
                self.feeds.redis.url
            )));
        }

        Ok(())
    }

    /// Get configuration for a specific chain
    pub fn get_chain_config(&self, chain: Chain) -> Option<&ChainConfig> {
        self.chains.get(&chain)
    }

    /// Check if a feature is enabled for a chain
    pub fn is_feature_enabled(&self, chain: Chain, feature: &str) -> bool {
        if let Some(config) = self.get_chain_config(chain) {
            match feature {
                "amm" => config.features.amm_monitoring,
                "bridge" => config.features.bridge_monitoring,
                "gas" => config.features.gas_monitoring,
                "flash_loan" => config.features.flash_loan_monitoring,
                "sequencer" => config.features.sequencer_monitoring,
                _ => false,
            }
        } else {
            false
        }
    }
}

impl Default for DataplaneConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig {
                log_level: "info".to_string(),
                worker_threads: num_cpus::get(),
                max_memory_mb: 4096,
                shutdown_timeout_seconds: 30,
                health_check_interval_seconds: 30,
            },
            chains: HashMap::new(),
            observers: ObserverConfig {
                event_buffer_size: 10000,
                batch_size: 100,
                processing_interval_ms: 100,
                max_retries: 3,
                retry_backoff_multiplier: 2.0,
                connection_timeout_seconds: 30,
                request_timeout_seconds: 10,
            },
            extractors: ExtractorConfig {
                amm: AmmExtractorConfig {
                    min_liquidity_usd: 10000.0,
                    depth_sizes: vec![100_000, 1_000_000, 10_000_000],
                    price_impact_threshold: 0.05,
                    update_frequency_seconds: 1,
                },
                bridge: BridgeExtractorConfig {
                    min_liquidity_usd: 100_000.0,
                    max_fee_bps: 1000,
                    settlement_window_hours: 24,
                },
                gas: GasExtractorConfig {
                    calculation_window: 20,
                    percentiles: vec![0.1, 0.25, 0.5, 0.75, 0.9],
                    prediction_params: GasPredictionParams {
                        base_fee_alpha: 0.125,
                        target_gas_ratio: 0.5,
                        max_change_denominator: 8,
                    },
                },
                flash_loan: FlashLoanExtractorConfig {
                    min_available_liquidity_usd: 1_000_000.0,
                    max_fee_bps: 100,
                    update_frequency_seconds: 5,
                },
                sequencer: SequencerExtractorConfig {
                    monitoring_window_blocks: 100,
                    uptime_window_hours: 24,
                    alert_thresholds: SequencerAlertThresholds {
                        max_interval_variance: 2.0,
                        min_uptime_percentage: 99.0,
                        max_pending_tx_count: 10000,
                    },
                },
            },
            feeds: FeedConfig {
                kafka: KafkaConfig {
                    bootstrap_servers: vec!["localhost:9092".to_string()],
                    topics: HashMap::new(),
                    producer: KafkaProducerConfig {
                        batch_size: 16384,
                        linger_ms: 5,
                        compression_type: "snappy".to_string(),
                        acks: "all".to_string(),
                        request_timeout_ms: 30000,
                    },
                    security: None,
                },
                grpc: GrpcConfig {
                    bind_address: "0.0.0.0".to_string(),
                    port: 50051,
                    tls: None,
                    request_timeout_seconds: 30,
                    max_concurrent_streams: 1000,
                },
                parquet: ParquetConfig {
                    output_dir: "./data/parquet".to_string(),
                    rotation_interval_hours: 1,
                    compression: "snappy".to_string(),
                    row_group_size: 100000,
                    batch_size: 1000,
                },
                redis: RedisConfig {
                    url: "redis://localhost:6379".to_string(),
                    pool_size: 10,
                    connection_timeout_seconds: 5,
                    default_ttl_seconds: 3600,
                },
            },
            monitoring: MonitoringConfig {
                metrics: MetricsConfig {
                    prometheus_bind_address: "0.0.0.0".to_string(),
                    prometheus_port: 9090,
                    collection_interval_seconds: 15,
                    latency_buckets: vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0],
                },
                health_check: HealthCheckConfig {
                    bind_address: "0.0.0.0".to_string(),
                    port: 8080,
                    check_interval_seconds: 30,
                    check_timeout_seconds: 5,
                },
                alerting: AlertingConfig {
                    enabled: false,
                    webhook_urls: vec![],
                    severity_levels: vec!["info".to_string(), "warn".to_string(), "error".to_string()],
                    rate_limit_per_hour: 100,
                },
            },
        }
    }
}

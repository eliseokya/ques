//! Configuration management for the beta dataplane

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::{Chain, ProviderType, OperationalMode, Result};

/// Main configuration structure for the beta dataplane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaDataplaneConfig {
    /// Global settings
    pub global: GlobalConfig,
    
    /// Provider configurations
    pub providers: ProvidersConfig,
    
    /// Chain-specific configurations
    pub chains: HashMap<Chain, ChainConfig>,
    
    /// Feature extraction configurations
    pub extraction: ExtractionConfig,
    
    /// Data feed configurations
    pub feeds: FeedsConfig,
    
    /// Optimization configurations
    pub optimization: OptimizationConfig,
    
    /// Monitoring configurations
    pub monitoring: MonitoringConfig,
}

/// Global beta dataplane settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Operational mode
    pub mode: OperationalMode,
    
    /// Log level
    pub log_level: String,
    
    /// Number of worker threads
    pub worker_threads: usize,
    
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    
    /// Graceful shutdown timeout in seconds
    pub shutdown_timeout_seconds: u64,
    
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    
    /// Enable dry-run mode (no data publishing)
    pub dry_run: bool,
}

/// Provider configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Default provider selection strategy
    pub selection_strategy: ProviderSelectionStrategy,
    
    /// Provider-specific configurations
    pub ethereum: Vec<ProviderConfig>,
    pub arbitrum: Vec<ProviderConfig>,
    pub optimism: Vec<ProviderConfig>,
    pub base: Vec<ProviderConfig>,
    
    /// Global provider settings
    pub global_settings: GlobalProviderSettings,
}

/// Provider selection strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSelectionStrategy {
    /// Use fastest responding provider
    FastestFirst,
    
    /// Round-robin across providers
    RoundRobin,
    
    /// Weighted selection based on reliability
    Weighted,
    
    /// Primary with fallback
    PrimaryFallback,
}

/// Individual provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type
    pub provider_type: ProviderType,
    
    /// Provider name/identifier
    pub name: String,
    
    /// HTTP RPC URL
    pub http_url: String,
    
    /// WebSocket URL (optional)
    pub ws_url: Option<String>,
    
    /// API key (optional)
    pub api_key: Option<String>,
    
    /// Rate limit (requests per second)
    pub rate_limit: u32,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Provider weight for weighted selection
    pub weight: f64,
    
    /// Enable this provider
    pub enabled: bool,
}

/// Global provider settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalProviderSettings {
    /// Default request timeout in seconds
    pub default_timeout_seconds: u64,
    
    /// Default retry attempts
    pub default_max_retries: u32,
    
    /// Connection pool size per provider
    pub connection_pool_size: usize,
    
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
    
    /// Failover threshold (failed requests before switching)
    pub failover_threshold: u32,
}

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Enable monitoring for this chain
    pub enabled: bool,
    
    /// Block confirmation requirements
    pub confirmations: u64,
    
    /// Maximum block lag before alerting
    pub max_block_lag: u64,
    
    /// Contract addresses to monitor
    pub contracts: ContractConfig,
    
    /// Chain-specific feature flags
    pub features: ChainFeatures,
    
    /// Chain-specific optimization settings
    pub optimization: ChainOptimization,
}

/// Contract addresses for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    /// DEX factory contracts
    pub dex_factories: Vec<String>,
    
    /// DEX router contracts
    pub dex_routers: Vec<String>,
    
    /// Bridge contracts
    pub bridges: Vec<String>,
    
    /// Flash loan providers
    pub flash_loan_providers: Vec<String>,
    
    /// Important token contracts
    pub tokens: Vec<String>,
    
    /// Custom contracts to monitor
    pub custom: Vec<String>,
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

/// Chain-specific optimization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOptimization {
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    
    /// Batch size for queries
    pub batch_size: usize,
    
    /// Query parallelism level
    pub parallelism: usize,
    
    /// Enable predictive pre-fetching
    pub predictive_fetching: bool,
}

/// Feature extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Enable feature extraction
    pub enabled: bool,
    
    /// Extraction batch size
    pub batch_size: usize,
    
    /// Extraction timeout in seconds
    pub timeout_seconds: u64,
    
    /// AMM extraction settings
    pub amm: AmmExtractionConfig,
    
    /// Bridge extraction settings
    pub bridge: BridgeExtractionConfig,
    
    /// Gas extraction settings
    pub gas: GasExtractionConfig,
    
    /// Flash loan extraction settings
    pub flash_loan: FlashLoanExtractionConfig,
}

/// AMM extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmExtractionConfig {
    /// Enable AMM extraction
    pub enabled: bool,
    
    /// Minimum liquidity threshold (USD)
    pub min_liquidity_usd: f64,
    
    /// Update frequency in seconds
    pub update_frequency_seconds: u64,
    
    /// Depth curve calculation sizes
    pub depth_sizes: Vec<u64>,
    
    /// Price impact threshold for alerts
    pub price_impact_threshold: f64,
}

/// Bridge extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeExtractionConfig {
    /// Enable bridge extraction
    pub enabled: bool,
    
    /// Minimum bridge liquidity (USD)
    pub min_liquidity_usd: f64,
    
    /// Update frequency in seconds
    pub update_frequency_seconds: u64,
    
    /// Maximum acceptable fee in basis points
    pub max_fee_bps: u32,
}

/// Gas extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasExtractionConfig {
    /// Enable gas extraction
    pub enabled: bool,
    
    /// Number of blocks for calculation
    pub calculation_window: u64,
    
    /// Gas price percentiles to track
    pub percentiles: Vec<f64>,
    
    /// Update frequency in seconds
    pub update_frequency_seconds: u64,
}

/// Flash loan extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanExtractionConfig {
    /// Enable flash loan extraction
    pub enabled: bool,
    
    /// Minimum available liquidity (USD)
    pub min_liquidity_usd: f64,
    
    /// Update frequency in seconds
    pub update_frequency_seconds: u64,
    
    /// Maximum acceptable fee in basis points
    pub max_fee_bps: u32,
}

/// Data feeds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedsConfig {
    /// Kafka configuration
    pub kafka: KafkaConfig,
    
    /// gRPC configuration
    pub grpc: GrpcConfig,
    
    /// Parquet configuration
    pub parquet: ParquetConfig,
    
    /// Redis configuration
    pub redis: RedisConfig,
}

/// Kafka configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Enable Kafka feed
    pub enabled: bool,
    
    /// Bootstrap servers
    pub bootstrap_servers: Vec<String>,
    
    /// Topic prefix
    pub topic_prefix: String,
    
    /// Producer settings
    pub producer: KafkaProducerConfig,
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
}

/// gRPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// Enable gRPC server
    pub enabled: bool,
    
    /// Bind address
    pub bind_address: String,
    
    /// Port
    pub port: u16,
    
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
}

/// Parquet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParquetConfig {
    /// Enable Parquet archival
    pub enabled: bool,
    
    /// Output directory
    pub output_dir: String,
    
    /// File rotation interval in hours
    pub rotation_interval_hours: u64,
    
    /// Compression algorithm
    pub compression: String,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,
    
    /// Connection pool size
    pub pool_size: u32,
    
    /// Default TTL in seconds
    pub default_ttl_seconds: u64,
}

/// Optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Caching configuration
    pub caching: CachingConfig,
    
    /// Batching configuration
    pub batching: BatchingConfig,
    
    /// Prediction configuration
    pub prediction: PredictionConfig,
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    /// Enable caching
    pub enabled: bool,
    
    /// Cache size in MB
    pub cache_size_mb: usize,
    
    /// Default TTL in seconds
    pub default_ttl_seconds: u64,
    
    /// Cache hit ratio target
    pub target_hit_ratio: f64,
}

/// Batching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    /// Enable batching
    pub enabled: bool,
    
    /// Default batch size
    pub default_batch_size: usize,
    
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
    
    /// Maximum batch size
    pub max_batch_size: usize,
}

/// Prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionConfig {
    /// Enable predictive pre-fetching
    pub enabled: bool,
    
    /// Prediction window in seconds
    pub prediction_window_seconds: u64,
    
    /// Confidence threshold for predictions
    pub confidence_threshold: f64,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Health check configuration
    pub health: HealthConfig,
    
    /// Metrics configuration
    pub metrics: MetricsConfig,
    
    /// Alerting configuration
    pub alerting: AlertingConfig,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Enable health checks
    pub enabled: bool,
    
    /// Health check interval in seconds
    pub check_interval_seconds: u64,
    
    /// Health check timeout in seconds
    pub timeout_seconds: u64,
    
    /// Bind address for health endpoint
    pub bind_address: String,
    
    /// Port for health endpoint
    pub port: u16,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    
    /// Prometheus bind address
    pub prometheus_bind_address: String,
    
    /// Prometheus port
    pub prometheus_port: u16,
    
    /// Metrics collection interval in seconds
    pub collection_interval_seconds: u64,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    
    /// Webhook URLs for alerts
    pub webhook_urls: Vec<String>,
    
    /// Alert rate limiting per hour
    pub rate_limit_per_hour: u32,
}

impl BetaDataplaneConfig {
    /// Load configuration from files and environment
    pub fn load() -> std::result::Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with default values
            .add_source(File::with_name("config/beta-dataplane").required(false))
            // Add environment-specific config
            .add_source(File::with_name(&format!(
                "config/beta-dataplane-{}",
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into())
            )).required(false))
            // Add local config (gitignored)
            .add_source(File::with_name("config/beta-dataplane-local").required(false))
            // Add environment variables with BETA_DATAPLANE_ prefix
            .add_source(Environment::with_prefix("BETA_DATAPLANE").separator("__"))
            .build()?;

        config.try_deserialize()
    }

    /// Validate the configuration
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        // Validate global settings
        if self.global.worker_threads == 0 {
            return Err(ConfigError::Message("worker_threads must be greater than 0".into()));
        }

        // Validate provider configurations
        for (chain, providers) in [
            (Chain::Ethereum, &self.providers.ethereum),
            (Chain::Arbitrum, &self.providers.arbitrum),
            (Chain::Optimism, &self.providers.optimism),
            (Chain::Base, &self.providers.base),
        ] {
            if providers.is_empty() {
                return Err(ConfigError::Message(format!(
                    "No providers configured for chain: {}", chain
                )));
            }

            for provider in providers {
                // Validate URLs
                if let Err(_) = Url::parse(&provider.http_url) {
                    return Err(ConfigError::Message(format!(
                        "Invalid HTTP URL for provider {}: {}", 
                        provider.name, provider.http_url
                    )));
                }

                if let Some(ref ws_url) = provider.ws_url {
                    if let Err(_) = Url::parse(ws_url) {
                        return Err(ConfigError::Message(format!(
                            "Invalid WebSocket URL for provider {}: {}", 
                            provider.name, ws_url
                        )));
                    }
                }

                // Validate rate limits
                if provider.rate_limit == 0 {
                    return Err(ConfigError::Message(format!(
                        "Rate limit must be greater than 0 for provider: {}", 
                        provider.name
                    )));
                }
            }
        }

        // Validate Redis URL
        if let Err(_) = Url::parse(&self.feeds.redis.url) {
            return Err(ConfigError::Message(format!(
                "Invalid Redis URL: {}", self.feeds.redis.url
            )));
        }

        Ok(())
    }

    /// Get providers for a specific chain
    pub fn get_providers_for_chain(&self, chain: Chain) -> &Vec<ProviderConfig> {
        match chain {
            Chain::Ethereum => &self.providers.ethereum,
            Chain::Arbitrum => &self.providers.arbitrum,
            Chain::Optimism => &self.providers.optimism,
            Chain::Base => &self.providers.base,
        }
    }

    /// Check if a feature is enabled for a chain
    pub fn is_feature_enabled(&self, chain: Chain, feature: &str) -> bool {
        if let Some(config) = self.chains.get(&chain) {
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

impl Default for BetaDataplaneConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig {
                mode: OperationalMode::Development,
                log_level: "info".to_string(),
                worker_threads: num_cpus::get(),
                max_memory_mb: 2048,
                shutdown_timeout_seconds: 30,
                health_check_interval_seconds: 30,
                dry_run: false,
            },
            providers: ProvidersConfig {
                selection_strategy: ProviderSelectionStrategy::FastestFirst,
                ethereum: vec![
                    ProviderConfig {
                        provider_type: ProviderType::Alchemy,
                        name: "alchemy-ethereum".to_string(),
                        http_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
                        ws_url: Some("wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()),
                        api_key: None,
                        rate_limit: 300,
                        timeout_seconds: 30,
                        max_retries: 3,
                        weight: 1.0,
                        enabled: true,
                    },
                ],
                arbitrum: vec![
                    ProviderConfig {
                        provider_type: ProviderType::Alchemy,
                        name: "alchemy-arbitrum".to_string(),
                        http_url: "https://arb-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
                        ws_url: Some("wss://arb-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()),
                        api_key: None,
                        rate_limit: 300,
                        timeout_seconds: 30,
                        max_retries: 3,
                        weight: 1.0,
                        enabled: true,
                    },
                ],
                optimism: vec![
                    ProviderConfig {
                        provider_type: ProviderType::Alchemy,
                        name: "alchemy-optimism".to_string(),
                        http_url: "https://opt-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
                        ws_url: Some("wss://opt-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()),
                        api_key: None,
                        rate_limit: 300,
                        timeout_seconds: 30,
                        max_retries: 3,
                        weight: 1.0,
                        enabled: true,
                    },
                ],
                base: vec![
                    ProviderConfig {
                        provider_type: ProviderType::Alchemy,
                        name: "alchemy-base".to_string(),
                        http_url: "https://base-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
                        ws_url: Some("wss://base-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()),
                        api_key: None,
                        rate_limit: 300,
                        timeout_seconds: 30,
                        max_retries: 3,
                        weight: 1.0,
                        enabled: true,
                    },
                ],
                global_settings: GlobalProviderSettings {
                    default_timeout_seconds: 30,
                    default_max_retries: 3,
                    connection_pool_size: 10,
                    health_check_interval_seconds: 60,
                    failover_threshold: 5,
                },
            },
            chains: HashMap::new(),
            extraction: ExtractionConfig {
                enabled: true,
                batch_size: 100,
                timeout_seconds: 30,
                amm: AmmExtractionConfig {
                    enabled: true,
                    min_liquidity_usd: 10000.0,
                    update_frequency_seconds: 1,
                    depth_sizes: vec![100_000, 1_000_000, 10_000_000],
                    price_impact_threshold: 0.05,
                },
                bridge: BridgeExtractionConfig {
                    enabled: true,
                    min_liquidity_usd: 100_000.0,
                    update_frequency_seconds: 5,
                    max_fee_bps: 1000,
                },
                gas: GasExtractionConfig {
                    enabled: true,
                    calculation_window: 20,
                    percentiles: vec![0.1, 0.25, 0.5, 0.75, 0.9],
                    update_frequency_seconds: 1,
                },
                flash_loan: FlashLoanExtractionConfig {
                    enabled: true,
                    min_liquidity_usd: 1_000_000.0,
                    update_frequency_seconds: 5,
                    max_fee_bps: 100,
                },
            },
            feeds: FeedsConfig {
                kafka: KafkaConfig {
                    enabled: true,
                    bootstrap_servers: vec!["localhost:9092".to_string()],
                    topic_prefix: "qenus.beta".to_string(),
                    producer: KafkaProducerConfig {
                        batch_size: 100,
                        linger_ms: 5,
                        compression_type: "snappy".to_string(),
                        acks: "all".to_string(),
                    },
                },
                grpc: GrpcConfig {
                    enabled: true,
                    bind_address: "0.0.0.0".to_string(),
                    port: 50053,
                    request_timeout_seconds: 30,
                },
                parquet: ParquetConfig {
                    enabled: true,
                    output_dir: "./data/beta-dataplane/parquet".to_string(),
                    rotation_interval_hours: 1,
                    compression: "snappy".to_string(),
                },
                redis: RedisConfig {
                    url: "redis://localhost:6379".to_string(),
                    pool_size: 10,
                    default_ttl_seconds: 300, // 5 minutes for beta
                },
            },
            optimization: OptimizationConfig {
                caching: CachingConfig {
                    enabled: true,
                    cache_size_mb: 512,
                    default_ttl_seconds: 60,
                    target_hit_ratio: 0.8,
                },
                batching: BatchingConfig {
                    enabled: true,
                    default_batch_size: 50,
                    batch_timeout_ms: 100,
                    max_batch_size: 200,
                },
                prediction: PredictionConfig {
                    enabled: true,
                    prediction_window_seconds: 30,
                    confidence_threshold: 0.7,
                },
            },
            monitoring: MonitoringConfig {
                health: HealthConfig {
                    enabled: true,
                    check_interval_seconds: 30,
                    timeout_seconds: 5,
                    bind_address: "0.0.0.0".to_string(),
                    port: 8080,
                },
                metrics: MetricsConfig {
                    enabled: true,
                    prometheus_bind_address: "0.0.0.0".to_string(),
                    prometheus_port: 9092,
                    collection_interval_seconds: 15,
                },
                alerting: AlertingConfig {
                    enabled: false,
                    webhook_urls: vec![],
                    rate_limit_per_hour: 100,
                },
            },
        }
    }
}

//! Configuration for the Qenus Reth fork

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::Result;

/// Main configuration for the Qenus Reth fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QenusRethConfig {
    /// Node configuration
    pub node: NodeConfig,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Sync configuration
    pub sync: SyncConfig,
    
    /// Feature extraction configuration
    pub extraction: ExtractionConfig,
    
    /// Data feeds configuration
    pub feeds: FeedsConfig,
    
    /// Observability configuration
    pub observability: ObservabilityConfig,
}

/// Node-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Data directory for the node
    pub data_dir: PathBuf,
    
    /// Chain specification (should be mainnet)
    pub chain: String,
    
    /// Node identity
    pub identity: String,
    
    /// Maximum number of peers
    pub max_peers: usize,
    
    /// Enable discovery
    pub discovery: bool,
    
    /// HTTP RPC configuration (minimal)
    pub http_rpc: Option<HttpRpcConfig>,
}

/// HTTP RPC configuration (minimal for internal use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRpcConfig {
    /// RPC bind address
    pub addr: String,
    
    /// RPC port
    pub port: u16,
    
    /// Enabled RPC modules (minimal set)
    pub modules: Vec<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database path
    pub path: PathBuf,
    
    /// Maximum database size in GB
    pub max_size_gb: u64,
    
    /// Cache size in MB
    pub cache_size_mb: u64,
    
    /// Number of database threads
    pub threads: usize,
    
    /// Enable compression
    pub compression: bool,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// P2P listen address
    pub listen_addr: String,
    
    /// P2P port
    pub port: u16,
    
    /// Boot nodes for initial peer discovery
    pub boot_nodes: Vec<String>,
    
    /// Trusted peers
    pub trusted_peers: Vec<String>,
    
    /// Maximum concurrent connections
    pub max_connections: usize,
    
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Sync mode (full, fast, snap)
    pub mode: String,
    
    /// Maximum blocks to download in parallel
    pub max_parallel_downloads: usize,
    
    /// Block download timeout in seconds
    pub download_timeout_seconds: u64,
    
    /// Enable checkpoint sync
    pub checkpoint_sync: bool,
    
    /// Checkpoint sync URL
    pub checkpoint_url: Option<String>,
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
    
    /// AMM extraction configuration
    pub amm: AmmExtractionConfig,
    
    /// Bridge extraction configuration
    pub bridge: BridgeExtractionConfig,
    
    /// Gas extraction configuration
    pub gas: GasExtractionConfig,
    
    /// Flash loan extraction configuration
    pub flash_loan: FlashLoanExtractionConfig,
}

/// AMM extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmExtractionConfig {
    /// Enable AMM extraction
    pub enabled: bool,
    
    /// Uniswap V3 factory address
    pub uniswap_v3_factory: String,
    
    /// Curve registry address
    pub curve_registry: String,
    
    /// Balancer vault address
    pub balancer_vault: String,
    
    /// Minimum liquidity threshold (USD)
    pub min_liquidity_usd: f64,
}

/// Bridge extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeExtractionConfig {
    /// Enable bridge extraction
    pub enabled: bool,
    
    /// Canonical bridge addresses
    pub canonical_bridges: Vec<String>,
    
    /// Third-party bridge addresses
    pub third_party_bridges: Vec<String>,
    
    /// Minimum bridge liquidity (USD)
    pub min_liquidity_usd: f64,
}

/// Gas extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasExtractionConfig {
    /// Enable gas extraction
    pub enabled: bool,
    
    /// Number of blocks for gas calculation
    pub calculation_window: u64,
    
    /// Gas price percentiles to track
    pub percentiles: Vec<f64>,
}

/// Flash loan extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanExtractionConfig {
    /// Enable flash loan extraction
    pub enabled: bool,
    
    /// Aave V3 pool addresses
    pub aave_v3_pools: Vec<String>,
    
    /// Balancer vault address
    pub balancer_vault: String,
    
    /// Minimum available liquidity (USD)
    pub min_liquidity_usd: f64,
}

/// Data feeds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedsConfig {
    /// Kafka configuration
    pub kafka: KafkaFeedConfig,
    
    /// gRPC configuration
    pub grpc: GrpcFeedConfig,
    
    /// Archive configuration
    pub archive: ArchiveFeedConfig,
}

/// Kafka feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaFeedConfig {
    /// Enable Kafka feed
    pub enabled: bool,
    
    /// Kafka bootstrap servers
    pub bootstrap_servers: Vec<String>,
    
    /// Topic prefix
    pub topic_prefix: String,
    
    /// Batch size
    pub batch_size: usize,
    
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
}

/// gRPC feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcFeedConfig {
    /// Enable gRPC feed
    pub enabled: bool,
    
    /// gRPC bind address
    pub bind_addr: String,
    
    /// gRPC port
    pub port: u16,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

/// Archive feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveFeedConfig {
    /// Enable archive feed
    pub enabled: bool,
    
    /// Archive directory
    pub archive_dir: PathBuf,
    
    /// Compression algorithm
    pub compression: String,
    
    /// Rotation interval in hours
    pub rotation_hours: u64,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level
    pub log_level: String,
    
    /// Metrics bind address
    pub metrics_addr: String,
    
    /// Metrics port
    pub metrics_port: u16,
    
    /// Tracing configuration
    pub tracing: TracingConfig,
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable tracing
    pub enabled: bool,
    
    /// Tracing endpoint
    pub endpoint: Option<String>,
    
    /// Service name
    pub service_name: String,
}

impl QenusRethConfig {
    /// Load configuration from files and environment
    pub fn load() -> std::result::Result<Self, config::ConfigError> {
        let config = Config::builder()
            // Start with default values
            .add_source(File::with_name("config/qenus-reth").required(false))
            // Add environment-specific config
            .add_source(File::with_name(&format!(
                "config/qenus-reth-{}",
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into())
            )).required(false))
            // Add local config (gitignored)
            .add_source(File::with_name("config/qenus-reth-local").required(false))
            // Add environment variables with QENUS_RETH_ prefix
            .add_source(Environment::with_prefix("QENUS_RETH").separator("__"))
            .build()?;

        config.try_deserialize()
    }

    /// Validate the configuration
    pub fn validate(&self) -> std::result::Result<(), config::ConfigError> {
        // Validate chain is mainnet
        if self.node.chain != "mainnet" {
            return Err(config::ConfigError::Message(
                "Qenus Reth only supports Ethereum mainnet".into()
            ));
        }

        // Validate data directory exists or can be created
        if !self.node.data_dir.exists() {
            std::fs::create_dir_all(&self.node.data_dir)
                .map_err(|e| config::ConfigError::Message(format!(
                    "Cannot create data directory: {}", e
                )))?;
        }

        // Validate database configuration
        if self.database.max_size_gb == 0 {
            return Err(config::ConfigError::Message(
                "Database max_size_gb must be greater than 0".into()
            ));
        }

        // Validate network configuration
        if self.network.max_connections == 0 {
            return Err(config::ConfigError::Message(
                "Network max_connections must be greater than 0".into()
            ));
        }

        Ok(())
    }
}

impl Default for QenusRethConfig {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                data_dir: PathBuf::from("./data/qenus-reth"),
                chain: "mainnet".to_string(),
                identity: "qenus-reth".to_string(),
                max_peers: 50,
                discovery: true,
                http_rpc: Some(HttpRpcConfig {
                    addr: "127.0.0.1".to_string(),
                    port: 8545,
                    modules: vec!["eth".to_string(), "debug".to_string()],
                }),
            },
            database: DatabaseConfig {
                path: PathBuf::from("./data/qenus-reth/db"),
                max_size_gb: 500,
                cache_size_mb: 2048,
                threads: 4,
                compression: true,
            },
            network: NetworkConfig {
                listen_addr: "0.0.0.0".to_string(),
                port: 30303,
                boot_nodes: vec![
                    // Ethereum mainnet boot nodes
                    "enode://d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666@18.138.108.67:30303".to_string(),
                    "enode://22a8232c3abc76a16ae9d6c3b164f98775fe226f0917b0ca871128a74a8e9630b458460865bab457221f1d448dd9791d24c4e5d88786180ac185df813a68d4de@3.209.45.79:30303".to_string(),
                ],
                trusted_peers: vec![],
                max_connections: 50,
                connection_timeout_seconds: 30,
            },
            sync: SyncConfig {
                mode: "full".to_string(),
                max_parallel_downloads: 10,
                download_timeout_seconds: 60,
                checkpoint_sync: true,
                checkpoint_url: Some("https://mainnet-checkpoint-sync.stakely.io".to_string()),
            },
            extraction: ExtractionConfig {
                enabled: true,
                batch_size: 100,
                timeout_seconds: 30,
                amm: AmmExtractionConfig {
                    enabled: true,
                    uniswap_v3_factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string(),
                    curve_registry: "0x90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5".to_string(),
                    balancer_vault: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
                    min_liquidity_usd: 10000.0,
                },
                bridge: BridgeExtractionConfig {
                    enabled: true,
                    canonical_bridges: vec![
                        "0x4Dbd4fc535Ac27206064B68FfCf827b0A60BAB3f".to_string(), // Arbitrum Bridge
                        "0x99C9fc46f92E8a1c0deC1b1747d010903E884bE1".to_string(), // Optimism Bridge
                    ],
                    third_party_bridges: vec![],
                    min_liquidity_usd: 100000.0,
                },
                gas: GasExtractionConfig {
                    enabled: true,
                    calculation_window: 20,
                    percentiles: vec![0.1, 0.25, 0.5, 0.75, 0.9],
                },
                flash_loan: FlashLoanExtractionConfig {
                    enabled: true,
                    aave_v3_pools: vec![
                        "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(), // Aave V3 Pool
                    ],
                    balancer_vault: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
                    min_liquidity_usd: 1000000.0,
                },
            },
            feeds: FeedsConfig {
                kafka: KafkaFeedConfig {
                    enabled: true,
                    bootstrap_servers: vec!["localhost:9092".to_string()],
                    topic_prefix: "qenus.ethereum".to_string(),
                    batch_size: 100,
                    batch_timeout_ms: 1000,
                },
                grpc: GrpcFeedConfig {
                    enabled: true,
                    bind_addr: "0.0.0.0".to_string(),
                    port: 50052,
                    timeout_seconds: 30,
                },
                archive: ArchiveFeedConfig {
                    enabled: true,
                    archive_dir: PathBuf::from("./data/qenus-reth/archive"),
                    compression: "zstd".to_string(),
                    rotation_hours: 1,
                },
            },
            observability: ObservabilityConfig {
                log_level: "info".to_string(),
                metrics_addr: "0.0.0.0".to_string(),
                metrics_port: 9091,
                tracing: TracingConfig {
                    enabled: false,
                    endpoint: None,
                    service_name: "qenus-reth".to_string(),
                },
            },
        }
    }
}

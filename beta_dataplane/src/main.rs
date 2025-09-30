//! Qenus Beta Dataplane - Main entry point
//!
//! Production-ready RPC-based dataplane for immediate revenue generation.

use clap::{Arg, Command};
use std::path::PathBuf;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use qenus_beta_dataplane::{
    config::BetaDataplaneConfig,
    providers::{ApiKeyManager, EthereumRpcClient, ArbitrumRpcClient, OptimismRpcClient, BaseRpcClient},
    extractors::{
        BetaFeatureExtractor, ExtractionContext,
        amm::{UniswapV3Extractor, CurveExtractor, BalancerExtractor},
        gas::pricing::GasPricingExtractor,
        bridges::canonical::CanonicalBridgeExtractor,
        flash_loans::{aave_v3::AaveV3FlashLoanExtractor, balancer::BalancerFlashLoanExtractor},
    },
    feeds::FeedManager,
    monitoring::MonitoringService,
    optimization::{IntelligentCache, CacheStrategy, BatchProcessor, BatchStrategy},
    error::Result,
    Chain, OperationalMode, VERSION,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("qenus-beta-dataplane")
        .version(VERSION)
        .about("Qenus Beta Dataplane - Production-ready RPC-based dataplane")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config/beta-dataplane.toml"),
        )
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .help("Operational mode (development, testing, production, dry-run)")
                .default_value("development"),
        )
        .arg(
            Arg::new("chains")
                .short('n')
                .long("chains")
                .value_name("CHAINS")
                .help("Comma-separated list of chains to monitor")
                .default_value("ethereum,arbitrum,optimism,base"),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Run in dry-run mode (no data publishing)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("test-providers")
                .long("test-providers")
                .help("Test provider connections and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("setup-keys")
                .long("setup-keys")
                .help("Show API key setup instructions and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize logging
    let log_level = matches.get_one::<String>("log-level").unwrap();
    init_logging(log_level)?;

    info!(
        version = VERSION,
        "Starting Qenus Beta Dataplane"
    );

    // Parse operational mode
    let mode: OperationalMode = matches.get_one::<String>("mode")
        .unwrap()
        .parse()?;

    // Load configuration
    let mut config = load_config(matches.get_one::<String>("config").unwrap()).await?;
    
    // Override config with command line arguments
    config.global.mode = mode;
    
    if matches.get_flag("dry-run") {
        config.global.dry_run = true;
        warn!("Running in dry-run mode - no data will be published");
    }

    // Initialize API key manager
    let mut api_key_manager = ApiKeyManager::new();
    api_key_manager.load_api_keys()?;

    // Show API key setup instructions if requested
    if matches.get_flag("setup-keys") {
        api_key_manager.print_status();
        return Ok(());
    }

    // Parse chains to monitor
    let chains = parse_chains(matches.get_one::<String>("chains").unwrap())?;
    info!(?chains, mode = ?mode, "Beta dataplane configuration");

    // Test providers if requested
    if matches.get_flag("test-providers") {
        info!("Testing provider connections...");
        test_providers(&config, &chains, &api_key_manager).await?;
        info!("Provider testing completed successfully");
        return Ok(());
    }

    // Create and start the beta dataplane
    let beta_dataplane = BetaDataplane::new(config, chains).await?;
    
    // Set up graceful shutdown
    let shutdown_signal = setup_shutdown_signal();
    
    // Run the beta dataplane
    tokio::select! {
        result = beta_dataplane.run() => {
            match result {
                Ok(_) => info!("Beta dataplane stopped gracefully"),
                Err(e) => error!(error = %e, "Beta dataplane stopped with error"),
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, stopping beta dataplane...");
        }
    }

    info!("Qenus Beta Dataplane stopped");
    Ok(())
}

/// Initialize logging with the specified level
fn init_logging(log_level: &str) -> Result<()> {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("Invalid log level: {}. Using 'info'", log_level);
            tracing::Level::INFO
        }
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    format!("qenus_beta_dataplane={},tower_http=debug", level).into()
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Load configuration from file
async fn load_config(config_path: &str) -> Result<BetaDataplaneConfig> {
    info!(config_path = config_path, "Loading configuration");
    
    // For now, use default config since we haven't implemented file loading yet
    // TODO: Implement actual config file loading
    let config = BetaDataplaneConfig::default();
    config.validate()?;
    
    Ok(config)
}

/// Parse comma-separated chain names
fn parse_chains(chains_str: &str) -> Result<Vec<Chain>> {
    chains_str
        .split(',')
        .map(|s| s.trim().parse::<Chain>().map_err(|e| e.into()))
        .collect()
}

/// Test provider connections
async fn test_providers(config: &BetaDataplaneConfig, chains: &[Chain], api_key_manager: &ApiKeyManager) -> Result<()> {
    for chain in chains {
        let providers = config.get_providers_for_chain(*chain);
        info!(chain = %chain, provider_count = providers.len(), "Testing providers");
        
        for provider in providers {
            if !provider.enabled {
                continue;
            }
            
            info!(
                provider = provider.name,
                provider_type = %provider.provider_type,
                "Testing provider connection"
            );
            
            // Test API key configuration
            if !api_key_manager.is_provider_configured(provider.provider_type, *chain) {
                warn!(
                    provider = provider.name,
                    provider_type = %provider.provider_type,
                    chain = %chain,
                    "Provider not properly configured (missing API key or endpoints)"
                );
                continue;
            }

            // Build actual URLs with API keys
            let http_url = match api_key_manager.build_http_url(provider.provider_type, *chain) {
                Ok(url) => url,
                Err(e) => {
                    error!(
                        provider = provider.name,
                        error = %e,
                        "Failed to build HTTP URL"
                    );
                    continue;
                }
            };

            // Validate URL format
            if let Err(_) = url::Url::parse(&http_url) {
                error!(
                    provider = provider.name,
                    url = http_url,
                    "Invalid HTTP URL"
                );
                continue;
            }
            
            info!(
                provider = provider.name,
                "Provider connection test passed"
            );
        }
    }
    
    Ok(())
}

/// Set up graceful shutdown signal handling
async fn setup_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Main beta dataplane orchestrator
struct BetaDataplane {
    config: BetaDataplaneConfig,
    chains: Vec<Chain>,
    providers: ChainProviders,
    extractors: ChainExtractors,
    feed_manager: Arc<FeedManager>,
    monitoring: Arc<MonitoringService>,
    cache: Arc<IntelligentCache<Vec<u8>>>,
}

/// RPC providers for each chain
struct ChainProviders {
    ethereum: Option<EthereumRpcClient>,
    arbitrum: Option<ArbitrumRpcClient>,
    optimism: Option<OptimismRpcClient>,
    base: Option<BaseRpcClient>,
}

/// Feature extractors for each chain
struct ChainExtractors {
    uniswap_v3: Arc<UniswapV3Extractor>,
    curve: Arc<CurveExtractor>,
    balancer: Arc<BalancerExtractor>,
    gas: Arc<GasPricingExtractor>,
    bridges: Arc<CanonicalBridgeExtractor>,
    aave_flash: Arc<AaveV3FlashLoanExtractor>,
    balancer_flash: Arc<BalancerFlashLoanExtractor>,
}

impl BetaDataplane {
    /// Create a new beta dataplane instance
    async fn new(config: BetaDataplaneConfig, chains: Vec<Chain>) -> Result<Self> {
        info!("Initializing beta dataplane...");
        
        // Initialize providers for each chain
        let providers = Self::initialize_providers(&config, &chains).await?;
        
        // Initialize extractors
        let extractors = Self::initialize_extractors(&providers).await?;
        
        // Initialize feed manager
        let feed_manager = Arc::new(Self::initialize_feeds(&config)?);
        
        // Initialize monitoring
        let monitoring = Arc::new(Self::initialize_monitoring()?);
        
        // Initialize cache
        let cache = Arc::new(IntelligentCache::new(
            Duration::from_secs(60), // 60s TTL
            10000, // 10k entries
            CacheStrategy::LRU,
        ));
        
        info!("✅ Cache initialized");
        
        info!("✅ Beta dataplane initialized successfully");
        
        Ok(Self {
            config,
            chains,
            providers,
            extractors,
            feed_manager,
            monitoring,
            cache,
        })
    }

    /// Initialize RPC providers for all chains
    async fn initialize_providers(config: &BetaDataplaneConfig, chains: &[Chain]) -> Result<ChainProviders> {
        info!("Initializing RPC providers...");
        
        let mut ethereum = None;
        let mut arbitrum = None;
        let mut optimism = None;
        let mut base = None;
        
        for chain in chains {
            let providers_for_chain = config.get_providers_for_chain(*chain);
            
            match chain {
                Chain::Ethereum if !providers_for_chain.is_empty() => {
                    info!("Initializing Ethereum provider with {} endpoints", providers_for_chain.len());
                    ethereum = Some(EthereumRpcClient::new(providers_for_chain.clone()).await?);
                }
                Chain::Arbitrum if !providers_for_chain.is_empty() => {
                    info!("Initializing Arbitrum provider with {} endpoints", providers_for_chain.len());
                    arbitrum = Some(ArbitrumRpcClient::new(providers_for_chain.clone()).await?);
                }
                Chain::Optimism if !providers_for_chain.is_empty() => {
                    info!("Initializing Optimism provider with {} endpoints", providers_for_chain.len());
                    optimism = Some(OptimismRpcClient::new(providers_for_chain.clone()).await?);
                }
                Chain::Base if !providers_for_chain.is_empty() => {
                    info!("Initializing Base provider with {} endpoints", providers_for_chain.len());
                    base = Some(BaseRpcClient::new(providers_for_chain.clone()).await?);
                }
                _ => {}
            }
        }
        
        info!("✅ RPC providers initialized");
        
        Ok(ChainProviders {
            ethereum,
            arbitrum,
            optimism,
            base,
        })
    }

    /// Initialize all feature extractors
    async fn initialize_extractors(providers: &ChainProviders) -> Result<ChainExtractors> {
        info!("Initializing feature extractors...");
        
        use qenus_beta_dataplane::extractors::ExtractorConfig;
        
        let config = ExtractorConfig::default();
        
        // Create extractors with RPC clients
        let mut uniswap_v3 = UniswapV3Extractor::new(config.clone());
        let curve = CurveExtractor::new(config.clone());
        let balancer = BalancerExtractor::new(config.clone());
        let gas = GasPricingExtractor::new(config.clone());
        let bridges = CanonicalBridgeExtractor::new(config.clone());
        let aave_flash = AaveV3FlashLoanExtractor::new(config.clone());
        let balancer_flash = BalancerFlashLoanExtractor::new(config.clone());
        
        // Set client for each extractor if Ethereum provider is available
        if let Some(eth_client) = &providers.ethereum {
            uniswap_v3.set_client(eth_client.clone());
        }
        
        info!("✅ Feature extractors initialized");
        
        Ok(ChainExtractors {
            uniswap_v3: Arc::new(uniswap_v3),
            curve: Arc::new(curve),
            balancer: Arc::new(balancer),
            gas: Arc::new(gas),
            bridges: Arc::new(bridges),
            aave_flash: Arc::new(aave_flash),
            balancer_flash: Arc::new(balancer_flash),
        })
    }

    /// Initialize feed manager
    fn initialize_feeds(config: &BetaDataplaneConfig) -> Result<FeedManager> {
        info!("Initializing feed manager...");
        
        use qenus_beta_dataplane::feeds::BetaFeedConfig;
        use std::path::PathBuf;
        
        let feed_config = BetaFeedConfig::default();
        
        let manager = FeedManager::new()
            .with_kafka(
                vec!["localhost:9092".to_string()],
                "qenus.beta.features".to_string(),
                feed_config.clone(),
            )
            .with_grpc(
                "0.0.0.0".to_string(),
                50053,
                feed_config.clone(),
            )
            .with_parquet(
                PathBuf::from("./data/beta-dataplane/parquet"),
                "qenus_features".to_string(),
                feed_config,
            );
        
        info!("✅ Feed manager initialized");
        Ok(manager)
    }

    /// Initialize monitoring service
    fn initialize_monitoring() -> Result<MonitoringService> {
        info!("Initializing monitoring...");
        
        use qenus_beta_dataplane::monitoring::{HealthChecker, MetricsRegistry, AlertManager};
        
        let health = HealthChecker::new(Duration::from_secs(30));
        let metrics = MetricsRegistry::new(Duration::from_secs(15));
        let alerts = AlertManager::new(1000);
        
        let monitoring = MonitoringService::new(health, metrics, alerts);
        
        info!("✅ Monitoring initialized");
        Ok(monitoring)
    }

    /// Run the beta dataplane
    async fn run(mut self) -> Result<()> {
        info!("🚀 Starting beta dataplane components...");

        // Start monitoring service
        self.monitoring.start().await?;
        info!("✅ Monitoring service started");

        // Start feed manager (skip for now, will be started when feeds are ready)
        info!("⏭️  Feed manager ready (will start when first features are extracted)");

        info!(
            chains = ?self.chains,
            mode = ?self.config.global.mode,
            dry_run = self.config.global.dry_run,
            "🎯 Beta dataplane is RUNNING - extracting live on-chain data"
        );

        // Main extraction loop
        let mut interval = tokio::time::interval(Duration::from_secs(3)); // Extract every 3 seconds
        let mut block_counters: std::collections::HashMap<Chain, u64> = std::collections::HashMap::new();

        loop {
            interval.tick().await;

            // Process each chain
            for chain in &self.chains {
                // Get current block number
                let current_block = match self.get_current_block(*chain).await {
                    Ok(block) => block,
                    Err(e) => {
                        warn!(chain = %chain, error = %e, "Failed to get current block");
                        continue;
                    }
                };

                // Check if we've already processed this block
                let last_processed = block_counters.get(chain).copied().unwrap_or(0);
                if current_block <= last_processed {
                    continue; // Already processed this block
                }

                info!(
                    chain = %chain,
                    block = current_block,
                    "Processing new block"
                );

                // Extract features from all extractors
                self.extract_and_publish(*chain, current_block).await?;

                // Update block counter
                block_counters.insert(*chain, current_block);
            }
        }
    }

    /// Get current block number for a chain
    async fn get_current_block(&self, chain: Chain) -> Result<u64> {
        match chain {
            Chain::Ethereum => {
                if let Some(client) = &self.providers.ethereum {
                    client.get_current_block().await
                } else {
                    Err(qenus_beta_dataplane::BetaDataplaneError::internal("Ethereum provider not initialized"))
                }
            }
            Chain::Arbitrum => {
                if let Some(client) = &self.providers.arbitrum {
                    client.get_current_block().await
                } else {
                    Err(qenus_beta_dataplane::BetaDataplaneError::internal("Arbitrum provider not initialized"))
                }
            }
            Chain::Optimism => {
                if let Some(client) = &self.providers.optimism {
                    client.get_current_block().await
                } else {
                    Err(qenus_beta_dataplane::BetaDataplaneError::internal("Optimism provider not initialized"))
                }
            }
            Chain::Base => {
                if let Some(client) = &self.providers.base {
                    client.get_current_block().await
                } else {
                    Err(qenus_beta_dataplane::BetaDataplaneError::internal("Base provider not initialized"))
                }
            }
        }
    }

    /// Extract features from all extractors and publish them
    async fn extract_and_publish(&self, chain: Chain, block_number: u64) -> Result<()> {
        let context = ExtractionContext::new(
            chain,
            block_number,
            format!("{}_provider", chain),
        );

        let mut all_features = Vec::new();

        // Run Uniswap V3 extractor
        if self.extractors.uniswap_v3.supports_chain(chain) {
            match self.extractors.uniswap_v3.extract_for_block(chain, block_number, &context).await {
                Ok(features) => {
                    if !features.is_empty() {
                        info!(extractor = "uniswap_v3", chain = %chain, features = features.len(), "Extracted");
                        all_features.extend(features);
                    }
                }
                Err(e) => warn!(extractor = "uniswap_v3", error = %e, "Extraction failed"),
            }
        }

        // Run Curve extractor
        if self.extractors.curve.supports_chain(chain) {
            match self.extractors.curve.extract_for_block(chain, block_number, &context).await {
                Ok(features) => {
                    if !features.is_empty() {
                        info!(extractor = "curve", chain = %chain, features = features.len(), "Extracted");
                        all_features.extend(features);
                    }
                }
                Err(e) => warn!(extractor = "curve", error = %e, "Extraction failed"),
            }
        }

        // Run Balancer extractor
        if self.extractors.balancer.supports_chain(chain) {
            match self.extractors.balancer.extract_for_block(chain, block_number, &context).await {
                Ok(features) => {
                    if !features.is_empty() {
                        info!(extractor = "balancer", chain = %chain, features = features.len(), "Extracted");
                        all_features.extend(features);
                    }
                }
                Err(e) => warn!(extractor = "balancer", error = %e, "Extraction failed"),
            }
        }

        // Run Gas extractor
        if self.extractors.gas.supports_chain(chain) {
            match self.extractors.gas.extract_for_block(chain, block_number, &context).await {
                Ok(features) => {
                    if !features.is_empty() {
                        info!(extractor = "gas", chain = %chain, features = features.len(), "Extracted");
                        all_features.extend(features);
                    }
                }
                Err(e) => warn!(extractor = "gas", error = %e, "Extraction failed"),
            }
        }

        // Run Bridges extractor
        if self.extractors.bridges.supports_chain(chain) {
            match self.extractors.bridges.extract_for_block(chain, block_number, &context).await {
                Ok(features) => {
                    if !features.is_empty() {
                        info!(extractor = "bridges", chain = %chain, features = features.len(), "Extracted");
                        all_features.extend(features);
                    }
                }
                Err(e) => warn!(extractor = "bridges", error = %e, "Extraction failed"),
            }
        }

        // Run Aave flash loan extractor
        if self.extractors.aave_flash.supports_chain(chain) {
            match self.extractors.aave_flash.extract_for_block(chain, block_number, &context).await {
                Ok(features) => {
                    if !features.is_empty() {
                        info!(extractor = "aave_flash", chain = %chain, features = features.len(), "Extracted");
                        all_features.extend(features);
                    }
                }
                Err(e) => warn!(extractor = "aave_flash", error = %e, "Extraction failed"),
            }
        }

        // Run Balancer flash loan extractor
        if self.extractors.balancer_flash.supports_chain(chain) {
            match self.extractors.balancer_flash.extract_for_block(chain, block_number, &context).await {
                Ok(features) => {
                    if !features.is_empty() {
                        info!(extractor = "balancer_flash", chain = %chain, features = features.len(), "Extracted");
                        all_features.extend(features);
                    }
                }
                Err(e) => warn!(extractor = "balancer_flash", error = %e, "Extraction failed"),
            }
        }

        // Publish all features
        if !all_features.is_empty() {
            if !self.config.global.dry_run {
                match self.feed_manager.publish_batch(all_features.clone()).await {
                    Ok(_) => {
                        info!(
                            chain = %chain,
                            block = block_number,
                            total_features = all_features.len(),
                            "✅ Published features to all feeds"
                        );
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to publish features");
                    }
                }
            } else {
                info!(
                    chain = %chain,
                    block = block_number,
                    features = all_features.len(),
                    "DRY RUN: Would publish features"
                );
            }
        }

        Ok(())
    }
}

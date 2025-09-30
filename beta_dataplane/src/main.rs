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
    providers::ApiKeyManager,
    error::Result,
    Chain, OperationalMode, VERSION,
};

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
}

impl BetaDataplane {
    /// Create a new beta dataplane instance
    async fn new(config: BetaDataplaneConfig, chains: Vec<Chain>) -> Result<Self> {
        info!("Initializing beta dataplane");
        
        Ok(Self {
            config,
            chains,
        })
    }

    /// Run the beta dataplane
    async fn run(self) -> Result<()> {
        info!("Starting beta dataplane components...");

        // TODO: Initialize and start all components in subsequent phases:
        // Phase 2: Provider management
        // Phase 3: Feature extractors  
        // Phase 4: Optimization systems
        // Phase 5: Data feeds
        // Phase 6: Monitoring

        info!(
            chains = ?self.chains,
            mode = ?self.config.global.mode,
            dry_run = self.config.global.dry_run,
            "Beta dataplane is running"
        );

        // Keep running until shutdown
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            
            // TODO: Add actual processing logic here
            // This is where we'll orchestrate:
            // - Provider management
            // - Feature extraction
            // - Data publishing
            // - Monitoring
        }
    }
}

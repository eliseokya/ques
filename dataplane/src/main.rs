//! Qenus Dataplane - Main entry point
//!
//! The sensory nervous system of Qenus, responsible for real-time blockchain
//! data ingestion and feature extraction.

use clap::{Arg, Command};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use qenus_dataplane::{
    config::DataplaneConfig,
    error::Result,
    Chain,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("qenus-dataplane")
        .version(qenus_dataplane::VERSION)
        .about("Qenus Dataplane - Real-time blockchain data ingestion")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config/dataplane.toml"),
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
        .get_matches();

    // Initialize logging
    let log_level = matches.get_one::<String>("log-level").unwrap();
    init_logging(log_level)?;

    info!(
        version = qenus_dataplane::VERSION,
        "Starting Qenus Dataplane"
    );

    // Load configuration
    let config = load_config(matches.get_one::<String>("config").unwrap()).await?;
    info!("Configuration loaded successfully");

    // Parse chains to monitor
    let chains = parse_chains(matches.get_one::<String>("chains").unwrap())?;
    info!(?chains, "Monitoring chains");

    // Check dry-run mode
    let dry_run = matches.get_flag("dry-run");
    if dry_run {
        warn!("Running in dry-run mode - no data will be published");
    }

    // Start the dataplane
    let dataplane = Dataplane::new(config, chains, dry_run).await?;
    
    // Set up graceful shutdown
    let shutdown_signal = setup_shutdown_signal();
    
    // Run the dataplane
    tokio::select! {
        result = dataplane.run() => {
            match result {
                Ok(_) => info!("Dataplane stopped gracefully"),
                Err(e) => error!(error = %e, "Dataplane stopped with error"),
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, stopping dataplane...");
        }
    }

    info!("Qenus Dataplane stopped");
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
                    format!("qenus_dataplane={},tower_http=debug", level).into()
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Load configuration from file
async fn load_config(config_path: &str) -> Result<DataplaneConfig> {
    info!(config_path = config_path, "Loading configuration");
    
    // For now, use default config since we haven't implemented file loading yet
    // TODO: Implement actual config file loading
    let config = DataplaneConfig::default();
    config.validate()?;
    
    Ok(config)
}

/// Parse comma-separated chain names
fn parse_chains(chains_str: &str) -> Result<Vec<Chain>> {
    chains_str
        .split(',')
        .map(|s| s.trim().parse::<Chain>())
        .collect()
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

/// Main dataplane orchestrator
struct Dataplane {
    config: Arc<DataplaneConfig>,
    chains: Vec<Chain>,
    dry_run: bool,
}

impl Dataplane {
    /// Create a new dataplane instance
    async fn new(config: DataplaneConfig, chains: Vec<Chain>, dry_run: bool) -> Result<Self> {
        Ok(Self {
            config: Arc::new(config),
            chains,
            dry_run,
        })
    }

    /// Run the dataplane
    async fn run(self) -> Result<()> {
        info!("Starting dataplane components...");

        // TODO: Initialize and start all components:
        // 1. Observers for each chain
        // 2. Feature extractors
        // 3. Data feeds (Kafka, gRPC, Parquet)
        // 4. Health checks and metrics
        // 5. Monitoring and alerting

        // For now, just log that we're running
        info!(
            chains = ?self.chains,
            dry_run = self.dry_run,
            "Dataplane is running"
        );

        // Keep running until shutdown
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            
            // TODO: Add actual processing logic here
            // This is where we'll orchestrate:
            // - Chain observers
            // - Feature extraction
            // - Data publishing
        }
    }
}

//! Qenus Reth - Main entry point
//!
//! A lean Ethereum L1 client optimized for real-time feature extraction.

use clap::{Arg, Command};
use std::path::PathBuf;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use qenus_reth::{
    config::QenusRethConfig,
    node::QenusRethNode,
    error::Result,
    validate_ethereum_config,
    VERSION,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("qenus-reth")
        .version(VERSION)
        .about("Qenus Reth - Lean Ethereum L1 client for feature extraction")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config/qenus-reth.toml"),
        )
        .arg(
            Arg::new("data-dir")
                .short('d')
                .long("data-dir")
                .value_name("DIR")
                .help("Data directory path")
                .default_value("./data/qenus-reth"),
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
            Arg::new("sync-mode")
                .long("sync-mode")
                .value_name("MODE")
                .help("Sync mode (full, fast, snap)")
                .default_value("full"),
        )
        .arg(
            Arg::new("extraction")
                .long("extraction")
                .help("Enable feature extraction")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize logging
    let log_level = matches.get_one::<String>("log-level").unwrap();
    init_logging(log_level)?;

    info!(
        version = VERSION,
        "Starting Qenus Reth - Lean Ethereum L1 client"
    );

    // Validate Ethereum configuration
    validate_ethereum_config()?;

    // Load configuration
    let mut config = load_config(matches.get_one::<String>("config").unwrap()).await?;
    
    // Override config with command line arguments
    if let Some(data_dir) = matches.get_one::<String>("data-dir") {
        config.node.data_dir = PathBuf::from(data_dir);
    }
    
    if let Some(sync_mode) = matches.get_one::<String>("sync-mode") {
        config.sync.mode = sync_mode.clone();
    }
    
    if matches.get_flag("extraction") {
        config.extraction.enabled = true;
    }

    info!("Configuration loaded successfully");
    info!("Data directory: {:?}", config.node.data_dir);
    info!("Sync mode: {}", config.sync.mode);
    info!("Feature extraction: {}", config.extraction.enabled);

    // Create and start the node
    let mut node = QenusRethNode::new(config)?;
    
    // Set up graceful shutdown
    let shutdown_signal = setup_shutdown_signal();
    
    // Run the node
    tokio::select! {
        result = node.run() => {
            match result {
                Ok(_) => info!("Qenus Reth node stopped gracefully"),
                Err(e) => error!("Qenus Reth node stopped with error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received, stopping node...");
            if let Err(e) = node.stop().await {
                error!("Error during node shutdown: {}", e);
            }
        }
    }

    info!("Qenus Reth stopped");
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
                    format!("qenus_reth={},reth=info,tower_http=debug", level).into()
                })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Load configuration from file
async fn load_config(config_path: &str) -> Result<QenusRethConfig> {
    info!(config_path = config_path, "Loading configuration");
    
    // For now, use default config since we haven't implemented file loading yet
    // TODO: Implement actual config file loading
    let config = QenusRethConfig::default();
    config.validate()?;
    
    Ok(config)
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

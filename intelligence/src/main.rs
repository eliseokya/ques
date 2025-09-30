//! Qenus Intelligence Layer - Main entry point
//!
//! Consumes dataplane features and generates trade intents.

use clap::{Arg, Command};
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use qenus_intelligence::{Result, VERSION};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("qenus-intelligence")
        .version(VERSION)
        .about("Qenus Intelligence Layer - Arbitrage opportunity detection")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config/intelligence.toml"),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info"),
        )
        .get_matches();

    // Initialize logging
    let log_level = matches.get_one::<String>("log-level").unwrap();
    init_logging(log_level)?;

    info!(
        version = VERSION,
        "Starting Qenus Intelligence Layer"
    );

    // TODO: Load configuration
    // TODO: Initialize market state
    // TODO: Start detector engines
    // TODO: Connect to dataplane feeds
    // TODO: Start main processing loop

    info!("Intelligence layer initialized");

    // Set up graceful shutdown
    let shutdown_signal = setup_shutdown_signal();

    tokio::select! {
        _ = run_intelligence() => {
            info!("Intelligence layer stopped");
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received");
        }
    }

    info!("Qenus Intelligence Layer stopped");
    Ok(())
}

/// Initialize logging
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
                .unwrap_or_else(|_| format!("qenus_intelligence={}", level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Main intelligence processing loop
async fn run_intelligence() -> Result<()> {
    info!("Intelligence layer running...");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        // TODO: Process dataplane features
        // TODO: Detect opportunities
        // TODO: Generate trade intents
    }
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


//! Qenus Intelligence Layer - Main entry point
//!
//! Consumes beta_dataplane features and generates trade intents.

use std::sync::Arc;
use clap::{Arg, Command};
use tokio::signal;
use tracing::{error, info, warn, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use qenus_intelligence::{
    Result, VERSION, IntelligenceConfig, MarketState, DetectorManager, FeatureIngestionManager,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("qenus-intelligence")
        .version(VERSION)
        .about("Qenus Intelligence Layer - The Brain")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path"),
        )
        .arg(
            Arg::new("business-path")
                .short('b')
                .long("business-path")
                .value_name("PATH")
                .help("Path to business module (for loading strategies)"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Run in dry-run mode (no actual intent emission)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generate-config")
                .long("generate-config")
                .value_name("OUTPUT")
                .help("Generate example config and exit"),
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

    // Handle config generation
    if let Some(output_path) = matches.get_one::<String>("generate-config") {
        let config = IntelligenceConfig::default();
        config.save_to_file(output_path)?;
        info!("Generated example config at: {}", output_path);
        return Ok(());
    }

    info!(version = VERSION, "🧠 Qenus Intelligence Layer starting...");

    // Load configuration
    let config = if let Some(config_path) = matches.get_one::<String>("config") {
        info!("Loading config from: {}", config_path);
        IntelligenceConfig::from_file(config_path)?
    } else {
        info!("Loading config from business module or defaults");
        let business_path = matches.get_one::<String>("business-path").map(|s| s.as_str());
        IntelligenceConfig::from_business_module_or_default(business_path)
    };

    let dry_run = matches.get_flag("dry-run");
    if dry_run {
        warn!("🔶 Running in DRY-RUN mode - no intents will be emitted");
    }

    // Initialize market state
    info!("Initializing market state (TTL: {}s)", config.market_state_ttl_secs);
    let market_state = Arc::new(MarketState::new(config.market_state_ttl_secs));

    // Initialize detectors
    info!("Initializing detectors...");
    let enabled_strategies = config.enabled_strategies();
    info!(
        "Enabled strategies: {}",
        enabled_strategies.iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let detector_manager = DetectorManager::new(
        config.get_strategy("triangle_arb").cloned(),
        config.get_strategy("dex_arb").cloned(),
        market_state.clone(),
    );

    // Initialize feature ingestion
    info!("Initializing feature ingestion (mode: {})", config.dataplane.mode);
    let mut ingestion_manager = FeatureIngestionManager::new(market_state.clone());

    // Start ingestion based on mode
    let dataplane_mode = config.dataplane.mode.clone();
    match dataplane_mode.as_str() {
        "kafka" => {
            if let Some(brokers) = &config.dataplane.kafka_brokers {
                info!("Starting Kafka ingestion from: {}", brokers);
                info!("Subscribing to topics: {:?}", config.dataplane.kafka_topics);
                ingestion_manager.start_kafka_ingestion(brokers, config.dataplane.kafka_topics.clone()).await?;
            } else {
                error!("Kafka mode selected but no brokers configured");
                return Err(qenus_intelligence::IntelligenceError::Internal(
                    "Kafka brokers not configured".to_string()
                ));
            }
        }
        "grpc" => {
            if let Some(endpoint) = &config.dataplane.grpc_endpoint {
                info!("Starting gRPC ingestion from: {}", endpoint);
                ingestion_manager.start_grpc_ingestion(endpoint).await?;
            } else {
                error!("gRPC mode selected but no endpoint configured");
                return Err(qenus_intelligence::IntelligenceError::Internal(
                    "gRPC endpoint not configured".to_string()
                ));
            }
        }
        "mock" => {
            warn!("🔶 Using MOCK ingestion for development");
            info!("Mock mode: Waiting for manual feature injection or tests");
        }
        _ => {
            error!("Unknown dataplane mode: {}", config.dataplane.mode);
            return Err(qenus_intelligence::IntelligenceError::Internal(
                format!("Unknown dataplane mode: {}", config.dataplane.mode)
            ));
        }
    }

    info!("🚀 Intelligence layer ready!");
    info!("Detection interval: {}s", config.detection.interval_secs);

    // Set up graceful shutdown
    let shutdown_signal = setup_shutdown_signal();

    tokio::select! {
        _ = run_detection_loop(detector_manager, market_state, config, dry_run) => {
            info!("Detection loop stopped");
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

/// Main detection loop
async fn run_detection_loop(
    detector_manager: DetectorManager,
    market_state: Arc<MarketState>,
    config: IntelligenceConfig,
    dry_run: bool,
) -> Result<()> {
    let mut interval = tokio::time::interval(
        tokio::time::Duration::from_secs(config.detection.interval_secs)
    );

    loop {
        interval.tick().await;

        // Run detection
        match detector_manager.detect_all().await {
            Ok(candidates) => {
                if !candidates.is_empty() {
                    info!("💡 Detected {} candidates", candidates.len());

                    for candidate in candidates.iter().take(config.detection.max_candidates_per_cycle) {
                        if candidate.confidence >= config.detection.min_confidence {
                            info!(
                                "  ✅ {} on {}: spread={:.2}bps, confidence={:.2}",
                                candidate.strategy,
                                candidate.asset,
                                candidate.spread_bps,
                                candidate.confidence
                            );

                            if !dry_run {
                                // TODO: Phase 2-4 - Simulate, decide, build intent
                                debug!("    Would simulate and emit intent");
                            }
                        }
                    }
                } else {
                    // Check if feeds are stale
                    let stats = market_state.get_stats().await;
                    if stats.total_amm_pools == 0 {
                        debug!("⏳ No candidates (market state empty)");
                    } else {
                        debug!("⏳ No candidates detected");
                    }
                }
            }
            Err(e) => {
                error!("Detection failed: {}", e);
            }
        }
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

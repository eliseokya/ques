//! Configuration management for Intelligence layer
//!
//! Loads strategy configs from business module YAMLs or uses defaults

use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use qenus_dataplane::Chain;

use crate::error::{IntelligenceError, Result};
use crate::types::{StrategyConfig, RiskLimits};

/// Intelligence layer configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntelligenceConfig {
    /// Strategy configurations
    pub strategies: HashMap<String, StrategyConfig>,
    
    /// Market state TTL in seconds
    pub market_state_ttl_secs: i64,
    
    /// Beta dataplane connection settings
    pub dataplane: DataplaneConnectionConfig,
    
    /// Detection settings
    pub detection: DetectionConfig,
}

/// Beta dataplane connection configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataplaneConnectionConfig {
    /// Kafka brokers (comma-separated)
    pub kafka_brokers: Option<String>,
    
    /// Kafka topics to subscribe to
    pub kafka_topics: Vec<String>,
    
    /// gRPC endpoint
    pub grpc_endpoint: Option<String>,
    
    /// Connection mode: "kafka", "grpc", or "mock"
    pub mode: String,
}

/// Detection configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DetectionConfig {
    /// How often to run detection (seconds)
    pub interval_secs: u64,
    
    /// Maximum candidates to emit per cycle
    pub max_candidates_per_cycle: usize,
    
    /// Minimum confidence threshold
    pub min_confidence: f64,
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            strategies: Self::default_strategies(),
            market_state_ttl_secs: 30,
            dataplane: DataplaneConnectionConfig {
                kafka_brokers: Some("localhost:9092".to_string()),
                kafka_topics: vec![
                    "qenus.beta.features.amm".to_string(),
                    "qenus.beta.features.gas".to_string(),
                    "qenus.beta.features.bridge".to_string(),
                    "qenus.beta.features.flashloan".to_string(),
                    "qenus.beta.features.sequencer".to_string(),
                ],
                grpc_endpoint: Some("http://localhost:50053".to_string()),
                mode: "mock".to_string(), // Default to mock for development
            },
            detection: DetectionConfig {
                interval_secs: 5,
                max_candidates_per_cycle: 100,
                min_confidence: 0.7,
            },
        }
    }
}

impl IntelligenceConfig {
    /// Load configuration from YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| IntelligenceError::Config(
                config::ConfigError::Foreign(Box::new(e))
            ))?;
        
        let config: Self = serde_yaml::from_str(&content)
            .map_err(|e| IntelligenceError::Config(
                config::ConfigError::Foreign(Box::new(e))
            ))?;
        
        Ok(config)
    }
    
    /// Try to load from business module, fall back to defaults
    pub fn from_business_module_or_default(business_path: Option<&str>) -> Self {
        if let Some(path) = business_path {
            // Try to load from business module
            let strategies_path = format!("{}/strategies", path);
            if Path::new(&strategies_path).exists() {
                tracing::info!("Loading strategies from business module: {}", strategies_path);
                // TODO: Load all YAML files from strategies directory
                // For now, return default
            } else {
                tracing::warn!("Business module path provided but not found: {}", path);
            }
        }
        
        tracing::info!("Using default strategy configurations");
        Self::default()
    }
    
    /// Load configuration from environment and files
    pub fn from_env_and_file() -> Result<Self> {
        // Check for config file path in environment
        if let Ok(config_path) = std::env::var("INTELLIGENCE_CONFIG_PATH") {
            tracing::info!("Loading intelligence config from: {}", config_path);
            return Self::from_file(config_path);
        }
        
        // Check for business module path
        let business_path = std::env::var("BUSINESS_MODULE_PATH").ok();
        Ok(Self::from_business_module_or_default(business_path.as_deref()))
    }
    
    /// Get default strategies (hardcoded for now, will be replaced by business module)
    fn default_strategies() -> HashMap<String, StrategyConfig> {
        let mut strategies = HashMap::new();
        
        // Triangle arbitrage strategy
        strategies.insert(
            "triangle_arb".to_string(),
            StrategyConfig {
                name: "triangle_arb".to_string(),
                enabled: true,
                min_profit_usd: 500.0,
                min_profit_bps: 10.0, // 0.1% minimum spread
                max_position_usd: 5_000_000.0,
                approved_assets: vec![
                    "WETH".to_string(),
                    "USDC".to_string(),
                    "USDT".to_string(),
                    "WBTC".to_string(),
                ],
                approved_chains: vec![
                    Chain::Ethereum,
                    Chain::Arbitrum,
                    Chain::Optimism,
                    Chain::Base,
                ],
                risk_limits: RiskLimits {
                    max_slippage_bps: 100.0, // 1%
                    max_gas_pct: 50.0, // Gas can be up to 50% of profit
                    max_bridge_latency_secs: 300, // 5 minutes
                    min_success_prob: 0.8,
                },
            },
        );
        
        // DEX arbitrage strategy
        strategies.insert(
            "dex_arb".to_string(),
            StrategyConfig {
                name: "dex_arb".to_string(),
                enabled: true,
                min_profit_usd: 200.0, // Lower minimum for same-chain
                min_profit_bps: 5.0, // 0.05% minimum spread
                max_position_usd: 2_000_000.0,
                approved_assets: vec![
                    "WETH".to_string(),
                    "USDC".to_string(),
                    "USDT".to_string(),
                    "DAI".to_string(),
                    "WBTC".to_string(),
                    "stETH".to_string(),
                ],
                approved_chains: vec![
                    Chain::Ethereum,
                    Chain::Arbitrum,
                    Chain::Optimism,
                    Chain::Base,
                ],
                risk_limits: RiskLimits {
                    max_slippage_bps: 50.0, // 0.5%
                    max_gas_pct: 30.0,
                    max_bridge_latency_secs: 0, // No bridge for same-chain
                    min_success_prob: 0.85, // Higher confidence for same-chain
                },
            },
        );
        
        strategies
    }
    
    /// Get a strategy by name
    pub fn get_strategy(&self, name: &str) -> Option<&StrategyConfig> {
        self.strategies.get(name)
    }
    
    /// Get all enabled strategies
    pub fn enabled_strategies(&self) -> Vec<&StrategyConfig> {
        self.strategies
            .values()
            .filter(|s| s.enabled)
            .collect()
    }
    
    /// Save configuration to YAML file (for generating examples)
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = serde_yaml::to_string(self)
            .map_err(|e| IntelligenceError::Internal(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, yaml)
            .map_err(|e| IntelligenceError::Io(e))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = IntelligenceConfig::default();
        
        // Should have default strategies
        assert!(config.strategies.contains_key("triangle_arb"));
        assert!(config.strategies.contains_key("dex_arb"));
        
        // Triangle arb should be enabled
        let triangle = config.get_strategy("triangle_arb").unwrap();
        assert!(triangle.enabled);
        assert_eq!(triangle.min_profit_bps, 10.0);
        assert_eq!(triangle.approved_assets.len(), 4);
        assert_eq!(triangle.approved_chains.len(), 4);
    }
    
    #[test]
    fn test_enabled_strategies() {
        let config = IntelligenceConfig::default();
        let enabled = config.enabled_strategies();
        
        // Both default strategies should be enabled
        assert_eq!(enabled.len(), 2);
    }
    
    #[test]
    fn test_save_and_load_config() {
        let config = IntelligenceConfig::default();
        let temp_path = "/tmp/test_intelligence_config.yaml";
        
        // Save
        config.save_to_file(temp_path).unwrap();
        
        // Load
        let loaded = IntelligenceConfig::from_file(temp_path).unwrap();
        
        // Verify
        assert_eq!(loaded.strategies.len(), config.strategies.len());
        assert_eq!(loaded.market_state_ttl_secs, config.market_state_ttl_secs);
        
        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }
}

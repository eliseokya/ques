//! Configuration for Intelligence layer

use serde::{Deserialize, Serialize};
use crate::{StrategyConfig, Result};

/// Intelligence layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig {
    /// Enabled strategies
    pub strategies: Vec<StrategyConfig>,
    
    /// Kafka configuration
    pub kafka: KafkaConfig,
    
    /// State management configuration
    pub state: StateConfig,
}

/// Kafka consumer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub brokers: Vec<String>,
    pub group_id: String,
    pub topics: Vec<String>,
}

/// State management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    pub redis_url: String,
    pub ttl_seconds: u64,
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            strategies: Vec::new(),
            kafka: KafkaConfig {
                brokers: vec!["localhost:9092".to_string()],
                group_id: "qenus-intelligence".to_string(),
                topics: vec!["qenus.beta.features".to_string()],
            },
            state: StateConfig {
                redis_url: "redis://localhost:6379".to_string(),
                ttl_seconds: 300,
            },
        }
    }
}


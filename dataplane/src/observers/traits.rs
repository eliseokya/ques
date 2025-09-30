//! Traits and common types for chain observers

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::{types::{Block, Transaction, Log}, Chain, Result};

/// Trait for chain observers that monitor blockchain data
#[async_trait]
pub trait ChainObserver: Send + Sync {
    /// Get the chain this observer monitors
    fn chain(&self) -> Chain;

    /// Start observing the chain
    async fn start(&mut self) -> Result<()>;

    /// Stop observing the chain
    async fn stop(&mut self) -> Result<()>;

    /// Check if the observer is running
    fn is_running(&self) -> bool;

    /// Get the current block number
    async fn current_block_number(&self) -> Result<u64>;

    /// Get a specific block by number
    async fn get_block(&self, block_number: u64) -> Result<Block>;

    /// Get recent blocks (up to limit)
    async fn get_recent_blocks(&self, limit: usize) -> Result<Vec<Block>>;

    /// Subscribe to new blocks
    async fn subscribe_blocks(&self) -> Result<mpsc::Receiver<Block>>;

    /// Subscribe to new transactions
    async fn subscribe_transactions(&self) -> Result<mpsc::Receiver<Transaction>>;

    /// Subscribe to logs/events matching filters
    async fn subscribe_logs(&self, filters: Vec<LogFilter>) -> Result<mpsc::Receiver<Log>>;

    /// Get observer metrics
    fn metrics(&self) -> ObserverMetrics;

    /// Get observer health status
    fn health(&self) -> ObserverHealth;
}

/// Event emitted by observers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObserverEvent {
    NewBlock {
        chain: Chain,
        block: Block,
    },
    NewTransaction {
        chain: Chain,
        transaction: Transaction,
    },
    NewLog {
        chain: Chain,
        log: Log,
    },
    Reorg {
        chain: Chain,
        old_block: u64,
        new_block: u64,
    },
    ConnectionLost {
        chain: Chain,
        reason: String,
    },
    ConnectionRestored {
        chain: Chain,
    },
}

/// Filter for subscribing to specific logs/events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    /// Contract address to filter by
    pub address: Option<String>,
    
    /// Topics to filter by (up to 4 topics)
    pub topics: Vec<Option<String>>,
    
    /// Starting block number
    pub from_block: Option<u64>,
    
    /// Ending block number
    pub to_block: Option<u64>,
}

impl LogFilter {
    /// Create a new log filter
    pub fn new() -> Self {
        Self {
            address: None,
            topics: Vec::new(),
            from_block: None,
            to_block: None,
        }
    }

    /// Filter by contract address
    pub fn address(mut self, address: String) -> Self {
        self.address = Some(address);
        self
    }

    /// Add a topic filter
    pub fn topic(mut self, topic: Option<String>) -> Self {
        if self.topics.len() < 4 {
            self.topics.push(topic);
        }
        self
    }

    /// Set block range
    pub fn block_range(mut self, from: Option<u64>, to: Option<u64>) -> Self {
        self.from_block = from;
        self.to_block = to;
        self
    }
}

impl Default for LogFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Observer performance and operational metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverMetrics {
    /// Chain being observed
    pub chain: Chain,
    
    /// Timestamp of metrics collection
    pub timestamp: DateTime<Utc>,
    
    /// Current block number
    pub current_block: u64,
    
    /// Blocks processed per second
    pub blocks_per_second: f64,
    
    /// Transactions processed per second
    pub transactions_per_second: f64,
    
    /// Logs processed per second
    pub logs_per_second: f64,
    
    /// Average block processing latency (ms)
    pub avg_block_latency_ms: f64,
    
    /// Connection uptime percentage
    pub connection_uptime: f64,
    
    /// Number of connection failures
    pub connection_failures: u64,
    
    /// Number of retries performed
    pub retry_count: u64,
    
    /// Current queue sizes
    pub queue_sizes: HashMap<String, usize>,
    
    /// Memory usage in MB
    pub memory_usage_mb: f64,
    
    /// Custom metrics specific to the observer
    pub custom_metrics: HashMap<String, f64>,
}

impl ObserverMetrics {
    /// Create new metrics for a chain
    pub fn new(chain: Chain) -> Self {
        Self {
            chain,
            timestamp: Utc::now(),
            current_block: 0,
            blocks_per_second: 0.0,
            transactions_per_second: 0.0,
            logs_per_second: 0.0,
            avg_block_latency_ms: 0.0,
            connection_uptime: 100.0,
            connection_failures: 0,
            retry_count: 0,
            queue_sizes: HashMap::new(),
            memory_usage_mb: 0.0,
            custom_metrics: HashMap::new(),
        }
    }

    /// Update block metrics
    pub fn update_block_metrics(&mut self, blocks_per_second: f64, latency_ms: f64) {
        self.blocks_per_second = blocks_per_second;
        self.avg_block_latency_ms = latency_ms;
        self.timestamp = Utc::now();
    }

    /// Update transaction metrics
    pub fn update_transaction_metrics(&mut self, transactions_per_second: f64) {
        self.transactions_per_second = transactions_per_second;
        self.timestamp = Utc::now();
    }

    /// Update log metrics
    pub fn update_log_metrics(&mut self, logs_per_second: f64) {
        self.logs_per_second = logs_per_second;
        self.timestamp = Utc::now();
    }

    /// Record a connection failure
    pub fn record_connection_failure(&mut self) {
        self.connection_failures += 1;
        self.timestamp = Utc::now();
    }

    /// Record a retry attempt
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
        self.timestamp = Utc::now();
    }

    /// Update queue size
    pub fn update_queue_size(&mut self, queue_name: String, size: usize) {
        self.queue_sizes.insert(queue_name, size);
        self.timestamp = Utc::now();
    }

    /// Add custom metric
    pub fn add_custom_metric(&mut self, name: String, value: f64) {
        self.custom_metrics.insert(name, value);
        self.timestamp = Utc::now();
    }
}

/// Observer health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverHealth {
    /// Chain being observed
    pub chain: Chain,
    
    /// Overall health status
    pub status: HealthStatus,
    
    /// Timestamp of health check
    pub timestamp: DateTime<Utc>,
    
    /// Detailed health information
    pub details: HashMap<String, HealthDetail>,
    
    /// Last error message (if any)
    pub last_error: Option<String>,
    
    /// Time since last successful operation
    pub last_success: Option<DateTime<Utc>>,
}

/// Health status levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Detailed health information for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDetail {
    /// Component name
    pub component: String,
    
    /// Component status
    pub status: HealthStatus,
    
    /// Status message
    pub message: String,
    
    /// Relevant metrics
    pub metrics: HashMap<String, f64>,
}

impl ObserverHealth {
    /// Create new health status for a chain
    pub fn new(chain: Chain) -> Self {
        Self {
            chain,
            status: HealthStatus::Unknown,
            timestamp: Utc::now(),
            details: HashMap::new(),
            last_error: None,
            last_success: None,
        }
    }

    /// Update overall health status
    pub fn update_status(&mut self, status: HealthStatus, message: Option<String>) {
        self.status = status;
        self.timestamp = Utc::now();
        
        if let Some(msg) = message {
            match self.status {
                HealthStatus::Healthy => {
                    self.last_success = Some(Utc::now());
                    self.last_error = None;
                }
                HealthStatus::Degraded | HealthStatus::Unhealthy => {
                    self.last_error = Some(msg);
                }
                HealthStatus::Unknown => {}
            }
        }
    }

    /// Add component health detail
    pub fn add_detail(&mut self, component: String, status: HealthStatus, message: String) {
        let detail = HealthDetail {
            component: component.clone(),
            status,
            message,
            metrics: HashMap::new(),
        };
        self.details.insert(component, detail);
        self.timestamp = Utc::now();
    }

    /// Update component metrics
    pub fn update_component_metrics(&mut self, component: &str, metrics: HashMap<String, f64>) {
        if let Some(detail) = self.details.get_mut(component) {
            detail.metrics = metrics;
        }
        self.timestamp = Utc::now();
    }

    /// Check if the observer is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy)
    }

    /// Check if the observer is degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self.status, HealthStatus::Degraded)
    }

    /// Check if the observer is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self.status, HealthStatus::Unhealthy)
    }
}

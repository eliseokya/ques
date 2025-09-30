//! Multi-RPC client with intelligent provider management
//!
//! Provides a unified interface for multiple RPC providers with
//! automatic failover, rate limiting, and performance optimization.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use ethers::providers::{Provider, Http, Ws, Middleware};
use ethers::types::{Block, Transaction, Log, Filter, BlockNumber, TxHash, H256, U64};
use reqwest::Client;
use url::Url;

use crate::{
    config::{ProviderConfig, ProviderSelectionStrategy},
    providers::{
        rate_limiter::ProviderRateLimitManager,
        failover::{FailoverManager, ProviderHealth, ProviderStatus},
    },
    Chain, ProviderType, Result, BetaDataplaneError,
};

/// Multi-RPC client for a specific chain
#[derive(Debug)]
pub struct MultiRpcClient {
    /// Chain this client serves
    chain: Chain,
    
    /// Provider configurations
    providers: Vec<ProviderConfig>,
    
    /// HTTP clients for each provider
    http_clients: Arc<RwLock<HashMap<String, Provider<Http>>>>,
    
    /// WebSocket clients for each provider
    ws_clients: Arc<RwLock<HashMap<String, Provider<Ws>>>>,
    
    /// Rate limit manager
    rate_limiter: ProviderRateLimitManager,
    
    /// Failover manager
    failover: FailoverManager,
    
    /// Provider selection strategy
    selection_strategy: ProviderSelectionStrategy,
    
    /// Client metrics
    metrics: Arc<RwLock<ClientMetrics>>,
}

/// Client performance metrics
#[derive(Debug, Clone)]
pub struct ClientMetrics {
    /// Total requests made
    pub total_requests: u64,
    
    /// Successful requests
    pub successful_requests: u64,
    
    /// Failed requests
    pub failed_requests: u64,
    
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Last request timestamp
    pub last_request: Option<Instant>,
    
    /// Provider usage distribution
    pub provider_usage: HashMap<String, u64>,
}

impl ClientMetrics {
    /// Create new client metrics
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            requests_per_second: 0.0,
            last_request: None,
            provider_usage: HashMap::new(),
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, provider_name: &str, response_time_ms: f64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_request = Some(Instant::now());
        
        // Update average response time
        let alpha = 0.1;
        self.avg_response_time_ms = alpha * response_time_ms + (1.0 - alpha) * self.avg_response_time_ms;
        
        // Update provider usage
        *self.provider_usage.entry(provider_name.to_string()).or_insert(0) += 1;
    }

    /// Record a failed request
    pub fn record_failure(&mut self, provider_name: &str) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_request = Some(Instant::now());
        
        // Update provider usage
        *self.provider_usage.entry(provider_name.to_string()).or_insert(0) += 1;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.successful_requests as f64 / self.total_requests as f64
        } else {
            1.0
        }
    }
}

impl MultiRpcClient {
    /// Create a new multi-RPC client
    pub async fn new(
        chain: Chain,
        providers: Vec<ProviderConfig>,
        selection_strategy: ProviderSelectionStrategy,
    ) -> Result<Self> {
        info!(chain = %chain, provider_count = providers.len(), "Creating multi-RPC client");
        
        let rate_limiter = ProviderRateLimitManager::new();
        let failover = FailoverManager::new(Default::default());
        
        // Initialize rate limiters and health tracking
        for provider in &providers {
            if provider.enabled {
                rate_limiter.add_provider(
                    provider.name.clone(),
                    provider.rate_limit as f64,
                ).await;
                
                failover.add_provider(provider.name.clone()).await;
            }
        }
        
        let client = Self {
            chain,
            providers,
            http_clients: Arc::new(RwLock::new(HashMap::new())),
            ws_clients: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter,
            failover,
            selection_strategy,
            metrics: Arc::new(RwLock::new(ClientMetrics::new())),
        };
        
        // Initialize HTTP clients
        client.initialize_http_clients().await?;
        
        // Start health monitoring
        client.failover.start_health_monitoring().await;
        
        info!(chain = %chain, "Multi-RPC client initialized successfully");
        Ok(client)
    }

    /// Initialize HTTP clients for all providers
    async fn initialize_http_clients(&self) -> Result<()> {
        let mut http_clients = self.http_clients.write().await;
        
        for provider in &self.providers {
            if !provider.enabled {
                continue;
            }
            
            info!(
                provider = provider.name,
                provider_type = %provider.provider_type,
                url = provider.http_url,
                "Initializing HTTP client"
            );
            
            // Create HTTP client with timeout
            let http_client = Client::builder()
                .timeout(Duration::from_secs(provider.timeout_seconds))
                .build()
                .map_err(|e| BetaDataplaneError::Provider {
                    provider: provider.name.clone(),
                    message: format!("Failed to create HTTP client: {}", e),
                })?;
            
            // Create ethers provider
            let provider_client = Provider::<Http>::new(
                Http::new_with_client(
                    Url::parse(&provider.http_url)
                        .map_err(|e| BetaDataplaneError::Provider {
                            provider: provider.name.clone(),
                            message: format!("Invalid HTTP URL: {}", e),
                        })?,
                    http_client,
                )
            );
            
            http_clients.insert(provider.name.clone(), provider_client);
        }
        
        info!("HTTP clients initialized for {} providers", http_clients.len());
        Ok(())
    }

    /// Get the best available provider
    async fn get_best_provider(&self) -> Result<String> {
        let provider_names: Vec<String> = self.providers
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.clone())
            .collect();
        
        match self.selection_strategy {
            ProviderSelectionStrategy::FastestFirst => {
                self.failover.select_best_provider(&provider_names).await
            }
            ProviderSelectionStrategy::RoundRobin => {
                // TODO: Implement round-robin selection
                self.failover.select_best_provider(&provider_names).await
            }
            ProviderSelectionStrategy::Weighted => {
                // TODO: Implement weighted selection
                self.failover.select_best_provider(&provider_names).await
            }
            ProviderSelectionStrategy::PrimaryFallback => {
                // Use first enabled provider, fallback to others
                provider_names.into_iter().next()
            }
        }
        .ok_or_else(|| BetaDataplaneError::internal("No providers available"))
    }

    /// Execute a request with automatic failover
    async fn execute_with_failover<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn(&Provider<Http>) -> futures::future::BoxFuture<'_, std::result::Result<T, ethers::providers::ProviderError>>,
        T: Send + 'static,
    {
        let provider_names: Vec<String> = self.providers
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.clone())
            .collect();
        
        for provider_name in &provider_names {
            // Check rate limit
            if !self.rate_limiter.try_acquire(provider_name).await {
                debug!(provider = provider_name, "Rate limit exceeded, trying next provider");
                continue;
            }
            
            // Get HTTP client
            let http_clients = self.http_clients.read().await;
            let client = match http_clients.get(provider_name) {
                Some(client) => client,
                None => {
                    warn!(provider = provider_name, "HTTP client not found");
                    continue;
                }
            };
            
            // Execute request with timing
            let start_time = Instant::now();
            match operation(client).await {
                Ok(result) => {
                    let response_time_ms = start_time.elapsed().as_millis() as f64;
                    
                    // Record success
                    self.failover.record_success(provider_name, response_time_ms).await;
                    self.metrics.write().await.record_success(provider_name, response_time_ms);
                    
                    debug!(
                        provider = provider_name,
                        response_time_ms = response_time_ms,
                        "Request succeeded"
                    );
                    
                    return Ok(result);
                }
                Err(e) => {
                    let error_message = e.to_string();
                    
                    // Record failure
                    self.failover.record_failure(provider_name, &error_message).await;
                    self.metrics.write().await.record_failure(provider_name);
                    
                    warn!(
                        provider = provider_name,
                        error = error_message,
                        "Request failed, trying next provider"
                    );
                    
                    continue;
                }
            }
        }
        
        Err(BetaDataplaneError::internal("All providers failed"))
    }

    /// Get current block number
    pub async fn get_block_number(&self) -> Result<U64> {
        self.execute_with_failover(|client| {
            Box::pin(async move {
                client.get_block_number().await
            })
        }).await
    }

    /// Get block by number
    pub async fn get_block(&self, block_number: u64) -> Result<Option<Block<TxHash>>> {
        self.execute_with_failover(|client| {
            Box::pin(async move {
                client.get_block(block_number).await
            })
        }).await
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: TxHash) -> Result<Option<Transaction>> {
        self.execute_with_failover(|client| {
            Box::pin(async move {
                client.get_transaction(tx_hash).await
            })
        }).await
    }

    /// Get current gas price
    pub async fn get_gas_price(&self) -> Result<ethers::types::U256> {
        self.execute_with_failover(|client| {
            Box::pin(async move {
                client.get_gas_price().await
            })
        }).await
    }

    /// Get logs with filter
    pub async fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>> {
        self.execute_with_failover(|client| {
            let filter = filter.clone();
            Box::pin(async move {
                client.get_logs(&filter).await
            })
        }).await
    }

    /// Call contract method
    pub async fn call(
        &self,
        tx: &ethers::types::transaction::eip2718::TypedTransaction,
        block: Option<BlockNumber>,
    ) -> Result<ethers::types::Bytes> {
        self.execute_with_failover(|client| {
            let tx = tx.clone();
            Box::pin(async move {
                client.call(&tx, block.map(|b| b.into())).await
            })
        }).await
    }

    /// Get client metrics
    pub async fn get_metrics(&self) -> ClientMetrics {
        self.metrics.read().await.clone()
    }

    /// Get provider health
    pub async fn get_provider_health(&self, provider_name: &str) -> Option<ProviderHealth> {
        self.failover.get_provider_health(provider_name).await
    }

    /// Get all provider health
    pub async fn get_all_provider_health(&self) -> HashMap<String, ProviderHealth> {
        self.failover.get_all_health().await
    }
}

impl Clone for MultiRpcClient {
    fn clone(&self) -> Self {
        Self {
            chain: self.chain,
            providers: self.providers.clone(),
            http_clients: Arc::clone(&self.http_clients),
            ws_clients: Arc::clone(&self.ws_clients),
            rate_limiter: self.rate_limiter.clone(),
            failover: self.failover.clone(),
            selection_strategy: self.selection_strategy.clone(),
            metrics: Arc::clone(&self.metrics),
        }
    }
}

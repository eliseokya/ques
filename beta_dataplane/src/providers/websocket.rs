//! WebSocket provider management for real-time data
//!
//! Handles WebSocket connections to RPC providers for real-time
//! block and event subscriptions with automatic reconnection.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn, error};
use serde_json::{json, Value};
use futures::{SinkExt, StreamExt};

use crate::{
    config::ProviderConfig,
    Chain, Result, BetaDataplaneError,
};

/// WebSocket subscription types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    /// New block headers
    NewHeads,
    
    /// New pending transactions
    PendingTransactions,
    
    /// Logs matching a filter
    Logs,
    
    /// New full blocks
    NewBlocks,
}

/// WebSocket subscription data
#[derive(Debug, Clone)]
pub struct SubscriptionData {
    /// Subscription type
    pub subscription_type: SubscriptionType,
    
    /// Raw JSON data
    pub data: Value,
    
    /// Provider that sent the data
    pub provider: String,
    
    /// Timestamp when received
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// WebSocket client for a single provider
#[derive(Debug)]
pub struct WebSocketClient {
    /// Provider configuration
    provider: ProviderConfig,
    
    /// Chain this client serves
    chain: Chain,
    
    /// Subscription sender
    subscription_tx: Option<mpsc::UnboundedSender<SubscriptionData>>,
    
    /// Connection status
    is_connected: Arc<RwLock<bool>>,
    
    /// Active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionType>>>,
}

impl WebSocketClient {
    /// Create a new WebSocket client
    pub fn new(provider: ProviderConfig, chain: Chain) -> Self {
        Self {
            provider,
            chain,
            subscription_tx: None,
            is_connected: Arc::new(RwLock::new(false)),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the WebSocket connection
    pub async fn start(&mut self) -> Result<mpsc::UnboundedReceiver<SubscriptionData>> {
        let ws_url = self.provider.ws_url.as_ref()
            .ok_or_else(|| BetaDataplaneError::Provider {
                provider: self.provider.name.clone(),
                message: "WebSocket URL not configured".to_string(),
            })?;

        info!(
            provider = self.provider.name,
            chain = %self.chain,
            url = ws_url,
            "Starting WebSocket connection"
        );

        let (subscription_tx, subscription_rx) = mpsc::unbounded_channel();
        self.subscription_tx = Some(subscription_tx.clone());

        // Start connection in background
        let provider_name = self.provider.name.clone();
        let ws_url = ws_url.clone();
        let is_connected = Arc::clone(&self.is_connected);
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            Self::connection_loop(
                provider_name,
                ws_url,
                subscription_tx,
                is_connected,
                subscriptions,
            ).await;
        });

        Ok(subscription_rx)
    }

    /// Main connection loop with automatic reconnection
    async fn connection_loop(
        provider_name: String,
        ws_url: String,
        subscription_tx: mpsc::UnboundedSender<SubscriptionData>,
        is_connected: Arc<RwLock<bool>>,
        subscriptions: Arc<RwLock<HashMap<String, SubscriptionType>>>,
    ) {
        let mut reconnect_delay = Duration::from_secs(1);
        let max_reconnect_delay = Duration::from_secs(60);

        loop {
            match Self::connect_and_handle(&provider_name, &ws_url, &subscription_tx, &subscriptions).await {
                Ok(_) => {
                    info!(provider = provider_name, "WebSocket connection established");
                    *is_connected.write().await = true;
                    reconnect_delay = Duration::from_secs(1); // Reset delay on successful connection
                }
                Err(e) => {
                    error!(
                        provider = provider_name,
                        error = %e,
                        "WebSocket connection failed"
                    );
                    *is_connected.write().await = false;
                }
            }

            // Wait before reconnecting
            warn!(
                provider = provider_name,
                delay_seconds = reconnect_delay.as_secs(),
                "Reconnecting WebSocket in {} seconds",
                reconnect_delay.as_secs()
            );
            
            tokio::time::sleep(reconnect_delay).await;
            
            // Exponential backoff with maximum
            reconnect_delay = std::cmp::min(reconnect_delay * 2, max_reconnect_delay);
        }
    }

    /// Connect and handle WebSocket messages
    async fn connect_and_handle(
        provider_name: &str,
        ws_url: &str,
        subscription_tx: &mpsc::UnboundedSender<SubscriptionData>,
        subscriptions: &Arc<RwLock<HashMap<String, SubscriptionType>>>,
    ) -> Result<()> {
        // Connect to WebSocket
        let (ws_stream, _) = connect_async(ws_url).await
            .map_err(|e| BetaDataplaneError::Provider {
                provider: provider_name.to_string(),
                message: format!("WebSocket connection failed: {}", e),
            })?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to new heads
        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": ["newHeads"]
        });

        ws_sender.send(Message::Text(subscribe_msg.to_string())).await
            .map_err(|e| BetaDataplaneError::Provider {
                provider: provider_name.to_string(),
                message: format!("Failed to send subscription: {}", e),
            })?;

        // Handle incoming messages
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Ok(json_data) = serde_json::from_str::<Value>(&text) {
                        // Check if this is a subscription notification
                        if json_data.get("method") == Some(&Value::String("eth_subscription".to_string())) {
                            let subscription_data = SubscriptionData {
                                subscription_type: SubscriptionType::NewHeads,
                                data: json_data,
                                provider: provider_name.to_string(),
                                timestamp: chrono::Utc::now(),
                            };

                            if let Err(_) = subscription_tx.send(subscription_data) {
                                warn!(provider = provider_name, "Subscription receiver dropped");
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!(provider = provider_name, "WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    error!(provider = provider_name, error = %e, "WebSocket error");
                    break;
                }
                _ => {
                    // Ignore other message types
                }
            }
        }

        Ok(())
    }

    /// Subscribe to new block headers
    pub async fn subscribe_new_heads(&self) -> Result<()> {
        // TODO: Implement subscription management
        debug!(
            provider = self.provider.name,
            "Subscribing to new heads"
        );
        Ok(())
    }

    /// Subscribe to logs with filter
    pub async fn subscribe_logs(&self, _filter: ethers::types::Filter) -> Result<()> {
        // TODO: Implement log subscription
        debug!(
            provider = self.provider.name,
            "Subscribing to logs"
        );
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> WebSocketStats {
        WebSocketStats {
            provider: self.provider.name.clone(),
            is_connected: self.is_connected().await,
            subscriptions_count: self.subscriptions.read().await.len(),
            messages_received: 0, // TODO: Track actual stats
            last_message: None,
        }
    }
}

/// WebSocket connection statistics
#[derive(Debug, Clone)]
pub struct WebSocketStats {
    /// Provider name
    pub provider: String,
    
    /// Connection status
    pub is_connected: bool,
    
    /// Number of active subscriptions
    pub subscriptions_count: usize,
    
    /// Total messages received
    pub messages_received: u64,
    
    /// Last message timestamp
    pub last_message: Option<chrono::DateTime<chrono::Utc>>,
}

/// WebSocket manager for multiple providers
#[derive(Debug)]
pub struct WebSocketManager {
    /// WebSocket clients for each provider
    clients: HashMap<String, WebSocketClient>,
    
    /// Combined subscription receiver
    combined_rx: Option<mpsc::UnboundedReceiver<SubscriptionData>>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            combined_rx: None,
        }
    }

    /// Add a WebSocket client
    pub fn add_client(&mut self, provider_name: String, client: WebSocketClient) {
        self.clients.insert(provider_name, client);
    }

    /// Start all WebSocket connections
    pub async fn start_all(&mut self) -> Result<mpsc::UnboundedReceiver<SubscriptionData>> {
        let (combined_tx, combined_rx) = mpsc::unbounded_channel();

        for (provider_name, client) in &mut self.clients {
            info!(provider = provider_name, "Starting WebSocket client");
            
            // Start individual client
            let individual_rx = client.start().await?;
            
            // Forward messages to combined channel
            let combined_tx_clone = combined_tx.clone();
            tokio::spawn(async move {
                let mut rx = individual_rx;
                while let Some(data) = rx.recv().await {
                    if let Err(_) = combined_tx_clone.send(data) {
                        break; // Combined receiver dropped
                    }
                }
            });
        }

        self.combined_rx = Some(combined_rx);
        Ok(self.combined_rx.take().unwrap())
    }

    /// Get statistics for all clients
    pub async fn get_all_stats(&self) -> HashMap<String, WebSocketStats> {
        let mut stats = HashMap::new();
        
        for (provider_name, client) in &self.clients {
            stats.insert(provider_name.clone(), client.get_stats().await);
        }
        
        stats
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

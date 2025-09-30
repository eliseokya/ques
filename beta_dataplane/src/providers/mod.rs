//! Provider management for beta dataplane
//!
//! Handles multi-RPC provider connections with failover, rate limiting,
//! and intelligent routing for optimal performance.

pub mod multi_rpc;
pub mod ethereum;
pub mod arbitrum;
pub mod optimism;
pub mod base;
pub mod websocket;
pub mod rate_limiter;
pub mod failover;
pub mod api_keys;

// Re-export commonly used types
pub use multi_rpc::{MultiRpcClient, ClientMetrics};
pub use rate_limiter::{RateLimiter, ProviderRateLimitManager};
pub use failover::{FailoverManager, ProviderHealth, ProviderStatus};
pub use ethereum::EthereumRpcClient;
pub use arbitrum::ArbitrumRpcClient;
pub use optimism::OptimismRpcClient;
pub use base::BaseRpcClient;
pub use websocket::{WebSocketClient, WebSocketManager, SubscriptionData, SubscriptionType};
pub use api_keys::{ApiKeyManager, ApiKeyConfigSummary, ProviderEndpoints};

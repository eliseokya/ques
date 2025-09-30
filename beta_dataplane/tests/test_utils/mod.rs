//! Test utilities and helpers
//!
//! Common functions and fixtures for tests.

use qenus_dataplane::{Feature, FeatureType, FeatureData, Chain};
use chrono::Utc;
use uuid::Uuid;

/// Create a sample AMM feature for testing
pub fn create_sample_amm_feature(block_number: u64) -> Feature {
    Feature {
        id: Uuid::new_v4(),
        feature_type: FeatureType::Amm,
        chain: Chain::Ethereum,
        block_number,
        timestamp: Utc::now(),
        data: FeatureData::Amm(qenus_dataplane::AmmFeature {
            protocol: "uniswap_v3".to_string(),
            pool: format!("0x{:040x}", block_number),
            token0: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(), // USDC
            token1: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(), // WETH
            price: 1850.0,
            liquidity: 5000000.0,
            fee_tier: 3000,
        }),
        source: "test_extractor".to_string(),
        version: "1.0.0".to_string(),
    }
}

/// Create a sample gas feature for testing
pub fn create_sample_gas_feature(block_number: u64) -> Feature {
    Feature {
        id: Uuid::new_v4(),
        feature_type: FeatureType::Gas,
        chain: Chain::Ethereum,
        block_number,
        timestamp: Utc::now(),
        data: FeatureData::Gas(qenus_dataplane::GasFeature {
            base_fee: 30.0,
            priority_fee: 2.0,
            gas_used: 12000000,
            gas_limit: 30000000,
        }),
        source: "test_extractor".to_string(),
        version: "1.0.0".to_string(),
    }
}

/// Create a sample bridge feature for testing
pub fn create_sample_bridge_feature(block_number: u64) -> Feature {
    Feature {
        id: Uuid::new_v4(),
        feature_type: FeatureType::Bridge,
        chain: Chain::Ethereum,
        block_number,
        timestamp: Utc::now(),
        data: FeatureData::Bridge(qenus_dataplane::BridgeFeature {
            protocol: "optimism_canonical".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "optimism".to_string(),
            token: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(), // WETH
            amount: 10.5,
            fee: 0.001,
        }),
        source: "test_extractor".to_string(),
        version: "1.0.0".to_string(),
    }
}

/// Create multiple sample features
pub fn create_sample_features(count: usize, base_block: u64) -> Vec<Feature> {
    (0..count)
        .map(|i| {
            let block_number = base_block + i as u64;
            match i % 3 {
                0 => create_sample_amm_feature(block_number),
                1 => create_sample_gas_feature(block_number),
                _ => create_sample_bridge_feature(block_number),
            }
        })
        .collect()
}

/// Wait for async operations with timeout
pub async fn wait_for_condition<F, Fut>(mut condition: F, timeout_secs: u64) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sample_features() {
        let features = create_sample_features(10, 18000000);
        assert_eq!(features.len(), 10);
        
        // Check variety
        let amm_count = features.iter().filter(|f| matches!(f.feature_type, FeatureType::Amm)).count();
        let gas_count = features.iter().filter(|f| matches!(f.feature_type, FeatureType::Gas)).count();
        let bridge_count = features.iter().filter(|f| matches!(f.feature_type, FeatureType::Bridge)).count();
        
        assert!(amm_count > 0);
        assert!(gas_count > 0);
        assert!(bridge_count > 0);
    }

    #[tokio::test]
    async fn test_wait_for_condition() {
        let mut counter = 0;
        let result = wait_for_condition(
            || {
                counter += 1;
                async move { counter >= 3 }
            },
            1,
        )
        .await;

        assert!(result);
    }
}
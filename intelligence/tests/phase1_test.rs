//! Phase 1 integration tests - Market state and detectors

use std::sync::Arc;
use qenus_intelligence::{MarketState, DetectorManager, StrategyConfig};
use qenus_dataplane::{Feature, FeatureData, AmmFeature, TokenInfo, DepthCurve, Chain};
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

#[tokio::test]
async fn test_market_state_ingestion() {
    // Create market state
    let market_state = Arc::new(MarketState::new(30));
    
    // Create test AMM feature
    let feature = Feature {
        id: Uuid::new_v4(),
        block_number: 1000,
        chain: Chain::Ethereum,
        timestamp: Utc::now(),
        feature_type: qenus_dataplane::FeatureType::Amm,
        data: FeatureData::Amm(AmmFeature {
            pool_address: "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640".to_string(),
            pool_type: "uniswap_v3".to_string(),
            token0: TokenInfo {
                address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
                symbol: "WETH".to_string(),
                decimals: 18,
            },
            token1: TokenInfo {
                address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
                symbol: "USDC".to_string(),
                decimals: 6,
            },
            fee_tier: Some(5),
            reserves: {
                let mut m = HashMap::new();
                m.insert("WETH".to_string(), "100.0".to_string());
                m.insert("USDC".to_string(), "300000.0".to_string());
                m
            },
            mid_price: 3000.0,
            liquidity: "1000000.0".to_string(),
            depth: DepthCurve {
                sizes: HashMap::new(),
            },
            volume_24h: None,
            fees_24h: None,
        }),
        source: "test".to_string(),
        version: "1.0".to_string(),
    };
    
    // Ingest feature
    market_state.ingest_feature(feature).await.unwrap();
    
    // Query state
    let price = market_state.get_price(Chain::Ethereum, "WETH").await;
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 3000.0);
    
    // Check stats
    let stats = market_state.get_stats().await;
    assert_eq!(stats.total_amm_pools, 1);
}

#[tokio::test]
async fn test_detector_manager_creation() {
    // Create market state
    let market_state = Arc::new(MarketState::new(30));
    
    // Create test strategy config
    let triangle_config = StrategyConfig {
        name: "test_triangle".to_string(),
        enabled: true,
        min_profit_usd: 500.0,
        min_profit_bps: 10.0,
        max_position_usd: 1_000_000.0,
        approved_assets: vec!["WETH".to_string(), "USDC".to_string()],
        approved_chains: vec![Chain::Ethereum, Chain::Arbitrum],
        risk_limits: Default::default(),
    };
    
    // Create detector manager
    let detector_manager = DetectorManager::new(
        Some(triangle_config),
        None,
        market_state,
    );
    
    // Run detection (should return empty as no state populated)
    let candidates = detector_manager.detect_all().await.unwrap();
    assert_eq!(candidates.len(), 0);
}

#[tokio::test]
async fn test_end_to_end_detection() {
    // Create market state
    let market_state = Arc::new(MarketState::new(30));
    
    // TODO: Populate market state with test features
    // TODO: Run detectors
    // TODO: Verify candidates are generated
    
    // This is a placeholder for a more comprehensive end-to-end test
    // that will be implemented when we have Kafka/gRPC ingestion
}


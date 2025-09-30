//! Live extraction tests using real Ankr endpoints
//!
//! Tests the feature extractors against live blockchain data.

use qenus_beta_dataplane::{
    config::{BetaDataplaneConfig, ProviderConfig},
    providers::EthereumRpcClient,
    extractors::{
        amm::UniswapV3Extractor,
        gas::GasPricingExtractor,
        traits::{BetaFeatureExtractor, ExtractionContext, ExtractorConfig},
    },
    Chain, ProviderType,
};

#[tokio::test]
async fn test_uniswap_v3_extraction_live() {
    // Create provider config with Ankr endpoint
    let provider_config = ProviderConfig {
        provider_type: ProviderType::Ankr,
        name: "ankr-ethereum-test".to_string(),
        http_url: "https://rpc.ankr.com/eth/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928".to_string(),
        ws_url: None,
        api_key: None,
        rate_limit: 200,
        timeout_seconds: 30,
        max_retries: 3,
        weight: 1.0,
        enabled: true,
    };

    // Create RPC client
    let client = EthereumRpcClient::new(vec![provider_config]).await.unwrap();

    // Get current block number
    let current_block = client.get_current_block().await.unwrap();
    println!("✅ Current Ethereum block: {}", current_block);
    assert!(current_block > 0);

    // Create Uniswap V3 extractor
    let mut extractor = UniswapV3Extractor::new(ExtractorConfig::default());
    extractor.set_client(client.clone());
    println!("✅ Uniswap V3 extractor initialized with {} pools", extractor.pool_count());

    // Create extraction context
    let context = ExtractionContext::new(
        Chain::Ethereum,
        current_block,
        "ankr-ethereum-test".to_string(),
    );

    // Extract features
    println!("🔍 Extracting Uniswap V3 pool features...");
    let features = extractor.extract_latest(Chain::Ethereum, &context).await.unwrap();

    println!("✅ Extracted {} features", features.len());
    
    // Validate features
    for (i, feature) in features.iter().enumerate() {
        println!("\n📊 Feature {}:", i + 1);
        println!("  Chain: {:?}", feature.chain);
        println!("  Block: {}", feature.block_number);
        println!("  Type: {:?}", feature.feature_type);
        
        if let qenus_dataplane::FeatureData::Amm(amm) = &feature.data {
            println!("  Pool: {}", amm.pool_address);
            println!("  Type: {}", amm.pool_type);
            println!("  Mid Price: {:.6}", amm.mid_price);
            println!("  Liquidity: {}", amm.liquidity);
            println!("  Token0: {} ({})", amm.token0.symbol, amm.token0.address);
            println!("  Token1: {} ({})", amm.token1.symbol, amm.token1.address);
            
            // Validate data
            assert!(amm.mid_price > 0.0, "Mid price should be positive");
            assert!(!amm.reserves.is_empty(), "Reserves should not be empty");
            assert!(!amm.depth.sizes.is_empty(), "Depth curve should not be empty");
        }
    }

    assert!(!features.is_empty(), "Should extract at least one feature");
}

#[tokio::test]
async fn test_gas_pricing_extraction_live() {
    // Create provider config
    let provider_config = ProviderConfig {
        provider_type: ProviderType::Ankr,
        name: "ankr-ethereum-test".to_string(),
        http_url: "https://rpc.ankr.com/eth/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928".to_string(),
        ws_url: None,
        api_key: None,
        rate_limit: 200,
        timeout_seconds: 30,
        max_retries: 3,
        weight: 1.0,
        enabled: true,
    };

    // Create RPC client
    let client = EthereumRpcClient::new(vec![provider_config]).await.unwrap();

    // Get current block
    let current_block = client.get_current_block().await.unwrap();
    println!("✅ Current block: {}", current_block);

    // Create gas pricing extractor
    let extractor = GasPricingExtractor::new(ExtractorConfig::default());

    // Create extraction context
    let context = ExtractionContext::new(
        Chain::Ethereum,
        current_block,
        "ankr-ethereum-test".to_string(),
    );

    // Extract gas features
    println!("🔍 Extracting gas pricing features...");
    let features = extractor.extract_latest(Chain::Ethereum, &context).await.unwrap();

    println!("✅ Extracted {} gas features", features.len());

    // Validate features
    for feature in &features {
        if let qenus_dataplane::FeatureData::Gas(gas) = &feature.data {
            println!("\n⛽ Gas Pricing:");
            println!("  Base Fee: {:.2} gwei", gas.base_fee);
            println!("  Priority Fee: {:.2} gwei", gas.priority_fee);
            println!("  Fast: {:.2} gwei", gas.fast_gas_price);
            println!("  Standard: {:.2} gwei", gas.standard_gas_price);
            println!("  Safe: {:.2} gwei", gas.safe_gas_price);
            println!("  Pending TXs: {}", gas.pending_tx_count);
            
            // Validate data
            assert!(gas.base_fee >= 0.0, "Base fee should be non-negative");
            assert!(gas.fast_gas_price >= gas.standard_gas_price, "Fast should be >= standard");
            assert!(gas.standard_gas_price >= gas.safe_gas_price, "Standard should be >= safe");
        }
    }

    assert!(!features.is_empty(), "Should extract gas features");
}

#[tokio::test]
async fn test_multi_chain_extraction() {
    println!("🌐 Testing multi-chain extraction capability...");

    let chains = vec![
        (Chain::Ethereum, "https://rpc.ankr.com/eth/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928"),
        (Chain::Arbitrum, "https://rpc.ankr.com/arbitrum/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928"),
        (Chain::Optimism, "https://rpc.ankr.com/optimism/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928"),
        (Chain::Base, "https://rpc.ankr.com/eth/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928"),
    ];

    for (chain, rpc_url) in chains {
        println!("\n📡 Testing {} connection...", chain);
        
        let provider_config = ProviderConfig {
            provider_type: ProviderType::Ankr,
            name: format!("ankr-{}-test", chain.name()),
            http_url: rpc_url.to_string(),
            ws_url: None,
            api_key: None,
            rate_limit: 200,
            timeout_seconds: 30,
            max_retries: 3,
            weight: 1.0,
            enabled: true,
        };

        // Test connection by getting block number
        let client = EthereumRpcClient::new(vec![provider_config]).await.unwrap();
        let block_number = client.get_current_block().await.unwrap();
        
        println!("  ✅ {} - Current block: {}", chain, block_number);
        assert!(block_number > 0, "{} should have a valid block number", chain);
    }

    println!("\n🎉 All chains are accessible!");
}

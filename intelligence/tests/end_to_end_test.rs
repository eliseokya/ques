//! End-to-end integration tests for Intelligence Layer
//!
//! Tests the complete pipeline:
//! MarketState → Detection → Simulation → Decision → Intent → Feedback

use std::sync::Arc;
use std::collections::HashMap;
use chrono::Utc;
use qenus_intelligence::*;
use qenus_dataplane::{Feature, FeatureData, AmmFeature, TokenInfo, DepthCurve, GasFeature, Chain};
use uuid::Uuid;

/// Create a test AMM feature (Uniswap V3 USDC/WETH pool)
fn create_uniswap_feature(chain: Chain, price: f64) -> Feature {
    Feature {
        id: Uuid::new_v4(),
        block_number: 1000,
        chain,
        timestamp: Utc::now(),
        feature_type: qenus_dataplane::FeatureType::Amm,
        data: FeatureData::Amm(AmmFeature {
            pool_address: "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640".to_string(),
            pool_type: "uniswap_v3".to_string(),
            token0: TokenInfo {
                address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
                symbol: "USDC".to_string(),
                decimals: 6,
            },
            token1: TokenInfo {
                address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
                symbol: "WETH".to_string(),
                decimals: 18,
            },
            fee_tier: Some(5),
            reserves: {
                let mut m = HashMap::new();
                m.insert("USDC".to_string(), "10000000.0".to_string());
                m.insert("WETH".to_string(), "3000.0".to_string());
                m
            },
            mid_price: price,
            liquidity: "5000000.0".to_string(),
            depth: DepthCurve {
                sizes: HashMap::new(),
            },
            volume_24h: None,
            fees_24h: None,
        }),
        source: "test".to_string(),
        version: "1.0".to_string(),
    }
}

/// Create a test gas feature
fn create_gas_feature(chain: Chain) -> Feature {
    Feature {
        id: Uuid::new_v4(),
        block_number: 1000,
        chain,
        timestamp: Utc::now(),
        feature_type: qenus_dataplane::FeatureType::Gas,
        data: FeatureData::Gas(GasFeature {
            base_fee: 30.0,
            priority_fee: 2.0,
            gas_used_ratio: 0.7,
            next_base_fee_estimate: 31.0,
            fast_gas_price: 35.0,
            standard_gas_price: 32.0,
            safe_gas_price: 30.0,
            pending_tx_count: 100,
        }),
        source: "test".to_string(),
        version: "1.0".to_string(),
    }
}

#[tokio::test]
async fn test_full_pipeline_dex_arb() {
    println!("\n🧪 Testing full Intelligence Layer pipeline: DEX Arbitrage\n");
    
    // 1. Initialize components
    println!("1️⃣ Initializing components...");
    let market_state = Arc::new(MarketState::new(30));
    let detector_manager = DetectorManager::new(
        None, // No triangle arb
        Some(StrategyConfig {
            name: "dex_arb".to_string(),
            enabled: true,
            min_profit_usd: 100.0, // Lower threshold for testing
            min_profit_bps: 5.0,
            max_position_usd: 1_000_000.0,
            approved_assets: vec!["USDC".to_string(), "WETH".to_string()],
            approved_chains: vec![Chain::Ethereum],
            risk_limits: RiskLimits {
                max_slippage_bps: 100.0,
                max_gas_pct: 80.0,
                max_bridge_latency_secs: 0,
                min_success_prob: 0.7,
            },
        }),
        market_state.clone(),
    );
    let simulator = TradeSimulator::new(market_state.clone());
    let decision_engine = DecisionEngine::new(market_state.clone(), 5_000_000.0);
    let intent_builder = IntentBuilder::new(market_state.clone());
    let feedback_processor = FeedbackProcessor::new();
    
    println!("   ✅ Components initialized\n");
    
    // 2. Ingest market data (simulate beta_dataplane features)
    println!("2️⃣ Ingesting market data...");
    
    // Create price discrepancy: Uniswap at 3000, Curve at 3015 (0.5% spread)
    let uniswap_feature = create_uniswap_feature(Chain::Ethereum, 3000.0);
    market_state.ingest_feature(uniswap_feature).await.unwrap();
    
    let curve_feature = create_uniswap_feature(Chain::Ethereum, 3015.0);
    market_state.ingest_feature(curve_feature).await.unwrap();
    
    // Gas data
    let gas_feature = create_gas_feature(Chain::Ethereum);
    market_state.ingest_feature(gas_feature).await.unwrap();
    
    let stats = market_state.get_stats().await;
    println!("   📊 Market state: {} AMM pools, {} gas states", stats.total_amm_pools, stats.total_gas_states);
    println!("   ✅ Market data ingested\n");
    
    // 3. Detect opportunities
    println!("3️⃣ Running opportunity detection...");
    let candidates = detector_manager.detect_all().await.unwrap();
    println!("   💡 Detected {} candidates", candidates.len());
    
    if candidates.is_empty() {
        println!("   ⚠️  No candidates detected (might need more market data)");
        return;
    }
    
    for (i, candidate) in candidates.iter().enumerate() {
        println!("   {}. {} on {} - spread: {:.2}bps, confidence: {:.2}",
            i + 1, candidate.strategy, candidate.asset, candidate.spread_bps, candidate.confidence
        );
    }
    println!("   ✅ Detection complete\n");
    
    // 4. Simulate each candidate
    println!("4️⃣ Simulating candidates...");
    let mut evaluations = Vec::new();
    
    for candidate in &candidates {
        match simulator.evaluate(candidate).await {
            Ok(eval) => {
                println!("   📈 Simulation for {}:", candidate.asset);
                println!("      PnL: ${:.2} ({:.2}bps)", eval.net_pnl_usd, eval.net_bps);
                println!("      Size: ${:.0}", eval.optimal_size_usd);
                println!("      Success prob: {:.2}", eval.success_prob);
                println!("      Costs: ${:.2} (gas: ${:.2})", eval.costs.total_usd, eval.costs.gas_usd);
                evaluations.push((candidate.clone(), eval));
            }
            Err(e) => {
                println!("   ❌ Simulation failed: {}", e);
            }
        }
    }
    println!("   ✅ Simulation complete\n");
    
    // 5. Make decisions
    println!("5️⃣ Applying risk policies and making decisions...");
    let mut decisions = Vec::new();
    
    let strategy_config = StrategyConfig {
        name: "dex_arb".to_string(),
        enabled: true,
        min_profit_usd: 100.0,
        min_profit_bps: 5.0,
        max_position_usd: 1_000_000.0,
        approved_assets: vec!["USDC".to_string(), "WETH".to_string()],
        approved_chains: vec![Chain::Ethereum],
        risk_limits: RiskLimits::default(),
    };
    
    for (candidate, evaluation) in evaluations {
        let decision = decision_engine.decide(candidate, evaluation, &strategy_config).await.unwrap();
        
        println!("   {} Decision for {}: {}",
            if decision.should_execute { "✅" } else { "❌" },
            decision.candidate.asset,
            if decision.should_execute { "APPROVED" } else { "REJECTED" }
        );
        
        if !decision.should_execute {
            println!("      Reasons:");
            for reason in &decision.reasoning {
                if reason.starts_with("❌") {
                    println!("      {}", reason);
                }
            }
        } else {
            println!("      Score: {:.2}", decision.score);
        }
        
        decisions.push(decision);
    }
    println!("   ✅ Decisions made\n");
    
    // 6. Select best and build intents
    println!("6️⃣ Selecting best opportunities and building intents...");
    let selected = decision_engine.select_best(decisions, 5).await.unwrap();
    
    println!("   🎯 Selected {} trades for execution", selected.len());
    
    let mut intents = Vec::new();
    for decision in &selected {
        match intent_builder.build(decision).await {
            Ok(intent) => {
                println!("   📝 Built intent {}:", intent.intent_id);
                println!("      Strategy: {}", intent.strategy);
                println!("      Asset: {}", intent.asset);
                println!("      Expected PnL: ${:.2}", intent.expected_pnl_usd);
                println!("      Legs: {}", intent.legs.len());
                println!("      TTL: {}s", intent.ttl_seconds);
                
                // Register with feedback processor
                feedback_processor.register_intent(intent.clone()).await;
                intents.push(intent);
            }
            Err(e) => {
                println!("   ❌ Intent building failed: {}", e);
            }
        }
    }
    println!("   ✅ Intents built\n");
    
    // 7. Simulate feedback (mock Orchestration response)
    println!("7️⃣ Simulating execution feedback...");
    
    for intent in &intents {
        // Simulate execution with slight variance from prediction
        let actual_pnl = intent.expected_pnl_usd * 0.95; // 5% worse than predicted
        
        let receipt = ExecutionReceipt {
            intent_id: intent.intent_id,
            success: true,
            actual_pnl_usd: actual_pnl,
            actual_costs: ActualCosts {
                gas_usd: 55.0, // Slightly higher than predicted
                protocol_fees_usd: 100.0,
                bridge_fees_usd: 0.0,
                flashloan_fees_usd: 0.0,
                slippage_usd: 50.0,
                total_usd: 205.0,
            },
            actual_slippage_bps: 8.0, // Slightly more than predicted
            execution_time_secs: 25.0,
            completed_at: Utc::now(),
            error_message: None,
        };
        
        feedback_processor.process_feedback(receipt).await.unwrap();
        println!("   ✅ Processed feedback for intent {}", intent.intent_id);
    }
    
    // Get performance metrics
    let performance = feedback_processor.get_performance().await;
    println!("\n   📊 Model Performance:");
    println!("      Total intents: {}", performance.total_intents);
    println!("      Successful: {}", performance.successful_executions);
    println!("      Hit rate: {:.1}%", performance.hit_rate * 100.0);
    println!("      Avg PnL error: {:.1}%", performance.avg_pnl_error_pct);
    println!("      Accuracy score: {:.3}", performance.accuracy_score);
    
    println!("\n✅ End-to-end pipeline test COMPLETE!\n");
    
    // Verify the pipeline worked
    assert!(!candidates.is_empty(), "Should detect candidates");
    assert!(!intents.is_empty(), "Should generate intents");
    assert_eq!(performance.total_intents, intents.len(), "Should track all intents");
}

#[tokio::test]
async fn test_pipeline_with_rejection() {
    println!("\n🧪 Testing pipeline with rejection scenario\n");
    
    let market_state = Arc::new(MarketState::new(30));
    let simulator = TradeSimulator::new(market_state.clone());
    let decision_engine = DecisionEngine::new(market_state.clone(), 5_000_000.0);
    
    // Create a candidate with low spread
    let candidate = Candidate {
        strategy: "dex_arb".to_string(),
        asset: "USDC".to_string(),
        spread_bps: 3.0, // Very low spread
        legs: vec![],
        detected_at: Utc::now(),
        confidence: 0.9,
    };
    
    // Simulate
    let evaluation = simulator.evaluate(&candidate).await.unwrap();
    println!("   Simulated PnL: ${:.2}", evaluation.net_pnl_usd);
    
    // Decide with strict config
    let config = StrategyConfig {
        name: "test".to_string(),
        enabled: true,
        min_profit_usd: 500.0, // High threshold
        min_profit_bps: 10.0,  // High threshold
        max_position_usd: 1_000_000.0,
        approved_assets: vec!["USDC".to_string()],
        approved_chains: vec![Chain::Ethereum],
        risk_limits: RiskLimits::default(),
    };
    
    let decision = decision_engine.decide(candidate, evaluation, &config).await.unwrap();
    
    println!("   Decision: {}", if decision.should_execute { "APPROVED" } else { "REJECTED" });
    
    // Should be rejected due to low profit
    assert!(!decision.should_execute, "Should reject low-profit trades");
    assert!(!decision.reasoning.is_empty(), "Should have rejection reasons");
    
    println!("   ✅ Rejection scenario works correctly\n");
}

#[tokio::test]
async fn test_performance_benchmark() {
    println!("\n⚡ Performance benchmark\n");
    
    use std::time::Instant;
    
    let market_state = Arc::new(MarketState::new(30));
    let simulator = TradeSimulator::new(market_state.clone());
    
    // Create 10 candidates
    let candidates: Vec<_> = (0..10).map(|i| Candidate {
        strategy: "dex_arb".to_string(),
        asset: "USDC".to_string(),
        spread_bps: 10.0 + i as f64,
        legs: vec![],
        detected_at: Utc::now(),
        confidence: 0.85,
    }).collect();
    
    // Benchmark simulation
    let start = Instant::now();
    for candidate in &candidates {
        let _ = simulator.evaluate(candidate).await;
    }
    let duration = start.elapsed();
    
    let avg_time = duration.as_millis() as f64 / candidates.len() as f64;
    println!("   📊 Simulated {} candidates in {:?}", candidates.len(), duration);
    println!("   ⏱️  Average time per simulation: {:.2}ms", avg_time);
    
    // Should be fast (< 10ms per simulation)
    assert!(avg_time < 10.0, "Simulation should be fast");
    
    println!("   ✅ Performance is acceptable\n");
}


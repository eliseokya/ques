# Intelligence Layer - Build Plan

**The Brain of Qenus: From Beta Dataplane Features → Executable Trade Intents**

---

## 🎯 **Strategic Context**

The Intelligence Layer is the **decision-making core** that:
1. Consumes live market data from **`beta_dataplane`** (NOT the incomplete `dataplane/reth_fork`)
2. Applies strategy configs from **`business/`** module
3. Outputs **`TradeIntent`** objects to Orchestration for execution

**Key Input Sources:**
- ✅ **beta_dataplane**: Live on-chain data via Kafka/gRPC (OPERATIONAL)
- 🔄 **business/**: Strategy YAMLs, risk policies (TO BE BUILT)
- ❌ **dataplane/**: Future optimization, NOT USED YET

---

## **Phase 0 – Scaffolding** (1 week) ✅ **COMPLETE**

### **Objective:** Set up skeleton, types, and ingestion loop

### **Completed:**
- ✅ Scaffold `intelligence/` folder structure
- ✅ Define core data types:
  - `MarketState` - Rolling market state manager
  - `Candidate` - Detected arbitrage opportunity
  - `EvaluationResult` - Simulation output with PnL
  - `TradeIntent` - Executable trade specification
- ✅ Module interfaces defined (state, detectors, simulator, decision, intent_builder, feedback)
- ✅ Error handling with comprehensive error types
- ✅ Configuration structure for strategies
- ✅ Workspace integration - compiles successfully

### **Deliverable:** ✅
Events from **beta_dataplane** can be parsed into `MarketState` and stored in memory.

---

## **Phase 1 – Candidate Detection** (2 weeks)

### **Objective:** Implement detectors that map beta_dataplane features → raw opportunities

### **Tasks:**
1. **Implement `state.rs` - Market State Manager**
   - Subscribe to beta_dataplane Kafka topics:
     - `qenus.beta.features.amm` (Uniswap, Curve, Balancer)
     - `qenus.beta.features.gas`
     - `qenus.beta.features.bridge`
     - `qenus.beta.features.flashloan`
   - Or consume via gRPC from beta_dataplane (port 50053)
   - Maintain rolling state per chain:
     ```rust
     get_price(chain, asset) → f64
     get_depth(chain, asset, size) → SlippageCurve
     get_bridge_fee(from_chain, to_chain, asset) → f64
     get_flashloan_liquidity(chain, asset) → f64
     ```
   - Cache features with TTL (use beta_dataplane's Redis if needed)

2. **Implement `detectors.rs` - Opportunity Detectors**
   - **Triangle Arbitrage Detector**:
     - Pattern: L2 → Bridge → L1 → Bridge → L2
     - Check: `price(L2_A) > price(L1) > price(L2_B) + costs`
     - Use beta_dataplane bridge features for fees/latency
   
   - **Single-Edge Arbitrage Detector**:
     - Pattern: L2 ↔ L1
     - Check: `price(L2) != price(L1) + bridge_fee`
   
   - **DEX Arbitrage Detector**:
     - Pattern: Uniswap → Curve → Balancer (same chain)
     - Check: Price discrepancies across DEXs from beta_dataplane AMM features

3. **Integrate with `business/` configs** (YAML)
   - Parse `business/strategies/triangle_arb.yaml`:
     ```yaml
     min_spread_bps: 10
     watchlist_assets: [USDC, WETH, USDT]
     enabled_chains: [ethereum, arbitrum, optimism, base]
     ```
   - Filter candidates based on strategy thresholds
   - Apply asset whitelists from business configs

4. **Emit `Candidate` objects**
   - Internal candidate bus (mpsc channel)
   - Log detected opportunities with spread size

### **Unit Tests:**
- Mock beta_dataplane features → expected candidates
- Known spreads (e.g., Uniswap $1850, Curve $1855) → DEX arb candidate
- Cross-chain price diff → triangle arb candidate

### **Deliverable:**
Intelligence detects possible arbitrage cycles from live **beta_dataplane** events.

---

## **Phase 2 – Simulation Engine** (3 weeks)

### **Objective:** Build realistic profitability models using beta_dataplane data

### **Tasks:**
1. **Implement `simulator.rs` with real beta_dataplane integration:**

   **AMM Models**:
   - Constant product (Uniswap V2 style)
   - Concentrated liquidity (Uniswap V3 - use tick math from beta_dataplane)
   - Stableswap (Curve - use virtual price from beta_dataplane)
   - Weighted pools (Balancer - use weights from beta_dataplane)

   **Slippage Curves**:
   - Use **beta_dataplane's depth curves** directly from AMM features
   - Evaluate at trade sizes: $100k, $500k, $1M
   - Extract from `Feature.data.Amm.depth` field

   **Gas Model**:
   - Consume **beta_dataplane gas features** (base_fee, priority_fee)
   - Convert gwei → USD using ETH price from AMM features
   - Estimate total gas: `(base_fee + priority_fee) * gas_units * eth_price`

   **Bridge Model**:
   - Use **beta_dataplane bridge features** (fee_bps, settlement_time_estimate)
   - Add latency penalty for time-value of capital
   - Check liquidity from bridge features

   **Flash Loan Model**:
   - Use **beta_dataplane flash loan features** (available_liquidity, fee_bps)
   - Aave V3: 5 bps fee (from beta_dataplane)
   - Balancer: 0 bps (from beta_dataplane)
   - Cap check: `trade_size <= available_liquidity`

2. **Output `EvaluationResult`**:
   ```rust
   {
     net_pnl_usd: 1250.0,
     net_bps: 12.5,
     optimal_size_usd: 250000.0,
     success_prob: 0.89,
     costs: {
       gas_usd: 45.0,
       protocol_fees_usd: 75.0,
       slippage_usd: 125.0,
       total_usd: 245.0
     }
   }
   ```

### **Fuzz Tests:**
- Randomized pool states from beta_dataplane → stable evaluations
- Extreme sizes → proper slippage calculation
- Low liquidity → correct failure detection

### **Deliverable:**
Every candidate has realistic projected PnL using **real beta_dataplane market data**.

---

## **Phase 3 – Decision Engine** (2 weeks)

### **Objective:** Apply risk policies and select best opportunities

### **Tasks:**
1. **Implement `decision.rs` with business module integration:**

   **Risk Policies** (from `business/policy.yaml`):
   - Min PnL threshold: `pnl_usd >= $500`
   - Max exposure: `position_size <= $5M per asset`
   - Max slippage: `slippage_bps <= 100` (1%)
   - Min success probability: `prob >= 0.80`

   **Sequencer Health Filters**:
   - Read sequencer health from beta_dataplane features
   - Skip chains if `sequencer_status == degraded || down`
   - Only execute on chains with `status == healthy`

   **Asset Filters**:
   - Check against `business/approved_assets.yaml`
   - Only trade whitelisted assets

2. **Scoring & Selection**:
   - Rank by: `score = pnl_usd * success_prob / risk_score`
   - Select top N candidates (configurable)
   - Respect position limits per asset

3. **Optimal Sizing**:
   - Don't overshoot available liquidity (from beta_dataplane)
   - Consider gas cost as % of profit
   - Respect flash loan caps (from beta_dataplane)

### **Integration Tests:**
- Mock evaluations → correct filtering
- Multiple candidates → best selection
- Policy violations → rejected

### **Deliverable:**
System chooses **if** and **how much** to trade, given evaluated candidates from beta_dataplane data.

---

## **Phase 4 – Intent Builder** (1 week)

### **Objective:** Standardize trade specifications for Orchestration

### **Tasks:**
1. **Implement `intent_builder.rs`:**

   **Add Execution Details**:
   - `minOut` values (slippage protection from simulation)
   - `maxFeeBps` limits (from beta_dataplane bridge/AMM fees)
   - `deadline` timestamps (based on beta_dataplane sequencer health)
   - Correlation `intent_id` (UUID v4)

   **Build Trade Legs**:
   ```rust
   legs: [
     {
       domain: Chain::Arbitrum,
       action: TradeAction::Swap,
       protocol: "uniswap_v3",
       asset_in: "USDC",
       asset_out: "WETH",
       amount_in: "100000.0",
       min_amount_out: "54.5", // From simulation
       max_fee_bps: 30,
       deadline: now + 30s
     },
     // ... more legs
   ]
   ```

2. **Serialize TradeIntent**:
   - Protobuf for efficient wire format
   - JSON for debugging/logging

3. **Define Orchestration Contract**:
   - Document TradeIntent schema
   - Version field for evolution
   - Validation rules

### **End-to-End Test:**
Candidate (from beta_dataplane features) → Evaluation → Decision → Intent

### **Deliverable:**
Validated `TradeIntent` objects ready for Orchestration execution.

---

## **Phase 5 – Feedback & Learning** (2 weeks)

### **Objective:** Close the loop and improve accuracy

### **Tasks:**
1. **Implement `feedback.rs`:**

   **Ingest Execution Receipts**:
   - From Orchestration layer (after trade execution)
   - Contains: `intent_id`, actual PnL, actual slippage, actual gas

   **Compare Predicted vs Actual**:
   ```rust
   predicted: {pnl: $1250, slippage: 0.5%, gas: $45}
   actual:    {pnl: $1180, slippage: 0.7%, gas: $52}
   error:     {pnl: -5.6%, slippage: +40%, gas: +15.6%}
   ```

2. **Maintain Error Models**:
   - Slippage bias by protocol (Uniswap vs Curve vs Balancer)
   - Gas variance by chain
   - Bridge latency distribution
   - Success rate by strategy

3. **Adaptive Adjustments**:
   - Update simulator coefficients
   - Adjust safety margins
   - Refine success probability estimates

4. **Logging**:
   - Persist all predictions + realizations to database
   - Enable research and model improvements

### **Deliverable:**
Intelligence learns continuously from **real execution data** and improves forecasts.

---

## **Phase 6 – Production Hardening** (3 weeks)

### **Objective:** Make Intelligence robust at scale

### **Tasks:**
1. **Performance Optimization**:
   - Multi-threaded feature ingestion from beta_dataplane
   - Parallel candidate detection across chains
   - Async simulation pipeline
   - Batched decision making

2. **Backpressure Handling**:
   - Don't overwhelm simulator if beta_dataplane sends high volume
   - Queue management with priority (high-spread opportunities first)
   - Graceful degradation under load

3. **State Consistency**:
   - Snapshot locking: Guarantee block-consistent `MarketState` during simulation
   - Handle beta_dataplane reorgs (rare but possible)
   - Atomic state updates

4. **Monitoring & Metrics**:
   - **PnL tracking**: Predicted vs realized per intent
   - **Hit rate**: % of intents that execute profitably
   - **Latency**: beta_dataplane feature → TradeIntent generated
   - **Throughput**: Candidates/second, Intents/second
   - Prometheus + Grafana dashboards

5. **Fail-Safes**:
   - **Stale feed detection**: Don't trade if beta_dataplane feed is >5s old
   - **Chain halt protection**: Auto-disable if beta_dataplane reports sequencer down
   - **Liquidity checks**: Never exceed available liquidity from beta_dataplane
   - **Circuit breakers**: Pause if consecutive failures

6. **Testing**:
   - Fuzz testing with chaotic beta_dataplane feeds
   - Chaos engineering (latency spikes, missing features)
   - Load testing (1000+ candidates/second)

### **Deliverable:**
24/7 production-ready Intelligence layer consuming **beta_dataplane** feeds.

---

## **Phase 7 – Advanced Features** (ongoing)

### **Objective:** Extend beyond MVP arbitrage

### **Features:**
1. **Multi-Strategy Support**:
   - Plug-in detector architecture
   - Enable/disable strategies via `business/` configs
   - Strategy-specific simulation models

2. **Advanced Decision Models**:
   - Bayesian probability updates
   - Reinforcement learning for strategy selection
   - Portfolio optimization (multiple concurrent trades)

3. **Pre-Trade Simulation**:
   - Backtest against beta_dataplane Parquet archives
   - Risk forecasting before execution
   - Historical performance analysis

4. **Cross-Intent Scheduling**:
   - Optimize across multiple simultaneous opportunities
   - Consider gas bundling (EIP-4844 blobs)
   - Flash loan pooling

5. **ML Enhancements**:
   - Neural networks for slippage prediction
   - LSTM for latency forecasting
   - Ensemble models for success probability

### **Deliverable:**
Continuously evolving "alpha machine brain" using **beta_dataplane** as sensory input.

---

## 📊 **Phase Summary**

| Phase | Duration | Status | Key Integration Point |
|-------|----------|--------|----------------------|
| **Phase 0** | 1 week | ✅ COMPLETE | Foundation ready |
| **Phase 1** | 2 weeks | 🔄 NEXT | Consume beta_dataplane Kafka topics |
| **Phase 2** | 3 weeks | 📋 PENDING | Use beta_dataplane depth curves, gas data |
| **Phase 3** | 2 weeks | 📋 PENDING | Apply business/ risk policies |
| **Phase 4** | 1 week | 📋 PENDING | Output to Orchestration |
| **Phase 5** | 2 weeks | 📋 PENDING | Learn from execution feedback |
| **Phase 6** | 3 weeks | 📋 PENDING | Production @ scale |
| **Phase 7** | Ongoing | 📋 PLANNED | Advanced alpha generation |

**Total Timeline: ~14 weeks to production-grade Intelligence**

---

## 🔗 **Key Integration Points**

### **1. Beta Dataplane → Intelligence** (Phase 1)
```rust
// Subscribe to beta_dataplane Kafka topics
consumer.subscribe(&["qenus.beta.features"])?;

// Or query via gRPC
let client = BetaDataplaneClient::connect("http://localhost:50053").await?;
let features = client.get_latest_features(chain).await?;
```

**What Intelligence Gets:**
- **AMM Features**: `pool_address`, `mid_price`, `liquidity`, `depth` (slippage curves)
- **Gas Features**: `base_fee`, `priority_fee`, `gas_used_ratio`
- **Bridge Features**: `liquidity`, `fee_bps`, `settlement_time_estimate`
- **Flash Loan Features**: `available_liquidity`, `fee_bps`, `max_loan_amount`

### **2. Business → Intelligence** (Phase 1)
```yaml
# business/strategies/triangle_arb.yaml
name: triangle_arb
enabled: true
min_spread_bps: 10
watchlist_assets: [USDC, WETH, USDT, WBTC]
enabled_chains: [ethereum, arbitrum, optimism, base]
```

**What Intelligence Reads:**
- Strategy definitions
- Risk thresholds
- Asset whitelists
- Chain preferences

### **3. Intelligence → Orchestration** (Phase 4)
```rust
// Intelligence outputs TradeIntent
let intent = TradeIntent {
    intent_id: Uuid::new_v4(),
    strategy: "triangle_arb",
    size_usd: 250_000.0,
    expected_pnl_usd: 1_250.0,
    legs: vec![/* execution steps */],
    ttl_seconds: 30,
    // ... from simulation using beta_dataplane data
};

// Orchestration executes
orchestrator.execute(intent).await?;
```

---

## 🎯 **Success Criteria by Phase**

### **Phase 1 Success:**
- ✅ Can consume all beta_dataplane Kafka topics
- ✅ Detects 10+ real arbitrage candidates per hour
- ✅ Uses business/ strategy configs correctly

### **Phase 2 Success:**
- ✅ Simulation accuracy >95% (vs manual calculation)
- ✅ Uses real beta_dataplane depth curves for slippage
- ✅ Gas cost estimation within 10% of reality

### **Phase 3 Success:**
- ✅ Correctly applies all risk policies from business/
- ✅ Rejects unprofitable opportunities
- ✅ Respects liquidity limits from beta_dataplane

### **Phase 4 Success:**
- ✅ Generates valid TradeIntent objects
- ✅ All fields populated from beta_dataplane + simulation
- ✅ Orchestration can parse and validate

### **Phase 5 Success:**
- ✅ Tracks predicted vs actual for 100+ trades
- ✅ Model error < 10% after learning period
- ✅ Continuous improvement visible in metrics

### **Phase 6 Success:**
- ✅ 24/7 uptime consuming beta_dataplane feeds
- ✅ <100ms latency (feature received → intent generated)
- ✅ Handles 1000+ beta_dataplane features/second
- ✅ Zero critical failures in 1-week test

---

## 🚀 **Getting Started (Phase 1)**

### **Week 1: State Management**
```rust
// intelligence/src/state.rs
impl MarketState {
    async fn consume_beta_dataplane_features(&mut self) {
        // Connect to beta_dataplane Kafka
        // Subscribe to qenus.beta.features
        // Update rolling state
    }
}
```

### **Week 2: Detection**
```rust
// intelligence/src/detectors.rs
impl TriangleArbDetector {
    fn detect(&self, state: &MarketState, config: &StrategyConfig) -> Vec<Candidate> {
        // Use beta_dataplane prices to find spreads
        // Check against business/ thresholds
        // Emit candidates
    }
}
```

---

## 📝 **Notes**

### **Why Beta Dataplane (not dataplane)?**
- ✅ **beta_dataplane** is **fully operational** and extracting live data NOW
- ✅ Uses RPC endpoints (slower but working)
- ❌ **dataplane/reth_fork** is **incomplete** - requires months of additional work
- ❌ Client forks are optimization for later

### **Migration Path:**
1. Build Intelligence with **beta_dataplane** (this plan)
2. Validate and generate revenue
3. Later: swap in **dataplane** (reth fork) for <1s latency
4. Intelligence code stays the same (same Feature schema)

---

**The Intelligence Layer will make Qenus profitable using beta_dataplane data!** 🧠💰


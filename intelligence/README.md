# Qenus Intelligence Layer

**The Brain of the Arbitrage System**

## Overview

The Intelligence Layer is the **decision-making brain** that combines:
1. **Live market data** from `beta_dataplane` (prices, liquidity, gas)
2. **Strategy configs** from `business` module (what to trade, risk limits)

And produces:
- **TradeIntent** objects for the Orchestration layer to execute

**Note:** The `dataplane` folder (with reth_fork) is for future optimization with direct client access. Currently NOT used - we use `beta_dataplane` which is fully operational.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Intelligence Layer                          │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Dataplane Features (Kafka/gRPC)                             │
│          ↓                                                    │
│    ┌──────────┐                                              │
│    │  State   │ ← Maintains rolling market state            │
│    └────┬─────┘                                              │
│         │                                                     │
│         ↓                                                     │
│    ┌──────────┐                                              │
│    │Detectors │ ← Finds candidate opportunities             │
│    └────┬─────┘                                              │
│         │                                                     │
│         ↓                                                     │
│    ┌──────────┐                                              │
│    │Simulator │ ← Models costs, slippage, probability       │
│    └────┬─────┘                                              │
│         │                                                     │
│         ↓                                                     │
│    ┌──────────┐                                              │
│    │ Decision │ ← Applies risk policy, selects best         │
│    └────┬─────┘                                              │
│         │                                                     │
│         ↓                                                     │
│    ┌──────────┐                                              │
│    │ Intent   │ ← Builds executable TradeIntent             │
│    │ Builder  │                                              │
│    └────┬─────┘                                              │
│         │                                                     │
│         ↓                                                     │
│   TradeIntent → Orchestration Layer                          │
│                                                               │
│    ┌──────────┐                                              │
│    │ Feedback │ ← Learns from execution results             │
│    └──────────┘                                              │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## Core Modules

### **state.rs** - Market State Manager
Maintains rolling state from dataplane features.

**API:**
- `get_price(chain, asset)` → Current mid-price
- `get_depth(chain, asset, size)` → Slippage curve
- `get_bridge_fee(chain, asset, size)` → Bridge costs
- `get_flashloan_liquidity(chain, asset)` → Available liquidity

### **detectors.rs** - Opportunity Detectors
Finds arbitrage candidates driven by strategy configs.

**Strategies:**
- Triangle arbitrage (L2 → L1 → L2)
- DEX arbitrage (Uniswap vs Curve vs Balancer)
- Cross-chain arbitrage (same asset, different chains)
- Flash loan arbitrage (borrow → trade → repay)

**Output:** `Vec<Candidate>` with spreads and execution paths

### **simulator.rs** - Execution Simulator
Models costs and probabilities for each candidate.

**Simulates:**
- AMM slippage (using depth curves from dataplane)
- Gas costs (in USD)
- Bridge fees + latency penalties
- Flash loan fees and caps
- Net PnL calculation

**Output:** `EvaluationResult` with profit, optimal size, success probability

### **decision.rs** - Decision Engine
Applies risk policies and selects best opportunities.

**Filters:**
- Minimum profit threshold
- Risk budget limits
- Chain health (skip degraded sequencers)
- Asset whitelist/blacklist

**Output:** Selected `EvaluationResult` for execution

### **intent_builder.rs** - Trade Intent Builder
Converts plan into fully specified TradeIntent.

**Adds:**
- minOut values (slippage protection)
- Deadlines and timeouts
- maxFeeBps limits
- Correlation IDs for tracking

**Output:** `TradeIntent` for Orchestration layer

### **feedback.rs** - Learning System
Reconciles predicted vs actual results.

**Tracks:**
- Predicted vs realized slippage
- Predicted vs realized gas costs
- Success/failure rates by strategy
- Model error distributions

**Updates:** Simulation models for improved accuracy

## Data Flow

```
┌──────────────────┐          ┌────────────────┐
│ beta_dataplane   │          │   business/    │
│                  │          │                │
│ • AMM prices     │          │ • Strategies   │
│ • Gas data       │          │ • Risk limits  │
│ • Flash loans    │          │ • Asset lists  │
│ • Bridge fees    │          │ • Policies     │
└────────┬─────────┘          └────────┬───────┘
         │ Features                    │ Configs
         │ (Kafka/gRPC)                │ (YAML files)
         └────────┬────────────────────┘
                  ↓
         ┌─────────────────┐
         │  Intelligence   │
         │                 │
         │ 1. state.rs     │ ← Maintains market state
         │ 2. detectors.rs │ ← Finds candidates
         │ 3. simulator.rs │ ← Calculates PnL
         │ 4. decision.rs  │ ← Applies risk policy
         │ 5. intent_builder│ ← Creates TradeIntent
         └────────┬────────┘
                  │ TradeIntent
                  ↓
         ┌─────────────────┐
         │  Orchestration  │ → Executes on-chain
         └────────┬────────┘
                  │ Results
                  ↓
         ┌─────────────────┐
         │ 6. feedback.rs  │ ← Learns from execution
         └─────────────────┘
```

## Key Types

### **TradeIntent** (Output)
```rust
{
  intent_id: UUID,
  strategy: "triangle_arb",
  asset: "USDC",
  size_usd: 100000.0,
  expected_pnl_usd: 250.0,
  net_bps: 25,
  success_prob: 0.92,
  legs: [
    {domain: "arbitrum", action: "swap", ...},
    {domain: "ethereum", action: "bridge", ...},
    {domain: "optimism", action: "swap", ...}
  ],
  ttl_seconds: 30
}
```

### **Candidate** (Internal)
```rust
{
  strategy: "dex_arb",
  asset: "WETH",
  spread_bps: 45.0,
  legs: [("ethereum", "uniswap"), ("ethereum", "curve")],
  confidence: 0.85
}
```

## Configuration

Strategies defined in `strategies/*.yaml`:

```yaml
name: triangle_arb
enabled: true
min_profit_usd: 50.0
min_profit_bps: 10.0
max_position_usd: 500000.0
approved_assets: [USDC, WETH, USDT]
approved_chains: [ethereum, arbitrum, optimism, base]
risk_limits:
  max_slippage_bps: 100
  max_gas_pct: 50
  max_bridge_latency_secs: 300
  min_success_prob: 0.8
```

## Status

🚧 **IN DEVELOPMENT**

- ✅ Project structure created
- ✅ Core types defined
- ✅ Module skeleton ready
- 🔄 State management (TODO)
- 🔄 Detectors (TODO)
- 🔄 Simulator (TODO)
- 🔄 Decision engine (TODO)
- 🔄 Intent builder (TODO)
- 🔄 Feedback system (TODO)

## Next Steps

1. Implement `state.rs` - consume dataplane features
2. Implement `detectors.rs` - find arbitrage candidates
3. Implement `simulator.rs` - evaluate profitability
4. Implement `decision.rs` - apply risk policies
5. Implement `intent_builder.rs` - create trade intents
6. Implement `feedback.rs` - learning loop

---

**The Intelligence Layer is where Qenus turns data into alpha.** 🧠💰


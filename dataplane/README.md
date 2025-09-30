# Qenus Dataplane

The **Dataplane** is the **sensory nervous system** of Qenus.
It is responsible for continuously ingesting, normalizing, and publishing live on-chain data from Ethereum L1 and selected flash-loan–enabled L2 rollups.
The Intelligence Layer consumes this stream in real time to detect arbitrage opportunities.

---

## 🧭 Scope

Supported domains:
- **Ethereum L1** via a lean fork of [Reth](https://github.com/paradigmxyz/reth).
- **Flash-loan enabled L2s only**:
  - Arbitrum
  - Optimism
  - Base

Other chains (e.g., zkSync, Scroll, Linea) are excluded until robust flash-loan support is available.

---

## 🧩 Directory Structure

```
dataplane/
├── reth_fork/          # Ethereum L1 lean fork for raw block + tx data
├── l2_observers/       # Arbitrum, Optimism, Base live chain observers
├── feature_extractors/ # Normalize raw logs → usable features
├── feeds/              # Streams features to Intelligence (Kafka, gRPC, Parquet)
├── tests/              # Validation & integration tests
└── README.md           # This document
```

---

## ⚙️ Components

### `reth_fork/`
- Stripped-down Ethereum client (based on Reth).
- Keeps only execution engine, block sync, receipts, and logs.
- Augmented with feature extractors for:
  - AMM pools (Uniswap v3, Curve).
  - Canonical bridge contracts.
  - Gas base/priority fee models.

### `l2_observers/`
- Lightweight agents that connect to supported rollups.
- Access sequencer-published blocks via RPC or follower nodes.
- Subscribe to events and state from:
  - **DEX pools** (Uniswap v3, Curve, Balancer, Velodrome, Camelot).
  - **Bridge contracts** (canonical + fast bridges).
  - **Flash-loan providers** (Aave v3 pools).
- Emit normalized features in the same schema as L1.

### `feature_extractors/`
- Convert raw contract state → actionable metrics:
  - **`amm.rs`**: mid-price, reserves, slippage depth curves.
  - **`bridge.rs`**: LP liquidity, fee curves, settlement latency estimates.
  - **`gas.rs`**: real-time base/priority fee model + latency distribution.
  - **`sequencer.rs`**: uptime, block interval variance, downtime alerts.

### `feeds/`
- Transport layer for delivering features to Intelligence:
  - **Kafka/Redpanda**: real-time streaming topics (e.g., `features.amm`, `features.bridge`).
  - **gRPC snapshot API**: block-consistent state for deterministic simulation.
  - **Arrow/Parquet writer**: high-fidelity historical logs for backtests and ML.

### `tests/`
- Ensure correctness of feature extraction and feed delivery.
- Includes unit tests, integration tests against known blocks, and schema conformance tests.

---

## 🧮 Feature Schema

All observers and extractors emit normalized events in the same schema.
Example (JSON-encoded, also defined as Rust structs and protobuf):

```json
{
  "block_number": 20000000,
  "chain": "arbitrum",
  "pool": "UniswapV3:USDC/WETH:0.05%",
  "reserves": {"USDC": 25000000, "WETH": 15000},
  "mid_price": 1668.42,
  "depth": {
    "100k": {"slippage_bps": 2.1},
    "1m": {"slippage_bps": 8.7}
  },
  "flash_loan": {
    "provider": "aave_v3",
    "asset": "USDC",
    "cap": 20000000,
    "fee_bps": 9
  },
  "gas": {"base": 0.2, "priority": 0.01},
  "bridge_fee_bps": 5,
  "sequencer_health": "healthy",
  "timestamp": 1699999999
}
```

## 🎯 Design Goals

- **Freshness**: publish features within ≤1s of block arrival.
- **Consistency**: normalized schema across L1 and all supported L2s.
- **Scalability**: handle high-throughput event streams across multiple chains.
- **Backtestability**: all published features logged in Arrow/Parquet for replay.
- **Extensibility**: new rollups or providers can be added by dropping in an observer.

## 🚀 Roadmap

- [ ] Ethereum L1 via lean Reth fork.
- [ ] L2 observers: Arbitrum, Optimism, Base.
- [ ] Feature extractors for AMMs, bridges, gas, sequencer health.
- [ ] Expanded bridge metrics (Across, Hop, Stargate).
- [ ] Enhanced flash-loan monitoring across all providers.
- [ ] Latency-aware ordering model for sequencer vs L1 finality.

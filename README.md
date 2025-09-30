# Qenus

High-performance DeFi arbitrage system with real-time on-chain data ingestion and intelligent decision-making.

## Architecture

Qenus consists of three main layers:

1. **Dataplane**: Sensory system that ingests on-chain data from Ethereum L1 and L2s (Arbitrum, Optimism, Base)
2. **Intelligence**: Real-time decision engine for arbitrage opportunity detection
3. **Business**: Execution layer for trade execution and risk management

## Project Structure

```
qenus/
├── dataplane/           # Full dataplane with client forks (future optimization)
│   └── reth_fork/      # Lean Reth fork for L1 ingestion
├── beta_dataplane/     # Beta version using RPC endpoints
│   ├── src/
│   │   ├── providers/  # Multi-RPC client management
│   │   ├── extractors/ # Feature extraction (AMMs, bridges, gas)
│   │   ├── feeds/      # Data publishing (Kafka, gRPC, Parquet)
│   │   ├── optimization/ # Caching, batching, prediction
│   │   └── monitoring/ # Health checks and metrics
│   └── config/         # Configuration files
├── intelligence/       # Decision engine (future phase)
└── business/          # Execution layer (future phase)
```

## Current Status

### Beta Dataplane (Active Development)

The beta dataplane is designed for rapid deployment and revenue generation using RPC endpoints:

- ✅ **Phase 1**: Core infrastructure with configuration management
- ✅ **Phase 2**: Multi-RPC provider management with failover
- ✅ **Phase 3**: Feature extractors for AMMs, bridges, gas, flash loans
- ✅ **Phase 4**: Performance optimization (caching, batching, prediction)
- 🚧 **Phase 5**: Data feeds (Kafka, gRPC, Parquet) - In Progress
- 📋 **Phase 6**: Production monitoring
- 📋 **Phase 7**: Integration testing
- 📋 **Phase 8**: Production deployment
- 📋 **Phase 9**: Live validation

### Full Dataplane (Future Optimization)

The full dataplane will use client forks for sub-1-second latency:

- 🔜 Lean Reth fork for Ethereum L1
- 🔜 L2 client forks for Arbitrum, Optimism, Base
- 🔜 MEV-Boost integration for pre-confirmation data
- 🔜 Direct state access and execution

## Features

### Supported Chains
- Ethereum L1
- Arbitrum
- Optimism
- Base

### Supported Protocols
- **AMMs**: Uniswap V3, Curve, Balancer
- **Bridges**: Canonical bridges, Hop Protocol, Across
- **Flash Loans**: Aave V3, Balancer
- **Gas**: Real-time pricing and prediction

### Performance
- Multi-RPC failover and load balancing
- Intelligent caching with TTL management
- Batch processing for efficiency
- Predictive pre-fetching
- Data compression (Snappy, Gzip, Zstd)

## Getting Started

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- RPC endpoints (Ankr, Alchemy, Infura, etc.)

### Installation

```bash
# Clone the repository
git clone https://github.com/eliseokya/ques.git
cd qenus

# Build the beta dataplane
cargo build --package qenus-beta-dataplane --release
```

### Configuration

1. Copy the environment template:
```bash
cp beta_dataplane/config/environment.example beta_dataplane/config/.env
```

2. Set up your RPC API keys in `.env`

3. Configure chains and providers in `beta_dataplane/config/beta-dataplane.toml`

### Running

```bash
# Run with default configuration
cargo run --package qenus-beta-dataplane

# Run with custom configuration
cargo run --package qenus-beta-dataplane -- --config custom-config.toml

# Test provider connections
cargo run --package qenus-beta-dataplane -- test-providers

# Check API key status
cargo run --package qenus-beta-dataplane -- setup-keys
```

## Development

### Building

```bash
# Build all workspace members
cargo build --workspace

# Build in release mode
cargo build --workspace --release

# Check for compilation errors
cargo check --workspace
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run beta dataplane tests
cargo test --package qenus-beta-dataplane

# Run live extraction tests (requires API keys)
cargo test --package qenus-beta-dataplane test_live_extraction -- --nocapture
```

### Documentation

```bash
# Generate and open documentation
cargo doc --workspace --open

# See beta_dataplane/docs/ for detailed documentation
```

## Roadmap

### Q4 2025
- ✅ Beta dataplane with RPC endpoints
- 🚧 Data feeds implementation
- 🔜 Production monitoring and alerting
- 🔜 Integration testing suite
- 🔜 Docker deployment setup

### Q1 2026
- Intelligence layer (arbitrage detection)
- Business layer (execution)
- Live trading with beta dataplane
- Revenue generation

### Q2 2026
- Full dataplane with client forks
- MEV-Boost integration
- Sub-1-second latency optimization
- Enhanced backtesting infrastructure

## License

Proprietary - All rights reserved

## Contact

For questions or collaboration opportunities, please open an issue.

---

**Note**: This is a high-performance trading system. Use at your own risk. Past performance does not guarantee future results.

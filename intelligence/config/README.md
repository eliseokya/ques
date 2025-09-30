# Intelligence Layer Configuration

## Overview

The Intelligence Layer can load strategy configurations from three sources (in priority order):

1. **Business Module** (future) - `BUSINESS_MODULE_PATH` environment variable
2. **Config File** - `INTELLIGENCE_CONFIG_PATH` environment variable
3. **Defaults** - Hardcoded fallback configurations

## Configuration Structure

### Strategy Configs

Each strategy has:
- **name**: Unique identifier
- **enabled**: Whether the strategy is active
- **min_profit_usd**: Minimum profit in USD to execute
- **min_profit_bps**: Minimum profit in basis points (1 bp = 0.01%)
- **max_position_usd**: Maximum position size
- **approved_assets**: Whitelist of tradeable assets
- **approved_chains**: Whitelist of chains to trade on
- **risk_limits**: Risk management thresholds

### Risk Limits

- **max_slippage_bps**: Maximum acceptable slippage
- **max_gas_pct**: Maximum gas cost as % of profit
- **max_bridge_latency_secs**: Maximum bridge settlement time
- **min_success_prob**: Minimum success probability

## Usage

### Development (Mock Mode)

```bash
# Uses default configs with mock data
cargo run -p qenus-intelligence
```

### With Custom Config

```bash
export INTELLIGENCE_CONFIG_PATH=./intelligence/config/intelligence.yaml
cargo run -p qenus-intelligence
```

### With Business Module (Future)

```bash
# When business module is built
export BUSINESS_MODULE_PATH=./business
cargo run -p qenus-intelligence

# This will load strategies from:
# - business/strategies/triangle_arb.yaml
# - business/strategies/dex_arb.yaml
# - etc.
```

## Strategy Examples

### Triangle Arbitrage

Cross-chain arbitrage via bridges:
- Buy asset on Chain A
- Bridge to Chain B
- Sell on Chain B
- Net profit after bridge fees & gas

### DEX Arbitrage

Same-chain price discrepancies:
- Buy on Uniswap
- Sell on Curve
- Net profit after swap fees & gas

## Future Integration

When the `business/` module is created, it will contain:

```
business/
├── strategies/
│   ├── triangle_arb.yaml
│   ├── dex_arb.yaml
│   ├── flash_arb.yaml
│   └── ...
├── policies/
│   ├── risk_policy.yaml
│   └── exposure_limits.yaml
└── approved_assets.yaml
```

The intelligence layer will automatically load these configs when `BUSINESS_MODULE_PATH` is set.


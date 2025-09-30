//! Contract utilities and ABI definitions
//!
//! Provides contract ABIs and helper functions for interacting with
//! smart contracts via RPC.

use ethers::abi::{Abi, Token};
use ethers::types::{H160, U256, Bytes};
use once_cell::sync::Lazy;
use serde_json::json;

use crate::{Result, BetaDataplaneError};

/// Uniswap V3 Pool contract ABI (slot0 function)
pub static UNISWAP_V3_POOL_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_value(json!([
        {
            "inputs": [],
            "name": "slot0",
            "outputs": [
                {"internalType": "uint160", "name": "sqrtPriceX96", "type": "uint160"},
                {"internalType": "int24", "name": "tick", "type": "int24"},
                {"internalType": "uint16", "name": "observationIndex", "type": "uint16"},
                {"internalType": "uint16", "name": "observationCardinality", "type": "uint16"},
                {"internalType": "uint16", "name": "observationCardinalityNext", "type": "uint16"},
                {"internalType": "uint8", "name": "feeProtocol", "type": "uint8"},
                {"internalType": "bool", "name": "unlocked", "type": "bool"}
            ],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "inputs": [],
            "name": "liquidity",
            "outputs": [{"internalType": "uint128", "name": "", "type": "uint128"}],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "inputs": [],
            "name": "token0",
            "outputs": [{"internalType": "address", "name": "", "type": "address"}],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "inputs": [],
            "name": "token1",
            "outputs": [{"internalType": "address", "name": "", "type": "address"}],
            "stateMutability": "view",
            "type": "function"
        }
    ]))
    .expect("Valid Uniswap V3 Pool ABI")
});

/// ERC20 contract ABI
pub static ERC20_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_value(json!([
        {
            "constant": true,
            "inputs": [{"name": "_owner", "type": "address"}],
            "name": "balanceOf",
            "outputs": [{"name": "balance", "type": "uint256"}],
            "type": "function"
        },
        {
            "constant": true,
            "inputs": [],
            "name": "decimals",
            "outputs": [{"name": "", "type": "uint8"}],
            "type": "function"
        },
        {
            "constant": true,
            "inputs": [],
            "name": "symbol",
            "outputs": [{"name": "", "type": "string"}],
            "type": "function"
        },
        {
            "constant": true,
            "inputs": [],
            "name": "totalSupply",
            "outputs": [{"name": "", "type": "uint256"}],
            "type": "function"
        }
    ]))
    .expect("Valid ERC20 ABI")
});

/// Curve Pool contract ABI
pub static CURVE_POOL_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_value(json!([
        {
            "name": "get_virtual_price",
            "outputs": [{"type": "uint256", "name": ""}],
            "inputs": [],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "name": "balances",
            "outputs": [{"type": "uint256", "name": ""}],
            "inputs": [{"type": "uint256", "name": "i"}],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "name": "coins",
            "outputs": [{"type": "address", "name": ""}],
            "inputs": [{"type": "uint256", "name": "i"}],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "name": "get_dy",
            "outputs": [{"type": "uint256", "name": ""}],
            "inputs": [
                {"type": "int128", "name": "i"},
                {"type": "int128", "name": "j"},
                {"type": "uint256", "name": "dx"}
            ],
            "stateMutability": "view",
            "type": "function"
        }
    ]))
    .expect("Valid Curve Pool ABI")
});

/// Balancer Vault ABI
pub static BALANCER_VAULT_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_value(json!([
        {
            "name": "getPoolTokens",
            "outputs": [
                {"type": "address[]", "name": "tokens"},
                {"type": "uint256[]", "name": "balances"},
                {"type": "uint256", "name": "lastChangeBlock"}
            ],
            "inputs": [{"type": "bytes32", "name": "poolId"}],
            "stateMutability": "view",
            "type": "function"
        }
    ]))
    .expect("Valid Balancer Vault ABI")
});

/// Aave V3 Pool ABI
pub static AAVE_V3_POOL_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_value(json!([
        {
            "name": "getReserveData",
            "outputs": [
                {"type": "uint256", "name": "configuration"},
                {"type": "uint128", "name": "liquidityIndex"},
                {"type": "uint128", "name": "currentLiquidityRate"},
                {"type": "uint128", "name": "variableBorrowIndex"},
                {"type": "uint128", "name": "currentVariableBorrowRate"},
                {"type": "uint128", "name": "currentStableBorrowRate"},
                {"type": "uint40", "name": "lastUpdateTimestamp"},
                {"type": "uint16", "name": "id"},
                {"type": "address", "name": "aTokenAddress"},
                {"type": "address", "name": "stableDebtTokenAddress"},
                {"type": "address", "name": "variableDebtTokenAddress"},
                {"type": "address", "name": "interestRateStrategyAddress"},
                {"type": "uint128", "name": "accruedToTreasury"},
                {"type": "uint128", "name": "unbacked"},
                {"type": "uint128", "name": "isolationModeTotalDebt"}
            ],
            "inputs": [{"type": "address", "name": "asset"}],
            "stateMutability": "view",
            "type": "function"
        }
    ]))
    .expect("Valid Aave V3 Pool ABI")
});

/// L1 Bridge ABI (for Arbitrum/Optimism/Base canonical bridges)
pub static L1_BRIDGE_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_value(json!([
        {
            "name": "l2TokenBridge",
            "outputs": [{"type": "address", "name": ""}],
            "inputs": [],
            "stateMutability": "view",
            "type": "function"
        }
    ]))
    .expect("Valid L1 Bridge ABI")
});

/// Contract registry for known contracts
pub struct ContractRegistry {
    // Contract addresses and metadata
}

impl ContractRegistry {
    /// Create a new contract registry
    pub fn new() -> Self {
        Self {}
    }

    /// Get Uniswap V3 factory address for a chain
    pub fn get_uniswap_v3_factory(chain: crate::Chain) -> H160 {
        match chain {
            crate::Chain::Ethereum => "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse().unwrap(),
            crate::Chain::Arbitrum => "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse().unwrap(),
            crate::Chain::Optimism => "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse().unwrap(),
            crate::Chain::Base => "0x33128a8fC17869897dcE68Ed026d694621f6FDfD".parse().unwrap(),
        }
    }

    /// Get Curve registry address
    pub fn get_curve_registry(chain: crate::Chain) -> Option<H160> {
        match chain {
            crate::Chain::Ethereum => Some("0x90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5".parse().unwrap()),
            _ => None, // Curve primarily on Ethereum
        }
    }

    /// Get Balancer vault address
    pub fn get_balancer_vault(chain: crate::Chain) -> H160 {
        match chain {
            crate::Chain::Ethereum => "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(),
            crate::Chain::Arbitrum => "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(),
            crate::Chain::Optimism => "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(),
            crate::Chain::Base => "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(),
        }
    }

    /// Get Aave V3 Pool address
    pub fn get_aave_v3_pool(chain: crate::Chain) -> Option<H160> {
        match chain {
            crate::Chain::Ethereum => Some("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".parse().unwrap()),
            crate::Chain::Arbitrum => Some("0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse().unwrap()),
            crate::Chain::Optimism => Some("0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse().unwrap()),
            crate::Chain::Base => Some("0xA238Dd80C259a72e81d7e4664a9801593F98d1c5".parse().unwrap()),
        }
    }
}

impl Default for ContractRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// ABI manager for encoding/decoding contract calls
pub struct AbiManager;

impl AbiManager {
    /// Encode a function call
    pub fn encode_function_call(
        abi: &Abi,
        function_name: &str,
        params: &[Token],
    ) -> Result<Bytes> {
        let function = abi.function(function_name)
            .map_err(|e| BetaDataplaneError::internal(format!(
                "Function {} not found in ABI: {}", function_name, e
            )))?;

        let encoded = function.encode_input(params)
            .map_err(|e| BetaDataplaneError::internal(format!(
                "Failed to encode function call: {}", e
            )))?;

        Ok(Bytes::from(encoded))
    }

    /// Decode function output
    pub fn decode_function_output(
        abi: &Abi,
        function_name: &str,
        output: &[u8],
    ) -> Result<Vec<Token>> {
        let function = abi.function(function_name)
            .map_err(|e| BetaDataplaneError::internal(format!(
                "Function {} not found in ABI: {}", function_name, e
            )))?;

        let decoded = function.decode_output(output)
            .map_err(|e| BetaDataplaneError::internal(format!(
                "Failed to decode function output: {}", e
            )))?;

        Ok(decoded)
    }

    /// Encode slot0() call for Uniswap V3
    pub fn encode_uniswap_slot0_call() -> Result<Bytes> {
        Self::encode_function_call(&UNISWAP_V3_POOL_ABI, "slot0", &[])
    }

    /// Decode slot0() output
    pub fn decode_uniswap_slot0_output(output: &[u8]) -> Result<UniswapV3Slot0> {
        let tokens = Self::decode_function_output(&UNISWAP_V3_POOL_ABI, "slot0", output)?;

        if tokens.len() != 7 {
            return Err(BetaDataplaneError::internal(
                "Invalid slot0 output length"
            ));
        }

        Ok(UniswapV3Slot0 {
            sqrt_price_x96: match &tokens[0] {
                Token::Uint(val) => *val,
                _ => return Err(BetaDataplaneError::internal("Invalid sqrtPriceX96 type")),
            },
            tick: match &tokens[1] {
                Token::Int(val) => {
                    // Tick is int24 - extract as i32
                    // For simplicity, take low bits (tick range is typically -887272 to 887272)
                    let low = val.low_u64();
                    if low > 0x7FFFFF { // Check if negative (int24 sign bit)
                        -((0x1000000 - low) as i32)
                    } else {
                        low as i32
                    }
                },
                _ => return Err(BetaDataplaneError::internal("Invalid tick type")),
            },
            observation_index: match &tokens[2] {
                Token::Uint(val) => val.as_u32() as u16,
                _ => return Err(BetaDataplaneError::internal("Invalid observationIndex type")),
            },
            observation_cardinality: match &tokens[3] {
                Token::Uint(val) => val.as_u32() as u16,
                _ => return Err(BetaDataplaneError::internal("Invalid observationCardinality type")),
            },
            observation_cardinality_next: match &tokens[4] {
                Token::Uint(val) => val.as_u32() as u16,
                _ => return Err(BetaDataplaneError::internal("Invalid observationCardinalityNext type")),
            },
            fee_protocol: match &tokens[5] {
                Token::Uint(val) => val.as_u32() as u8,
                _ => return Err(BetaDataplaneError::internal("Invalid feeProtocol type")),
            },
            unlocked: match &tokens[6] {
                Token::Bool(val) => *val,
                _ => return Err(BetaDataplaneError::internal("Invalid unlocked type")),
            },
        })
    }

    /// Encode liquidity() call for Uniswap V3
    pub fn encode_uniswap_liquidity_call() -> Result<Bytes> {
        Self::encode_function_call(&UNISWAP_V3_POOL_ABI, "liquidity", &[])
    }

    /// Decode liquidity() output
    pub fn decode_uniswap_liquidity_output(output: &[u8]) -> Result<U256> {
        let tokens = Self::decode_function_output(&UNISWAP_V3_POOL_ABI, "liquidity", output)?;

        if tokens.len() != 1 {
            return Err(BetaDataplaneError::internal("Invalid liquidity output length"));
        }

        match &tokens[0] {
            Token::Uint(val) => Ok(*val),
            _ => Err(BetaDataplaneError::internal("Invalid liquidity type")),
        }
    }

    /// Encode ERC20 decimals() call
    pub fn encode_erc20_decimals_call() -> Result<Bytes> {
        Self::encode_function_call(&ERC20_ABI, "decimals", &[])
    }

    /// Decode ERC20 decimals() output
    pub fn decode_erc20_decimals_output(output: &[u8]) -> Result<u8> {
        let tokens = Self::decode_function_output(&ERC20_ABI, "decimals", output)?;

        if tokens.len() != 1 {
            return Err(BetaDataplaneError::internal("Invalid decimals output length"));
        }

        match &tokens[0] {
            Token::Uint(val) => Ok(val.as_u32() as u8),
            _ => Err(BetaDataplaneError::internal("Invalid decimals type")),
        }
    }

    /// Encode ERC20 symbol() call
    pub fn encode_erc20_symbol_call() -> Result<Bytes> {
        Self::encode_function_call(&ERC20_ABI, "symbol", &[])
    }

    /// Decode ERC20 symbol() output
    pub fn decode_erc20_symbol_output(output: &[u8]) -> Result<String> {
        let tokens = Self::decode_function_output(&ERC20_ABI, "symbol", output)?;

        if tokens.len() != 1 {
            return Err(BetaDataplaneError::internal("Invalid symbol output length"));
        }

        match &tokens[0] {
            Token::String(val) => Ok(val.clone()),
            _ => Err(BetaDataplaneError::internal("Invalid symbol type")),
        }
    }

    // === Curve Contract Functions ===

    /// Encode get_virtual_price() call for Curve
    pub fn encode_curve_virtual_price_call() -> Result<Bytes> {
        Self::encode_function_call(&CURVE_POOL_ABI, "get_virtual_price", &[])
    }

    /// Decode get_virtual_price() output
    pub fn decode_curve_virtual_price_output(output: &[u8]) -> Result<U256> {
        let tokens = Self::decode_function_output(&CURVE_POOL_ABI, "get_virtual_price", output)?;

        if tokens.len() != 1 {
            return Err(BetaDataplaneError::internal("Invalid virtual_price output length"));
        }

        match &tokens[0] {
            Token::Uint(val) => Ok(*val),
            _ => Err(BetaDataplaneError::internal("Invalid virtual_price type")),
        }
    }

    /// Encode balances(i) call for Curve
    pub fn encode_curve_balances_call(index: u64) -> Result<Bytes> {
        Self::encode_function_call(&CURVE_POOL_ABI, "balances", &[Token::Uint(U256::from(index))])
    }

    /// Decode balances() output
    pub fn decode_curve_balances_output(output: &[u8]) -> Result<U256> {
        let tokens = Self::decode_function_output(&CURVE_POOL_ABI, "balances", output)?;

        if tokens.len() != 1 {
            return Err(BetaDataplaneError::internal("Invalid balances output length"));
        }

        match &tokens[0] {
            Token::Uint(val) => Ok(*val),
            _ => Err(BetaDataplaneError::internal("Invalid balances type")),
        }
    }

    /// Encode coins(i) call for Curve
    pub fn encode_curve_coins_call(index: u64) -> Result<Bytes> {
        Self::encode_function_call(&CURVE_POOL_ABI, "coins", &[Token::Uint(U256::from(index))])
    }

    /// Decode coins() output
    pub fn decode_curve_coins_output(output: &[u8]) -> Result<H160> {
        let tokens = Self::decode_function_output(&CURVE_POOL_ABI, "coins", output)?;

        if tokens.len() != 1 {
            return Err(BetaDataplaneError::internal("Invalid coins output length"));
        }

        match &tokens[0] {
            Token::Address(val) => Ok(*val),
            _ => Err(BetaDataplaneError::internal("Invalid coins type")),
        }
    }

    // === Balancer Contract Functions ===

    /// Encode getPoolTokens(poolId) call for Balancer
    pub fn encode_balancer_pool_tokens_call(pool_id: [u8; 32]) -> Result<Bytes> {
        Self::encode_function_call(&BALANCER_VAULT_ABI, "getPoolTokens", &[Token::FixedBytes(pool_id.to_vec())])
    }

    /// Decode getPoolTokens() output
    pub fn decode_balancer_pool_tokens_output(output: &[u8]) -> Result<BalancerPoolTokens> {
        let tokens = Self::decode_function_output(&BALANCER_VAULT_ABI, "getPoolTokens", output)?;

        if tokens.len() != 3 {
            return Err(BetaDataplaneError::internal("Invalid getPoolTokens output length"));
        }

        let token_addresses = match &tokens[0] {
            Token::Array(arr) => {
                arr.iter()
                    .map(|t| match t {
                        Token::Address(addr) => Ok(*addr),
                        _ => Err(BetaDataplaneError::internal("Invalid token address type")),
                    })
                    .collect::<Result<Vec<_>>>()?
            }
            _ => return Err(BetaDataplaneError::internal("Invalid tokens type")),
        };

        let balances = match &tokens[1] {
            Token::Array(arr) => {
                arr.iter()
                    .map(|t| match t {
                        Token::Uint(val) => Ok(*val),
                        _ => Err(BetaDataplaneError::internal("Invalid balance type")),
                    })
                    .collect::<Result<Vec<_>>>()?
            }
            _ => return Err(BetaDataplaneError::internal("Invalid balances type")),
        };

        let last_change_block = match &tokens[2] {
            Token::Uint(val) => val.as_u64(),
            _ => return Err(BetaDataplaneError::internal("Invalid lastChangeBlock type")),
        };

        Ok(BalancerPoolTokens {
            tokens: token_addresses,
            balances,
            last_change_block,
        })
    }

    // === Aave V3 Contract Functions ===

    /// Encode getReserveData(asset) call for Aave V3
    pub fn encode_aave_reserve_data_call(asset: H160) -> Result<Bytes> {
        Self::encode_function_call(&AAVE_V3_POOL_ABI, "getReserveData", &[Token::Address(asset)])
    }

    /// Decode getReserveData() output
    pub fn decode_aave_reserve_data_output(output: &[u8]) -> Result<AaveReserveData> {
        let tokens = Self::decode_function_output(&AAVE_V3_POOL_ABI, "getReserveData", output)?;

        if tokens.len() != 15 {
            return Err(BetaDataplaneError::internal("Invalid getReserveData output length"));
        }

        Ok(AaveReserveData {
            configuration: match &tokens[0] {
                Token::Uint(val) => *val,
                _ => return Err(BetaDataplaneError::internal("Invalid configuration type")),
            },
            liquidity_index: match &tokens[1] {
                Token::Uint(val) => *val,
                _ => return Err(BetaDataplaneError::internal("Invalid liquidityIndex type")),
            },
            a_token_address: match &tokens[8] {
                Token::Address(val) => *val,
                _ => return Err(BetaDataplaneError::internal("Invalid aTokenAddress type")),
            },
        })
    }
}

/// Uniswap V3 slot0 data
#[derive(Debug, Clone)]
pub struct UniswapV3Slot0 {
    pub sqrt_price_x96: U256,
    pub tick: i32,
    pub observation_index: u16,
    pub observation_cardinality: u16,
    pub observation_cardinality_next: u16,
    pub fee_protocol: u8,
    pub unlocked: bool,
}

/// Balancer pool tokens data
#[derive(Debug, Clone)]
pub struct BalancerPoolTokens {
    pub tokens: Vec<H160>,
    pub balances: Vec<U256>,
    pub last_change_block: u64,
}

/// Aave V3 reserve data
#[derive(Debug, Clone)]
pub struct AaveReserveData {
    pub configuration: U256,
    pub liquidity_index: U256,
    pub a_token_address: H160,
}

//! Ethereum-specific RPC provider implementation
//!
//! Handles Ethereum mainnet RPC connections with optimizations
//! for DeFi protocol monitoring and feature extraction.

use std::sync::Arc;
use tracing::{info, debug};
use ethers::types::{Block, Transaction, Log, Filter, TxHash, H160, U256};

use crate::{
    config::ProviderConfig,
    providers::multi_rpc::MultiRpcClient,
    Result, BetaDataplaneError,
};

/// Ethereum-specific RPC client
pub struct EthereumRpcClient {
    /// Multi-RPC client
    client: MultiRpcClient,
}

impl EthereumRpcClient {
    /// Create a new Ethereum RPC client
    pub async fn new(providers: Vec<ProviderConfig>) -> Result<Self> {
        info!("Initializing Ethereum RPC client with {} providers", providers.len());
        
        let client = MultiRpcClient::new(
            crate::Chain::Ethereum,
            providers,
            crate::config::ProviderSelectionStrategy::FastestFirst,
        ).await?;
        
        Ok(Self { client })
    }

    /// Get current block number
    pub async fn get_current_block(&self) -> Result<u64> {
        let block_number = self.client.get_block_number().await?;
        Ok(block_number.as_u64())
    }

    /// Get block with full transaction details
    pub async fn get_block_with_txs(&self, block_number: u64) -> Result<Option<Block<Transaction>>> {
        // TODO: Implement full transaction block retrieval
        // For now, return None as placeholder
        debug!("Getting block {} with transactions", block_number);
        Ok(None)
    }

    /// Get Uniswap V3 pool state
    pub async fn get_uniswap_v3_slot0(&self, pool_address: H160) -> Result<crate::utils::contracts::UniswapV3Slot0> {
        debug!("Getting Uniswap V3 slot0 for pool {}", pool_address);
        
        // Encode slot0() function call
        let calldata = crate::utils::contracts::AbiManager::encode_uniswap_slot0_call()?;
        
        // Create transaction for call
        use ethers::types::transaction::eip2718::TypedTransaction;
        use ethers::types::NameOrAddress;
        
        let mut tx = TypedTransaction::default();
        tx.set_to(NameOrAddress::Address(pool_address));
        tx.set_data(calldata);
        
        // Execute call
        let result = self.client.call(&tx, None).await?;
        
        // Decode result
        let slot0 = crate::utils::contracts::AbiManager::decode_uniswap_slot0_output(&result)?;
        
        debug!(
            pool = %pool_address,
            sqrt_price_x96 = %slot0.sqrt_price_x96,
            tick = slot0.tick,
            "Retrieved Uniswap V3 slot0"
        );
        
        Ok(slot0)
    }

    /// Get Uniswap V3 pool liquidity
    pub async fn get_uniswap_v3_liquidity(&self, pool_address: H160) -> Result<U256> {
        debug!("Getting Uniswap V3 liquidity for pool {}", pool_address);
        
        // Encode liquidity() function call
        let calldata = crate::utils::contracts::AbiManager::encode_uniswap_liquidity_call()?;
        
        // Create transaction for call
        use ethers::types::transaction::eip2718::TypedTransaction;
        use ethers::types::NameOrAddress;
        
        let mut tx = TypedTransaction::default();
        tx.set_to(NameOrAddress::Address(pool_address));
        tx.set_data(calldata);
        
        // Execute call
        let result = self.client.call(&tx, None).await?;
        
        // Decode result
        let liquidity = crate::utils::contracts::AbiManager::decode_uniswap_liquidity_output(&result)?;
        
        debug!(
            pool = %pool_address,
            liquidity = %liquidity,
            "Retrieved Uniswap V3 liquidity"
        );
        
        Ok(liquidity)
    }


    /// Get gas price information
    pub async fn get_gas_price_info(&self) -> Result<GasPriceInfo> {
        debug!("Getting gas price information");
        
        // Get current gas price
        let gas_price = self.client.get_gas_price().await?;
        
        // Try to get EIP-1559 data (base fee + priority fee)
        // Note: This requires eth_feeHistory which may not be supported by all clients
        let (base_fee, priority_fee) = match self.get_fee_history(1).await {
            Ok(history) => {
                let base = history.base_fee.unwrap_or(gas_price);
                let priority = history.priority_fee.unwrap_or(U256::from(2000000000u64));
                (Some(base), Some(priority))
            }
            Err(_) => {
                // Fallback: estimate from gas price
                let estimated_base = gas_price * 8 / 10; // ~80% of gas price
                let estimated_priority = gas_price - estimated_base;
                (Some(estimated_base), Some(estimated_priority))
            }
        };
        
        Ok(GasPriceInfo {
            gas_price,
            base_fee,
            priority_fee,
        })
    }

    /// Get fee history (for EIP-1559 chains)
    async fn get_fee_history(&self, block_count: u64) -> Result<FeeHistory> {
        // This would use eth_feeHistory RPC method
        // For now, return a simplified version based on current block
        let latest_block = self.client.get_block_number().await?;
        
        // Estimate base fee from latest block (simplified)
        // In production, would call eth_feeHistory
        let base_fee = Some(U256::from(20000000000u64)); // 20 gwei estimate
        let priority_fee = Some(U256::from(2000000000u64)); // 2 gwei estimate
        
        Ok(FeeHistory {
            base_fee,
            priority_fee,
            gas_used_ratio: 0.5, // 50% full blocks
        })
    }

    /// Get logs for a filter
    pub async fn get_logs(&self, filter: Filter) -> Result<Vec<Log>> {
        self.client.get_logs(&filter).await
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: TxHash) -> Result<Option<Transaction>> {
        self.client.get_transaction(tx_hash).await
    }

    /// Get client metrics
    pub async fn get_metrics(&self) -> crate::providers::multi_rpc::ClientMetrics {
        self.client.get_metrics().await
    }

    /// Helper: Call a contract method
    async fn call_contract(&self, address: H160, calldata: ethers::types::Bytes, block: Option<ethers::types::BlockNumber>) -> Result<ethers::types::Bytes> {
        use ethers::types::transaction::eip2718::TypedTransaction;
        use ethers::types::NameOrAddress;

        let mut tx = TypedTransaction::default();
        tx.set_to(NameOrAddress::Address(address));
        tx.set_data(calldata);

        self.client.call(&tx, block.map(|b| b.into())).await
    }
}

/// Uniswap V3 slot0 data structure
#[derive(Debug, Clone)]
pub struct UniswapV3Slot0 {
    /// Current sqrt price (encoded as Q64.96)
    pub sqrt_price_x96: U256,
    
    /// Current tick
    pub tick: i32,
    
    /// Current observation index
    pub observation_index: u16,
    
    /// Current observation cardinality
    pub observation_cardinality: u16,
    
    /// Next observation cardinality
    pub observation_cardinality_next: u16,
    
    /// Fee protocol
    pub fee_protocol: u8,
    
    /// Whether the pool is unlocked
    pub unlocked: bool,
}

/// Gas price information
#[derive(Debug, Clone)]
pub struct GasPriceInfo {
    /// Current gas price
    pub gas_price: U256,
    
    /// Base fee (EIP-1559)
    pub base_fee: Option<U256>,
    
    /// Priority fee (EIP-1559)
    pub priority_fee: Option<U256>,
}

/// Fee history data
#[derive(Debug, Clone)]
pub struct FeeHistory {
    pub base_fee: Option<U256>,
    pub priority_fee: Option<U256>,
    pub gas_used_ratio: f64,
}

impl Clone for EthereumRpcClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl EthereumRpcClient {
    // === Curve Contract Calls ===

    /// Get virtual price from Curve pool
    pub async fn get_curve_virtual_price(&self, pool_address: H160) -> Result<U256> {
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_curve_virtual_price_call()?;
        let result = self.call_contract(pool_address, calldata, None).await?;
        AbiManager::decode_curve_virtual_price_output(&result)
    }

    /// Get balance for a specific coin in Curve pool
    pub async fn get_curve_balance(&self, pool_address: H160, coin_index: u64) -> Result<U256> {
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_curve_balances_call(coin_index)?;
        let result = self.call_contract(pool_address, calldata, None).await?;
        AbiManager::decode_curve_balances_output(&result)
    }

    /// Get coin address for a specific index in Curve pool
    pub async fn get_curve_coin(&self, pool_address: H160, coin_index: u64) -> Result<H160> {
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_curve_coins_call(coin_index)?;
        let result = self.call_contract(pool_address, calldata, None).await?;
        AbiManager::decode_curve_coins_output(&result)
    }

    // === Balancer Contract Calls ===

    /// Get pool tokens and balances from Balancer Vault
    pub async fn get_balancer_pool_tokens(&self, vault_address: H160, pool_id: [u8; 32]) -> Result<crate::utils::contracts::BalancerPoolTokens> {
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_balancer_pool_tokens_call(pool_id)?;
        let result = self.call_contract(vault_address, calldata, None).await?;
        AbiManager::decode_balancer_pool_tokens_output(&result)
    }

    // === Aave V3 Contract Calls ===

    /// Get reserve data from Aave V3 pool
    pub async fn get_aave_reserve_data(&self, pool_address: H160, asset: H160) -> Result<crate::utils::contracts::AaveReserveData> {
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_aave_reserve_data_call(asset)?;
        let result = self.call_contract(pool_address, calldata, None).await?;
        AbiManager::decode_aave_reserve_data_output(&result)
    }

    // === ERC20 Contract Calls ===

    /// Get ERC20 token decimals
    pub async fn get_erc20_decimals(&self, token_address: H160) -> Result<u8> {
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_erc20_decimals_call()?;
        let result = self.call_contract(token_address, calldata, None).await?;
        AbiManager::decode_erc20_decimals_output(&result)
    }

    /// Get ERC20 token symbol
    pub async fn get_erc20_symbol(&self, token_address: H160) -> Result<String> {
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_erc20_symbol_call()?;
        let result = self.call_contract(token_address, calldata, None).await?;
        AbiManager::decode_erc20_symbol_output(&result)
    }

    /// Get ERC20 token balance
    pub async fn get_erc20_balance(&self, token_address: H160, owner: H160) -> Result<U256> {
        use ethers::abi::Token;
        use crate::utils::contracts::AbiManager;

        let calldata = AbiManager::encode_function_call(
            &crate::utils::contracts::ERC20_ABI,
            "balanceOf",
            &[Token::Address(owner)],
        )?;
        let result = self.call_contract(token_address, calldata, None).await?;
        let tokens = AbiManager::decode_function_output(&crate::utils::contracts::ERC20_ABI, "balanceOf", &result)?;

        match &tokens[0] {
            Token::Uint(val) => Ok(*val),
            _ => Err(BetaDataplaneError::internal("Invalid balance type")),
        }
    }
}

//! API key management for RPC providers
//!
//! Handles secure API key storage, retrieval, and URL construction
//! with support for environment variables and configuration files.

use std::collections::HashMap;
use std::env;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn, error};

use crate::{ProviderType, Chain, Result, BetaDataplaneError};

/// API key manager for RPC providers
#[derive(Debug, Clone)]
pub struct ApiKeyManager {
    /// API keys loaded from configuration
    api_keys: HashMap<String, String>,
    
    /// Provider endpoint templates
    endpoints: HashMap<ProviderType, ProviderEndpoints>,
}

/// Provider endpoint templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpoints {
    /// HTTP endpoint templates by chain
    pub http_endpoints: HashMap<Chain, String>,
    
    /// WebSocket endpoint templates by chain
    pub ws_endpoints: HashMap<Chain, String>,
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new() -> Self {
        Self {
            api_keys: HashMap::new(),
            endpoints: Self::default_endpoints(),
        }
    }

    /// Load API keys from environment variables and configuration
    pub fn load_api_keys(&mut self) -> Result<()> {
        // Load from environment variables first (highest priority)
        self.load_from_environment();
        
        // TODO: Load from configuration files
        // self.load_from_config_file("config/providers.toml")?;
        
        // Validate that we have required API keys
        self.validate_api_keys()?;
        
        Ok(())
    }

    /// Load API keys from environment variables
    fn load_from_environment(&mut self) {
        let env_mappings = [
            // Alchemy
            ("ALCHEMY_ETHEREUM_KEY", "alchemy_ethereum"),
            ("ALCHEMY_ARBITRUM_KEY", "alchemy_arbitrum"),
            ("ALCHEMY_OPTIMISM_KEY", "alchemy_optimism"),
            ("ALCHEMY_BASE_KEY", "alchemy_base"),
            
            // Infura
            ("INFURA_ETHEREUM_KEY", "infura_ethereum"),
            ("INFURA_ARBITRUM_KEY", "infura_arbitrum"),
            ("INFURA_OPTIMISM_KEY", "infura_optimism"),
            
            // QuickNode
            ("QUICKNODE_ETHEREUM_KEY", "quicknode_ethereum"),
            ("QUICKNODE_ARBITRUM_KEY", "quicknode_arbitrum"),
            ("QUICKNODE_OPTIMISM_KEY", "quicknode_optimism"),
            ("QUICKNODE_BASE_KEY", "quicknode_base"),
            
            // Ankr
            ("ANKR_ETHEREUM_KEY", "ankr_ethereum"),
            ("ANKR_ARBITRUM_KEY", "ankr_arbitrum"),
            ("ANKR_OPTIMISM_KEY", "ankr_optimism"),
            ("ANKR_BASE_KEY", "ankr_base"),
        ];

        for (env_var, key_name) in env_mappings {
            if let Ok(api_key) = env::var(env_var) {
                if !api_key.is_empty() && api_key != "YOUR_KEY_HERE" {
                    self.api_keys.insert(key_name.to_string(), api_key);
                    debug!(key_name = key_name, "Loaded API key from environment");
                }
            }
        }
    }

    /// Validate that we have the minimum required API keys
    fn validate_api_keys(&self) -> Result<()> {
        // For Ankr with embedded API keys, we don't need separate env vars
        // The validation is relaxed - we just check that we have endpoints configured
        
        // Check that we have at least Ankr endpoints configured
        if let Some(ankr_endpoints) = self.endpoints.get(&ProviderType::Ankr) {
            if ankr_endpoints.http_endpoints.contains_key(&Chain::Ethereum) {
                // We're good - Ankr endpoints are configured
                return Ok(());
            }
        }

        // If using other providers, check for API keys
        let recommended_keys = ["alchemy_ethereum", "infura_ethereum"];
        let mut has_any_provider = false;
        
        for key in &recommended_keys {
            if self.api_keys.contains_key(*key) {
                has_any_provider = true;
                break;
            }
        }

        if !has_any_provider {
            warn!("No API keys configured - relying on Ankr endpoints with embedded keys");
        }

        Ok(())
    }

    /// Get API key for a provider and chain
    pub fn get_api_key(&self, provider_type: ProviderType, chain: Chain) -> Option<&String> {
        let key_name = format!("{}_{}", provider_type.name(), chain.name());
        self.api_keys.get(&key_name)
    }

    /// Build HTTP URL for a provider and chain
    pub fn build_http_url(&self, provider_type: ProviderType, chain: Chain) -> Result<String> {
        let endpoints = self.endpoints.get(&provider_type)
            .ok_or_else(|| BetaDataplaneError::Provider {
                provider: provider_type.name().to_string(),
                message: "Provider endpoints not configured".to_string(),
            })?;

        let template = endpoints.http_endpoints.get(&chain)
            .ok_or_else(|| BetaDataplaneError::Provider {
                provider: provider_type.name().to_string(),
                message: format!("HTTP endpoint not configured for chain {}", chain),
            })?;

        // Replace API key placeholder
        if template.contains("{api_key}") {
            let api_key = self.get_api_key(provider_type, chain)
                .ok_or_else(|| BetaDataplaneError::Provider {
                    provider: provider_type.name().to_string(),
                    message: format!("API key not found for chain {}", chain),
                })?;
            
            Ok(template.replace("{api_key}", api_key))
        } else {
            // No API key required (free endpoints)
            Ok(template.clone())
        }
    }

    /// Build WebSocket URL for a provider and chain
    pub fn build_ws_url(&self, provider_type: ProviderType, chain: Chain) -> Result<Option<String>> {
        let endpoints = self.endpoints.get(&provider_type)
            .ok_or_else(|| BetaDataplaneError::Provider {
                provider: provider_type.name().to_string(),
                message: "Provider endpoints not configured".to_string(),
            })?;

        let template = match endpoints.ws_endpoints.get(&chain) {
            Some(template) => template,
            None => return Ok(None), // WebSocket not supported for this chain
        };

        // Replace API key placeholder
        if template.contains("{api_key}") {
            let api_key = self.get_api_key(provider_type, chain)
                .ok_or_else(|| BetaDataplaneError::Provider {
                    provider: provider_type.name().to_string(),
                    message: format!("API key not found for chain {}", chain),
                })?;
            
            Ok(Some(template.replace("{api_key}", api_key)))
        } else {
            // No API key required
            Ok(Some(template.clone()))
        }
    }

    /// Get available providers for a chain (those with API keys or configured endpoints)
    pub fn get_available_providers(&self, chain: Chain) -> Vec<ProviderType> {
        let mut available = Vec::new();
        
        for provider_type in [ProviderType::Alchemy, ProviderType::Infura, ProviderType::QuickNode, ProviderType::Ankr] {
            // Check if provider is configured (has API key or has endpoint)
            if self.is_provider_configured(provider_type, chain) {
                available.push(provider_type);
            }
        }
        
        // Always include free providers as fallback
        if !available.contains(&ProviderType::Ankr) {
            available.push(ProviderType::Custom);
        }
        
        available
    }

    /// Check if a provider is configured for a chain
    pub fn is_provider_configured(&self, provider_type: ProviderType, chain: Chain) -> bool {
        // Check if we have endpoints
        if let Some(endpoints) = self.endpoints.get(&provider_type) {
            if endpoints.http_endpoints.contains_key(&chain) {
                // Check if API key is required and available
                if endpoints.http_endpoints[&chain].contains("{api_key}") {
                    self.get_api_key(provider_type, chain).is_some()
                } else {
                    true // No API key required
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get configuration summary
    pub fn get_config_summary(&self) -> ApiKeyConfigSummary {
        let mut summary = ApiKeyConfigSummary {
            total_keys_loaded: self.api_keys.len(),
            providers_by_chain: HashMap::new(),
            missing_keys: Vec::new(),
        };

        for chain in [Chain::Ethereum, Chain::Arbitrum, Chain::Optimism, Chain::Base] {
            let available = self.get_available_providers(chain);
            summary.providers_by_chain.insert(chain, available);
        }

        // Check for missing recommended keys
        let recommended_keys = [
            ("alchemy_ethereum", "Alchemy Ethereum (primary)"),
            ("infura_ethereum", "Infura Ethereum (fallback)"),
            ("alchemy_arbitrum", "Alchemy Arbitrum"),
            ("alchemy_optimism", "Alchemy Optimism"),
            ("alchemy_base", "Alchemy Base"),
        ];

        for (key, description) in recommended_keys {
            if !self.api_keys.contains_key(key) {
                summary.missing_keys.push(format!("{}: {}", key, description));
            }
        }

        summary
    }

    /// Default provider endpoints
    fn default_endpoints() -> HashMap<ProviderType, ProviderEndpoints> {
        let mut endpoints = HashMap::new();

        // Alchemy endpoints
        endpoints.insert(ProviderType::Alchemy, ProviderEndpoints {
            http_endpoints: [
                (Chain::Ethereum, "https://eth-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
                (Chain::Arbitrum, "https://arb-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
                (Chain::Optimism, "https://opt-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
                (Chain::Base, "https://base-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
            ].into_iter().collect(),
            ws_endpoints: [
                (Chain::Ethereum, "wss://eth-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
                (Chain::Arbitrum, "wss://arb-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
                (Chain::Optimism, "wss://opt-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
                (Chain::Base, "wss://base-mainnet.g.alchemy.com/v2/{api_key}".to_string()),
            ].into_iter().collect(),
        });

        // Infura endpoints
        endpoints.insert(ProviderType::Infura, ProviderEndpoints {
            http_endpoints: [
                (Chain::Ethereum, "https://mainnet.infura.io/v3/{api_key}".to_string()),
                (Chain::Arbitrum, "https://arbitrum-mainnet.infura.io/v3/{api_key}".to_string()),
                (Chain::Optimism, "https://optimism-mainnet.infura.io/v3/{api_key}".to_string()),
            ].into_iter().collect(),
            ws_endpoints: [
                (Chain::Ethereum, "wss://mainnet.infura.io/ws/v3/{api_key}".to_string()),
                (Chain::Arbitrum, "wss://arbitrum-mainnet.infura.io/ws/v3/{api_key}".to_string()),
                (Chain::Optimism, "wss://optimism-mainnet.infura.io/ws/v3/{api_key}".to_string()),
            ].into_iter().collect(),
        });

        // Ankr endpoints (API key is embedded in the URL)
        endpoints.insert(ProviderType::Ankr, ProviderEndpoints {
            http_endpoints: [
                (Chain::Ethereum, "https://rpc.ankr.com/eth/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928".to_string()),
                (Chain::Arbitrum, "https://rpc.ankr.com/arbitrum/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928".to_string()),
                (Chain::Optimism, "https://rpc.ankr.com/optimism/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928".to_string()),
                (Chain::Base, "https://rpc.ankr.com/eth/f110e805f79ecd3b6874b5da27de68011d6be7bce468150a076d531f41b64928".to_string()),
            ].into_iter().collect(),
            ws_endpoints: HashMap::new(), // Ankr doesn't provide WebSocket endpoints
        });

        // Free endpoints (no API key required)
        endpoints.insert(ProviderType::Custom, ProviderEndpoints {
            http_endpoints: [
                (Chain::Ethereum, "https://ethereum.publicnode.com".to_string()),
                (Chain::Arbitrum, "https://arbitrum-one.publicnode.com".to_string()),
                (Chain::Optimism, "https://optimism.publicnode.com".to_string()),
                (Chain::Base, "https://base.publicnode.com".to_string()),
            ].into_iter().collect(),
            ws_endpoints: HashMap::new(), // Public nodes typically don't provide WebSocket
        });

        endpoints
    }
}

/// API key configuration summary
#[derive(Debug, Clone)]
pub struct ApiKeyConfigSummary {
    /// Total number of API keys loaded
    pub total_keys_loaded: usize,
    
    /// Available providers by chain
    pub providers_by_chain: HashMap<Chain, Vec<ProviderType>>,
    
    /// Missing recommended API keys
    pub missing_keys: Vec<String>,
}

impl ApiKeyConfigSummary {
    /// Check if configuration is production-ready
    pub fn is_production_ready(&self) -> bool {
        // Need at least 2 providers per chain for redundancy
        for (chain, providers) in &self.providers_by_chain {
            if providers.len() < 2 {
                warn!(chain = %chain, provider_count = providers.len(), "Insufficient provider redundancy");
                return false;
            }
        }
        
        // Should have minimal missing keys
        if self.missing_keys.len() > 2 {
            warn!(missing_count = self.missing_keys.len(), "Too many missing API keys");
            return false;
        }
        
        true
    }

    /// Get setup instructions for missing keys
    pub fn get_setup_instructions(&self) -> Vec<String> {
        let mut instructions = Vec::new();
        
        if self.missing_keys.is_empty() {
            instructions.push("✅ All recommended API keys are configured!".to_string());
        } else {
            instructions.push("🔑 Missing API Keys - Set these environment variables:".to_string());
            instructions.push("".to_string());
            
            for missing in &self.missing_keys {
                if missing.contains("alchemy") {
                    instructions.push(format!("export ALCHEMY_ETHEREUM_KEY=your_key_here  # {}", missing));
                } else if missing.contains("infura") {
                    instructions.push(format!("export INFURA_ETHEREUM_KEY=your_key_here   # {}", missing));
                } else if missing.contains("quicknode") {
                    instructions.push(format!("export QUICKNODE_ETHEREUM_KEY=your_key_here # {}", missing));
                } else if missing.contains("ankr") {
                    instructions.push(format!("export ANKR_ETHEREUM_KEY=your_key_here     # {}", missing));
                }
            }
            
            instructions.push("".to_string());
            instructions.push("📚 Provider Setup Guides:".to_string());
            instructions.push("• Alchemy: https://docs.alchemy.com/docs/alchemy-quickstart-guide".to_string());
            instructions.push("• Infura: https://docs.infura.io/infura/getting-started".to_string());
            instructions.push("• QuickNode: https://www.quicknode.com/guides/ethereum-development/getting-started".to_string());
            instructions.push("• Ankr: https://www.ankr.com/docs/rpc-service/getting-started/".to_string());
        }
        
        instructions
    }
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for API key management
impl ApiKeyManager {
    /// Create a configured provider config with API keys
    pub fn create_provider_config(
        &self,
        provider_type: ProviderType,
        chain: Chain,
        name: String,
    ) -> Result<crate::config::ProviderConfig> {
        let http_url = self.build_http_url(provider_type, chain)?;
        let ws_url = self.build_ws_url(provider_type, chain)?;
        
        Ok(crate::config::ProviderConfig {
            provider_type,
            name,
            http_url,
            ws_url,
            api_key: self.get_api_key(provider_type, chain).cloned(),
            rate_limit: provider_type.default_rate_limit(),
            timeout_seconds: 30,
            max_retries: 3,
            weight: 1.0,
            enabled: true,
        })
    }

    /// Auto-configure providers based on available API keys
    pub fn auto_configure_providers(&self, chain: Chain) -> Vec<crate::config::ProviderConfig> {
        let mut providers = Vec::new();
        
        for provider_type in self.get_available_providers(chain) {
            if let Ok(config) = self.create_provider_config(
                provider_type,
                chain,
                format!("{}-{}", provider_type.name(), chain.name()),
            ) {
                providers.push(config);
            }
        }
        
        // Sort by priority (Alchemy first, then others)
        providers.sort_by_key(|p| match p.provider_type {
            ProviderType::Alchemy => 0,
            ProviderType::QuickNode => 1,
            ProviderType::Infura => 2,
            ProviderType::Ankr => 3,
            ProviderType::LlamaRpc => 4,
            ProviderType::Custom => 5,
        });
        
        providers
    }

    /// Print configuration status
    pub fn print_status(&self) {
        let summary = self.get_config_summary();
        
        println!("🔑 API Key Configuration Status");
        println!("================================");
        println!("Total API keys loaded: {}", summary.total_keys_loaded);
        println!();
        
        for (chain, providers) in &summary.providers_by_chain {
            println!("📡 {} providers: {:?}", chain, providers);
        }
        
        println!();
        if summary.is_production_ready() {
            println!("✅ Configuration is production-ready!");
        } else {
            println!("⚠️  Configuration needs improvement for production");
        }
        
        if !summary.missing_keys.is_empty() {
            println!();
            for instruction in summary.get_setup_instructions() {
                println!("{}", instruction);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_manager_creation() {
        let manager = ApiKeyManager::new();
        assert!(manager.api_keys.is_empty());
        assert!(!manager.endpoints.is_empty());
    }

    #[test]
    fn test_endpoint_templates() {
        let manager = ApiKeyManager::new();
        
        // Test Alchemy endpoints
        if let Some(alchemy_endpoints) = manager.endpoints.get(&ProviderType::Alchemy) {
            assert!(alchemy_endpoints.http_endpoints.contains_key(&Chain::Ethereum));
            assert!(alchemy_endpoints.ws_endpoints.contains_key(&Chain::Ethereum));
        }
    }

    #[tokio::test]
    async fn test_provider_configuration() {
        let mut manager = ApiKeyManager::new();
        
        // Simulate having an API key
        manager.api_keys.insert("alchemy_ethereum".to_string(), "test_key".to_string());
        
        // Test URL building
        let http_url = manager.build_http_url(ProviderType::Alchemy, Chain::Ethereum);
        assert!(http_url.is_ok());
        assert!(http_url.unwrap().contains("test_key"));
    }
}

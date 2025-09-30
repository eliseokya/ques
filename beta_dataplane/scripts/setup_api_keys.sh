#!/bin/bash

# Qenus Beta Dataplane - API Key Setup Script
# This script helps you configure API keys for RPC providers

set -e

echo "🔑 Qenus Beta Dataplane - API Key Setup"
echo "========================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to prompt for API key
prompt_for_key() {
    local provider=$1
    local env_var=$2
    local description=$3
    local current_value=${!env_var}
    
    echo -e "${BLUE}$provider${NC} - $description"
    
    if [ -n "$current_value" ] && [ "$current_value" != "YOUR_KEY_HERE" ]; then
        echo -e "  ${GREEN}✅ Already configured${NC} (${current_value:0:8}...)"
        return
    fi
    
    echo -e "  ${YELLOW}⚠️  Not configured${NC}"
    read -p "  Enter API key (or press Enter to skip): " new_key
    
    if [ -n "$new_key" ]; then
        export $env_var="$new_key"
        echo "export $env_var=\"$new_key\"" >> ~/.bashrc
        echo "export $env_var=\"$new_key\"" >> ~/.zshrc
        echo -e "  ${GREEN}✅ Configured and saved to shell profile${NC}"
    else
        echo -e "  ${YELLOW}⏭️  Skipped${NC}"
    fi
    echo ""
}

echo "This script will help you configure API keys for RPC providers."
echo "API keys will be saved to your shell profile (~/.bashrc and ~/.zshrc)"
echo ""

# Alchemy (Primary provider)
echo -e "${GREEN}🧪 Alchemy (Recommended Primary Provider)${NC}"
echo "Sign up at: https://www.alchemy.com/"
echo "Free tier: 300 requests/second, 300M compute units/month"
echo ""
prompt_for_key "Alchemy Ethereum" "ALCHEMY_ETHEREUM_KEY" "Primary Ethereum provider"
prompt_for_key "Alchemy Arbitrum" "ALCHEMY_ARBITRUM_KEY" "Primary Arbitrum provider"
prompt_for_key "Alchemy Optimism" "ALCHEMY_OPTIMISM_KEY" "Primary Optimism provider"
prompt_for_key "Alchemy Base" "ALCHEMY_BASE_KEY" "Primary Base provider"

# Infura (Fallback provider)
echo -e "${GREEN}🌐 Infura (Recommended Fallback Provider)${NC}"
echo "Sign up at: https://infura.io/"
echo "Free tier: 100,000 requests/day"
echo ""
prompt_for_key "Infura Ethereum" "INFURA_ETHEREUM_KEY" "Fallback Ethereum provider"
prompt_for_key "Infura Arbitrum" "INFURA_ARBITRUM_KEY" "Fallback Arbitrum provider"
prompt_for_key "Infura Optimism" "INFURA_OPTIMISM_KEY" "Fallback Optimism provider"

# QuickNode (Premium provider)
echo -e "${GREEN}⚡ QuickNode (Premium Provider - Optional)${NC}"
echo "Sign up at: https://www.quicknode.com/"
echo "Paid service: Higher rate limits and better performance"
echo ""
prompt_for_key "QuickNode Ethereum" "QUICKNODE_ETHEREUM_KEY" "Premium Ethereum provider"
prompt_for_key "QuickNode Arbitrum" "QUICKNODE_ARBITRUM_KEY" "Premium Arbitrum provider"

# Ankr (Alternative provider)
echo -e "${GREEN}🔗 Ankr (Alternative Provider - Optional)${NC}"
echo "Sign up at: https://www.ankr.com/"
echo "Free tier available"
echo ""
prompt_for_key "Ankr Ethereum" "ANKR_ETHEREUM_KEY" "Alternative Ethereum provider"

echo "🔄 Reloading shell environment..."
source ~/.bashrc 2>/dev/null || true
source ~/.zshrc 2>/dev/null || true

echo ""
echo -e "${GREEN}✅ API Key setup complete!${NC}"
echo ""
echo "🧪 Test your configuration:"
echo "  cd /path/to/qenus/beta_dataplane"
echo "  cargo run --bin beta-dataplane -- --test-providers"
echo ""
echo "🚀 Start the beta dataplane:"
echo "  cargo run --bin beta-dataplane -- --mode development"
echo ""
echo "📊 Monitor provider health:"
echo "  curl http://localhost:8080/health"
echo ""

# Test configuration
echo "🔍 Testing current configuration..."
cd "$(dirname "$0")/.."

if command -v cargo &> /dev/null; then
    echo "Running provider test..."
    if cargo run --bin beta-dataplane -- --test-providers 2>/dev/null; then
        echo -e "${GREEN}✅ Provider configuration test passed!${NC}"
    else
        echo -e "${YELLOW}⚠️  Provider test failed - check your API keys${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Cargo not found - install Rust to test configuration${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Setup complete! You're ready to start the beta dataplane.${NC}"

# API Key Setup Guide

**Essential for Beta Dataplane Operation**

The Beta Dataplane requires API keys from RPC providers to access blockchain data. This guide walks you through setting up the necessary API keys for production-ready operation.

---

## 🎯 **Quick Start**

### **Minimum Required (for testing):**
1. **Alchemy Ethereum** - Primary provider
2. **Infura Ethereum** - Fallback provider

### **Production Recommended:**
1. **All Alchemy keys** - Primary providers for all chains
2. **All Infura keys** - Fallback providers for all chains
3. **QuickNode** - Premium provider for critical chains

---

## 🔑 **Provider Setup**

### **1. Alchemy (Primary Provider) - REQUIRED**
```bash
# Sign up: https://www.alchemy.com/
# Free tier: 300 req/sec, 300M compute units/month

export ALCHEMY_ETHEREUM_KEY="your_alchemy_ethereum_key"
export ALCHEMY_ARBITRUM_KEY="your_alchemy_arbitrum_key"  
export ALCHEMY_OPTIMISM_KEY="your_alchemy_optimism_key"
export ALCHEMY_BASE_KEY="your_alchemy_base_key"
```

**Setup Steps:**
1. Go to [alchemy.com](https://www.alchemy.com/)
2. Create account and verify email
3. Create new app for each chain (Ethereum, Arbitrum, Optimism, Base)
4. Copy API keys from dashboard
5. Set environment variables

### **2. Infura (Fallback Provider) - RECOMMENDED**
```bash
# Sign up: https://infura.io/
# Free tier: 100,000 requests/day

export INFURA_ETHEREUM_KEY="your_infura_ethereum_key"
export INFURA_ARBITRUM_KEY="your_infura_arbitrum_key"
export INFURA_OPTIMISM_KEY="your_infura_optimism_key"
```

**Setup Steps:**
1. Go to [infura.io](https://infura.io/)
2. Create account and verify email
3. Create new project
4. Enable desired networks (Ethereum, Arbitrum, Optimism)
5. Copy project ID (this is your API key)
6. Set environment variables

### **3. QuickNode (Premium Provider) - OPTIONAL**
```bash
# Sign up: https://www.quicknode.com/
# Paid service: Higher performance and rate limits

export QUICKNODE_ETHEREUM_KEY="your_quicknode_ethereum_key"
export QUICKNODE_ARBITRUM_KEY="your_quicknode_arbitrum_key"
export QUICKNODE_OPTIMISM_KEY="your_quicknode_optimism_key"
export QUICKNODE_BASE_KEY="your_quicknode_base_key"
```

**Setup Steps:**
1. Go to [quicknode.com](https://www.quicknode.com/)
2. Create account and choose plan
3. Create endpoint for each chain
4. Copy endpoint URLs (extract key from URL)
5. Set environment variables

### **4. Ankr (Alternative Provider) - OPTIONAL**
```bash
# Sign up: https://www.ankr.com/
# Free tier available

export ANKR_ETHEREUM_KEY="your_ankr_ethereum_key"
export ANKR_ARBITRUM_KEY="your_ankr_arbitrum_key"
export ANKR_OPTIMISM_KEY="your_ankr_optimism_key"
export ANKR_BASE_KEY="your_ankr_base_key"
```

---

## 🛠️ **Setup Methods**

### **Method 1: Interactive Setup Script**
```bash
cd beta_dataplane
./scripts/setup_api_keys.sh
```

### **Method 2: Manual Environment Variables**
```bash
# Add to ~/.bashrc or ~/.zshrc
export ALCHEMY_ETHEREUM_KEY="your_key_here"
export INFURA_ETHEREUM_KEY="your_key_here"
# ... add other keys

# Reload shell
source ~/.bashrc  # or ~/.zshrc
```

### **Method 3: Environment File**
```bash
# Copy example file
cp config/environment.example .env

# Edit .env file with your API keys
nano .env

# Load environment (if using dotenv)
source .env
```

---

## ✅ **Validation**

### **Check API Key Status**
```bash
cargo run --bin beta-dataplane -- --setup-keys
```

**Expected Output:**
```
🔑 API Key Configuration Status
================================
Total API keys loaded: 8

📡 ethereum providers: [Alchemy, Infura, Custom]
📡 arbitrum providers: [Alchemy, Infura, Custom]
📡 optimism providers: [Alchemy, Infura, Custom]
📡 base providers: [Alchemy, Custom]

✅ Configuration is production-ready!
```

### **Test Provider Connections**
```bash
cargo run --bin beta-dataplane -- --test-providers
```

**Expected Output:**
```
INFO Testing providers for ethereum (3 providers)
INFO Provider connection test passed: alchemy-ethereum-primary
INFO Provider connection test passed: infura-ethereum-fallback
INFO Provider testing completed successfully
```

---

## 🚨 **Troubleshooting**

### **Common Issues:**

#### **"Missing required API keys" Error**
```bash
# Check which keys are missing
cargo run --bin beta-dataplane -- --setup-keys

# Set the missing keys
export ALCHEMY_ETHEREUM_KEY="your_key"
```

#### **"Invalid HTTP URL" Error**
- Check that API key doesn't contain special characters
- Ensure API key is not placeholder text
- Verify provider supports the requested chain

#### **"Rate limit exceeded" Error**
- You're hitting provider rate limits
- Add more providers for redundancy
- Upgrade to higher tier plan

#### **"Provider unavailable" Error**
- Check internet connection
- Verify API key is valid and active
- Check provider status page

### **Debug Mode:**
```bash
# Run with debug logging
cargo run --bin beta-dataplane -- --log-level debug --test-providers
```

---

## 💰 **Cost Optimization**

### **Free Tier Limits:**
```
Provider    | Free Tier Limit        | Overage Cost
------------|------------------------|---------------
Alchemy     | 300M CU/month         | $0.50/1M CU
Infura      | 100K requests/day     | $0.50/10K req
QuickNode   | No free tier          | $9/month minimum
Ankr        | 500 requests/day      | $99/month unlimited
```

### **Recommended Strategy:**
1. **Start with free tiers** (Alchemy + Infura)
2. **Monitor usage** through provider dashboards
3. **Upgrade selectively** based on performance needs
4. **Use multiple providers** to distribute load

### **Cost Estimation:**
```
Development: $0/month (free tiers)
Testing: $10-50/month (light usage)
Production: $100-500/month (depending on volume)
```

---

## 🔒 **Security Best Practices**

### **API Key Security:**
1. **Never commit API keys** to version control
2. **Use environment variables** or secure key management
3. **Rotate keys regularly** (monthly recommended)
4. **Monitor usage** for unauthorized access
5. **Use separate keys** for different environments

### **Environment Separation:**
```bash
# Development
export ALCHEMY_ETHEREUM_KEY="dev_key_here"

# Production  
export ALCHEMY_ETHEREUM_KEY="prod_key_here"
```

### **Key Rotation:**
```bash
# Update key in provider dashboard
# Update environment variable
export ALCHEMY_ETHEREUM_KEY="new_key_here"

# Restart beta dataplane
cargo run --bin beta-dataplane -- --mode production
```

---

## 🚀 **Production Deployment**

### **Recommended Configuration:**
```bash
# Primary providers (high performance)
export ALCHEMY_ETHEREUM_KEY="..."
export ALCHEMY_ARBITRUM_KEY="..."
export ALCHEMY_OPTIMISM_KEY="..."
export ALCHEMY_BASE_KEY="..."

# Fallback providers (redundancy)
export INFURA_ETHEREUM_KEY="..."
export INFURA_ARBITRUM_KEY="..."
export INFURA_OPTIMISM_KEY="..."

# Premium providers (critical operations)
export QUICKNODE_ETHEREUM_KEY="..."
export QUICKNODE_ARBITRUM_KEY="..."
```

### **Monitoring:**
- Set up alerts for rate limit warnings
- Monitor provider health metrics
- Track cost usage through provider dashboards
- Set up automatic failover testing

---

## 📞 **Support**

### **Provider Support:**
- **Alchemy**: [Discord](https://discord.gg/alchemy) | [Docs](https://docs.alchemy.com/)
- **Infura**: [Support](https://infura.io/contact) | [Docs](https://docs.infura.io/)
- **QuickNode**: [Support](https://www.quicknode.com/contact-us) | [Docs](https://www.quicknode.com/docs)
- **Ankr**: [Support](https://www.ankr.com/contact/) | [Docs](https://www.ankr.com/docs/)

### **Beta Dataplane Issues:**
- Check logs: `cargo run --bin beta-dataplane -- --log-level debug`
- Test providers: `cargo run --bin beta-dataplane -- --test-providers`
- Check health: `curl http://localhost:8080/health`

**With proper API key setup, you'll have redundant, high-performance access to all supported blockchains!** 🚀

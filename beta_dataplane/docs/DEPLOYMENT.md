# Qenus Beta Dataplane - Production Deployment Guide

## 📋 **Prerequisites**

### **System Requirements**
- **OS**: Linux (Ubuntu 22.04 LTS recommended) or macOS
- **CPU**: 4+ cores
- **RAM**: 8GB+ (16GB recommended for production)
- **Disk**: 100GB+ SSD
- **Network**: Stable internet with low latency to RPC providers

### **Software Requirements**
- Docker 24.0+ and Docker Compose 2.0+
- Git
- curl, jq (for health checks)
- Optional: kubectl (for Kubernetes deployment)

## 🚀 **Quick Start**

### **1. Clone and Configure**

```bash
# Clone the repository
git clone https://github.com/eliseokya/ques.git
cd qenus/beta_dataplane

# Copy environment template
cp config/environment.example config/.env.production

# Edit with your API keys and settings
nano config/.env.production
```

### **2. Set API Keys**

Edit `config/.env.production`:
```bash
ANKR_API_KEY=your_ankr_key
ALCHEMY_API_KEY=your_alchemy_key  # Optional
INFURA_API_KEY=your_infura_key    # Optional
```

### **3. Deploy**

```bash
# Development/Testing
docker-compose up -d

# Production
docker-compose -f docker-compose.prod.yml up -d
```

### **4. Verify**

```bash
# Check health
curl http://localhost:8080/health

# Check all services
docker-compose ps

# View logs
docker-compose logs -f beta-dataplane
```

## 🔧 **Production Deployment**

### **Step 1: Prepare Server**

```bash
# Update system
sudo apt-get update && sudo apt-get upgrade -y

# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Create data directories
sudo mkdir -p /data/qenus/{beta-dataplane,kafka,redis,prometheus,grafana,postgres}
sudo chown -R $USER:$USER /data/qenus
```

### **Step 2: Configure Environment**

```bash
# Create production environment
cp config/environment.example config/.env.production

# Required variables
nano config/.env.production
```

Set these critical values:
- `POSTGRES_PASSWORD` - Strong database password
- `GRAFANA_ADMIN_PASSWORD` - Grafana admin password
- `ANKR_API_KEY` - Your Ankr API key
- `API_KEY` - API key for gRPC access

### **Step 3: Deploy Services**

```bash
# Use the deployment script
./scripts/deploy-production.sh

# Or manually
docker-compose -f docker-compose.prod.yml up -d
```

### **Step 4: Verify Deployment**

```bash
# Run health check
./scripts/health-check.sh

# Check logs
docker-compose -f docker-compose.prod.yml logs -f

# Test endpoints
curl http://localhost:8080/health
curl http://localhost:9092/metrics
```

## 📊 **Service Architecture**

```
┌─────────────────────────────────────────────────┐
│           Qenus Beta Dataplane Stack            │
├─────────────────────────────────────────────────┤
│                                                  │
│  ┌──────────────┐      ┌─────────────┐         │
│  │ Beta         │─────▶│   Kafka     │         │
│  │ Dataplane    │      │  (Redpanda) │         │
│  │              │      └─────────────┘         │
│  │ Port: 8080   │                               │
│  │ gRPC: 50053  │      ┌─────────────┐         │
│  │ Metrics:9092 │◀─────│   Redis     │         │
│  └──────────────┘      │  (Cache)    │         │
│         │              └─────────────┘         │
│         │                                       │
│         │              ┌─────────────┐         │
│         └─────────────▶│ Prometheus  │         │
│                        │ Port: 9090  │         │
│                        └──────┬──────┘         │
│                               │                 │
│                        ┌──────▼──────┐         │
│                        │  Grafana    │         │
│                        │ Port: 3000  │         │
│                        └─────────────┘         │
│                                                  │
│  ┌──────────────┐                               │
│  │ PostgreSQL   │◀──── (Optional state)         │
│  │ Port: 5432   │                               │
│  └──────────────┘                               │
└─────────────────────────────────────────────────┘
```

## 🔍 **Monitoring**

### **Access Dashboards**

- **Grafana**: http://localhost:3000
  - Username: `admin`
  - Password: Set in `.env.production`

- **Prometheus**: http://localhost:9090

- **Kafka UI**: http://localhost:8082 (Redpanda Console)

### **Key Metrics to Monitor**

```promql
# Feature extraction rate
rate(beta_dataplane_features_extracted_total[5m])

# RPC call success rate
beta_dataplane_rpc_success_rate

# Cache hit rate
beta_dataplane_cache_hit_rate

# Alert count
beta_dataplane_active_alerts
```

## 🛡️ **Security**

### **Production Checklist**

- [ ] Change all default passwords
- [ ] Use strong API keys
- [ ] Enable TLS/SSL for external endpoints
- [ ] Restrict network access (firewall rules)
- [ ] Regular security updates
- [ ] Rotate API keys periodically
- [ ] Enable audit logging
- [ ] Backup encryption keys

### **Network Security**

```bash
# Use UFW or iptables to restrict access
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 443/tcp   # HTTPS (if using reverse proxy)
sudo ufw deny 5432/tcp   # Deny external PostgreSQL access
sudo ufw deny 6379/tcp   # Deny external Redis access
sudo ufw enable
```

## 🔄 **Maintenance**

### **Updates**

```bash
# Pull latest code
git pull origin main

# Rebuild and redeploy
docker-compose -f docker-compose.prod.yml build
./scripts/deploy-production.sh
```

### **Backups**

```bash
# Manual backup
./scripts/backup.sh

# Restore from backup
./scripts/restore.sh /backups/qenus/backup_TIMESTAMP.tar.gz
```

### **Logs**

```bash
# View logs
docker-compose -f docker-compose.prod.yml logs -f beta-dataplane

# Export logs
docker-compose -f docker-compose.prod.yml logs --no-color > logs/export_$(date +%Y%m%d).log
```

## 🆘 **Troubleshooting**

### **Service Won't Start**

```bash
# Check logs
docker-compose -f docker-compose.prod.yml logs beta-dataplane

# Check configuration
docker-compose -f docker-compose.prod.yml config

# Restart service
docker-compose -f docker-compose.prod.yml restart beta-dataplane
```

### **High Memory Usage**

```bash
# Check container stats
docker stats

# Adjust limits in docker-compose.prod.yml
# Restart with new limits
docker-compose -f docker-compose.prod.yml up -d
```

### **Connection Issues**

```bash
# Test RPC providers
cargo run --package qenus-beta-dataplane -- test-providers

# Check network connectivity
docker-compose -f docker-compose.prod.yml exec beta-dataplane ping -c 3 rpc.ankr.com
```

## 📈 **Scaling**

### **Horizontal Scaling**

To run multiple instances:

```yaml
# In docker-compose.prod.yml
beta-dataplane:
  deploy:
    replicas: 3  # Run 3 instances
```

### **Resource Tuning**

Adjust based on load:

```yaml
deploy:
  resources:
    limits:
      cpus: '8'      # Increase for higher throughput
      memory: 8G
```

## 🎯 **Performance Optimization**

### **Configuration Tuning**

Edit `config/beta-dataplane.toml`:

```toml
[global]
worker_threads = 8        # Match CPU cores
max_memory_mb = 4096

[optimization.caching]
cache_size_mb = 1024      # Increase for better hit rate

[optimization.batching]
default_batch_size = 100  # Tune based on throughput
```

## 📞 **Support**

For issues or questions:
1. Check logs: `docker-compose logs`
2. Run health check: `./scripts/health-check.sh`
3. Review metrics: http://localhost:9090
4. Open an issue on GitHub

---

**🎉 You're now running Qenus Beta Dataplane in production!**

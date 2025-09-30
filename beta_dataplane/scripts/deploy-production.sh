#!/bin/bash
# Production deployment script for Qenus Beta Dataplane

set -e

echo "🚀 Qenus Beta Dataplane - Production Deployment"
echo "==============================================="

# Configuration
COMPOSE_FILE="docker-compose.prod.yml"
ENV_FILE="config/.env.production"
DATA_DIR="/data/qenus"
BACKUP_DIR="/backups/qenus"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root or with sudo
if [ "$EUID" -eq 0 ]; then
    echo -e "${RED}❌ Do not run this script as root!${NC}"
    exit 1
fi

# Check if .env.production exists
if [ ! -f "$ENV_FILE" ]; then
    echo -e "${RED}❌ Production environment file not found: $ENV_FILE${NC}"
    echo "Please create it from config/environment.example"
    exit 1
fi

# Check if data directories exist
echo "📁 Checking data directories..."
sudo mkdir -p "$DATA_DIR"/{beta-dataplane,kafka,redis,prometheus,grafana,postgres}
sudo mkdir -p "$BACKUP_DIR"
sudo chown -R $USER:$USER "$DATA_DIR"

# Pull latest images
echo "📥 Pulling latest Docker images..."
docker-compose -f $COMPOSE_FILE pull

# Stop existing containers
echo "🛑 Stopping existing containers..."
docker-compose -f $COMPOSE_FILE down

# Create backup of current data (if exists)
if [ -d "$DATA_DIR/beta-dataplane" ] && [ "$(ls -A $DATA_DIR/beta-dataplane)" ]; then
    echo "💾 Creating backup..."
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    sudo tar -czf "$BACKUP_DIR/backup_$TIMESTAMP.tar.gz" "$DATA_DIR" 2>/dev/null || true
    echo -e "${GREEN}✅ Backup created: $BACKUP_DIR/backup_$TIMESTAMP.tar.gz${NC}"
fi

# Start services
echo "🚀 Starting production services..."
docker-compose -f $COMPOSE_FILE up -d

# Wait for services to be healthy
echo "⏳ Waiting for services to become healthy..."
sleep 10

# Check health
echo "🏥 Checking service health..."
HEALTHY=true

# Check beta-dataplane
if ! docker-compose -f $COMPOSE_FILE ps | grep -q "beta-dataplane.*Up"; then
    echo -e "${RED}❌ Beta Dataplane failed to start${NC}"
    HEALTHY=false
fi

# Check Kafka
if ! docker-compose -f $COMPOSE_FILE ps | grep -q "kafka.*Up"; then
    echo -e "${RED}❌ Kafka failed to start${NC}"
    HEALTHY=false
fi

# Check Redis
if ! docker-compose -f $COMPOSE_FILE ps | grep -q "redis.*Up"; then
    echo -e "${RED}❌ Redis failed to start${NC}"
    HEALTHY=false
fi

if [ "$HEALTHY" = true ]; then
    echo -e "${GREEN}✅ All services started successfully!${NC}"
    echo ""
    echo "📊 Access Points:"
    echo "  - Beta Dataplane Health: http://localhost:8080/health"
    echo "  - gRPC API: localhost:50053"
    echo "  - Prometheus: http://localhost:9090"
    echo "  - Grafana: http://localhost:3000 (admin/qenus_admin_change_me)"
    echo "  - Kafka: localhost:9092"
    echo ""
    echo "📝 Logs:"
    echo "  docker-compose -f $COMPOSE_FILE logs -f beta-dataplane"
    echo ""
    echo "🔍 Status:"
    docker-compose -f $COMPOSE_FILE ps
else
    echo -e "${RED}❌ Deployment failed! Check logs with:${NC}"
    echo "  docker-compose -f $COMPOSE_FILE logs"
    exit 1
fi


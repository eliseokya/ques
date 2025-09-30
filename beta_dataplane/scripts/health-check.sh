#!/bin/bash
# Health check script for production monitoring

set -e

HEALTH_URL="${HEALTH_URL:-http://localhost:8080/health}"
METRICS_URL="${METRICS_URL:-http://localhost:9092/metrics}"
GRPC_URL="${GRPC_URL:-localhost:50053}"

echo "🏥 Qenus Beta Dataplane - Health Check"
echo "======================================"

# Check HTTP health endpoint
echo -n "Health endpoint... "
if curl -f -s "$HEALTH_URL" > /dev/null 2>&1; then
    echo "✅ OK"
    HEALTH_STATUS=$(curl -s "$HEALTH_URL" | jq -r '.status' 2>/dev/null || echo "unknown")
    echo "   Status: $HEALTH_STATUS"
else
    echo "❌ FAILED"
    exit 1
fi

# Check metrics endpoint
echo -n "Metrics endpoint... "
if curl -f -s "$METRICS_URL" > /dev/null 2>&1; then
    echo "✅ OK"
else
    echo "❌ FAILED"
fi

# Check gRPC endpoint (if grpcurl is available)
if command -v grpcurl &> /dev/null; then
    echo -n "gRPC endpoint... "
    if grpcurl -plaintext "$GRPC_URL" list > /dev/null 2>&1; then
        echo "✅ OK"
    else
        echo "❌ FAILED"
    fi
fi

# Check Docker containers
echo ""
echo "🐳 Docker Container Status:"
docker-compose ps

# Check system resources
echo ""
echo "💻 System Resources:"
echo "CPU Usage:"
docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}" | head -10

echo ""
echo "✅ Health check complete"


# Qenus Beta Dataplane - Production Dockerfile
# Multi-stage build for optimized production images

# Stage 1: Build
FROM rust:1.75-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY dataplane ./dataplane
COPY beta_dataplane ./beta_dataplane

# Build the beta dataplane in release mode
WORKDIR /app/beta_dataplane
RUN cargo build --release --bin beta-dataplane

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash qenus

# Create directories
RUN mkdir -p /app/config /app/data /app/logs && \
    chown -R qenus:qenus /app

# Copy binary from builder
COPY --from=builder /app/target/release/beta-dataplane /app/beta-dataplane

# Copy configuration files
COPY --chown=qenus:qenus beta_dataplane/config /app/config

# Set working directory
WORKDIR /app

# Switch to non-root user
USER qenus

# Expose ports
# 8080 - Health checks
# 9092 - Prometheus metrics
# 50053 - gRPC API
EXPOSE 8080 9092 50053

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Run the beta dataplane
ENTRYPOINT ["/app/beta-dataplane"]
CMD ["--config", "/app/config/beta-dataplane.toml", "--mode", "production"]


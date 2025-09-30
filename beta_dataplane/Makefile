# Qenus Beta Dataplane - Makefile

.PHONY: help build test run clean docker-build docker-up docker-down deploy health

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build the beta dataplane
	cargo build --release --package qenus-beta-dataplane

test: ## Run all tests
	cargo test --package qenus-beta-dataplane

test-lib: ## Run library tests only
	cargo test --package qenus-beta-dataplane --lib

test-integration: ## Run integration tests
	cargo test --package qenus-beta-dataplane --test integration_tests

run: ## Run the beta dataplane locally
	cargo run --package qenus-beta-dataplane

run-dev: ## Run in development mode with debug logging
	RUST_LOG=debug cargo run --package qenus-beta-dataplane -- --mode development

test-providers: ## Test RPC provider connections
	cargo run --package qenus-beta-dataplane -- test-providers

check: ## Check code for errors
	cargo check --workspace

clippy: ## Run clippy linter
	cargo clippy --workspace --all-targets -- -D warnings

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/

# Docker targets
docker-build: ## Build Docker image
	docker build -f Dockerfile -t qenus-beta-dataplane:latest ..

docker-up: ## Start all services with docker-compose
	docker-compose up -d

docker-down: ## Stop all services
	docker-compose down

docker-logs: ## View docker logs
	docker-compose logs -f beta-dataplane

docker-ps: ## Show running containers
	docker-compose ps

# Production targets
deploy-prod: ## Deploy to production
	./scripts/deploy-production.sh

health: ## Run health check
	./scripts/health-check.sh

backup: ## Create backup
	./scripts/backup.sh

# Development targets
watch: ## Watch and auto-rebuild on changes
	cargo watch -x 'run --package qenus-beta-dataplane'

bench: ## Run benchmarks
	cargo bench --package qenus-beta-dataplane

doc: ## Generate and open documentation
	cargo doc --package qenus-beta-dataplane --open

# CI targets
ci-test: fmt-check clippy test ## Run all CI checks

# Update dependencies
update: ## Update dependencies
	cargo update

audit: ## Security audit
	cargo audit

# Default target
.DEFAULT_GOAL := help


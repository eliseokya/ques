//! Production monitoring for beta dataplane
//!
//! Comprehensive health checks, metrics collection, and alerting
//! for production-grade operations.

pub mod health;
pub mod metrics;
pub mod alerts;
pub mod dashboard;

// Re-export commonly used types (TODO: Implement in Phase 6)
// pub use health::{HealthChecker, ComponentHealth, SystemHealth};
// pub use metrics::{MetricsCollector, SystemMetrics};
// pub use alerts::{AlertManager, AlertRule, AlertSeverity};

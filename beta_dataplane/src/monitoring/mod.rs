//! Production monitoring for beta dataplane
//!
//! Comprehensive health checks, metrics collection, and alerting
//! for production-grade operations.

pub mod health;
pub mod metrics;
pub mod alerts;
pub mod dashboard;

pub use health::{HealthChecker, HealthReport, HealthStatus, ComponentHealth};
pub use metrics::{MetricsRegistry, MetricsCollector, Metric, MetricValue, MetricsSummary};
pub use alerts::{AlertManager, Alert, AlertRule, AlertSeverity, AlertState, ComparisonOperator, AlertStats};
pub use dashboard::{MonitoringDashboard, MonitoringService, HealthCheckResponse, DashboardOverview};

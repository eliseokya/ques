//! Metrics collection and aggregation system
//!
//! Tracks performance metrics across all dataplane components.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{Result, BetaDataplaneError};

/// Metric value types
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// Counter (monotonically increasing)
    Counter(u64),
    
    /// Gauge (can go up or down)
    Gauge(f64),
    
    /// Histogram (distribution of values)
    Histogram {
        count: u64,
        sum: f64,
        min: f64,
        max: f64,
        avg: f64,
    },
}

impl MetricValue {
    /// Create a new histogram
    pub fn new_histogram(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::Histogram {
                count: 0,
                sum: 0.0,
                min: 0.0,
                max: 0.0,
                avg: 0.0,
            };
        }

        let count = values.len() as u64;
        let sum: f64 = values.iter().sum();
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let avg = sum / count as f64;

        Self::Histogram {
            count,
            sum,
            min,
            max,
            avg,
        }
    }
}

/// A single metric with metadata
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric name
    pub name: String,
    
    /// Metric value
    pub value: MetricValue,
    
    /// Tags for grouping and filtering
    pub tags: HashMap<String, String>,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Help text describing the metric
    pub help: String,
}

impl Metric {
    /// Create a new counter metric
    pub fn counter(name: String, value: u64, help: String) -> Self {
        Self {
            name,
            value: MetricValue::Counter(value),
            tags: HashMap::new(),
            timestamp: chrono::Utc::now(),
            help,
        }
    }

    /// Create a new gauge metric
    pub fn gauge(name: String, value: f64, help: String) -> Self {
        Self {
            name,
            value: MetricValue::Gauge(value),
            tags: HashMap::new(),
            timestamp: chrono::Utc::now(),
            help,
        }
    }

    /// Create a new histogram metric
    pub fn histogram(name: String, values: &[f64], help: String) -> Self {
        Self {
            name,
            value: MetricValue::new_histogram(values),
            tags: HashMap::new(),
            timestamp: chrono::Utc::now(),
            help,
        }
    }

    /// Add a tag
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Add multiple tags
    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags.extend(tags);
        self
    }
}

/// Metrics collector for a specific component
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// Component name
    component: String,
    
    /// Collected metrics
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
    
    /// Histogram buffers for efficient aggregation
    histogram_buffers: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    
    /// Start time for duration metrics
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(component: String) -> Self {
        Self {
            component,
            metrics: Arc::new(RwLock::new(HashMap::new())),
            histogram_buffers: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a counter value
    pub async fn record_counter(&self, name: &str, value: u64, help: &str) {
        let metric_name = format!("{}_{}", self.component, name);
        let metric = Metric::counter(metric_name.clone(), value, help.to_string())
            .with_tag("component".to_string(), self.component.clone());
        
        let mut metrics = self.metrics.write().await;
        metrics.insert(metric_name, metric);
    }

    /// Increment a counter
    pub async fn increment_counter(&self, name: &str, help: &str) {
        let metric_name = format!("{}_{}", self.component, name);
        let mut metrics = self.metrics.write().await;
        
        let current_value = if let Some(metric) = metrics.get(&metric_name) {
            if let MetricValue::Counter(v) = metric.value {
                v
            } else {
                0
            }
        } else {
            0
        };
        
        let metric = Metric::counter(metric_name.clone(), current_value + 1, help.to_string())
            .with_tag("component".to_string(), self.component.clone());
        
        metrics.insert(metric_name, metric);
    }

    /// Record a gauge value
    pub async fn record_gauge(&self, name: &str, value: f64, help: &str) {
        let metric_name = format!("{}_{}", self.component, name);
        let metric = Metric::gauge(metric_name.clone(), value, help.to_string())
            .with_tag("component".to_string(), self.component.clone());
        
        let mut metrics = self.metrics.write().await;
        metrics.insert(metric_name, metric);
    }

    /// Record a histogram value
    pub async fn record_histogram_value(&self, name: &str, value: f64) {
        let metric_name = format!("{}_{}", self.component, name);
        let mut buffers = self.histogram_buffers.write().await;
        
        buffers
            .entry(metric_name)
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Flush histogram buffers into metrics
    pub async fn flush_histograms(&self, help_texts: &HashMap<String, String>) {
        let mut buffers = self.histogram_buffers.write().await;
        let mut metrics = self.metrics.write().await;
        
        for (name, values) in buffers.iter_mut() {
            if !values.is_empty() {
                let help = help_texts
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| "Histogram metric".to_string());
                
                let metric = Metric::histogram(name.clone(), values, help)
                    .with_tag("component".to_string(), self.component.clone());
                
                metrics.insert(name.clone(), metric);
                values.clear();
            }
        }
    }

    /// Get all metrics
    pub async fn get_metrics(&self) -> HashMap<String, Metric> {
        self.metrics.read().await.clone()
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Clear all metrics
    pub async fn clear(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.clear();
        
        let mut buffers = self.histogram_buffers.write().await;
        buffers.clear();
    }
}

/// Global metrics registry
pub struct MetricsRegistry {
    /// Collectors for each component
    collectors: Arc<RwLock<HashMap<String, MetricsCollector>>>,
    
    /// Histogram help texts
    histogram_help: Arc<RwLock<HashMap<String, String>>>,
    
    /// Auto-flush interval
    flush_interval: Duration,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new(flush_interval: Duration) -> Self {
        Self {
            collectors: Arc::new(RwLock::new(HashMap::new())),
            histogram_help: Arc::new(RwLock::new(HashMap::new())),
            flush_interval,
        }
    }

    /// Register a new metrics collector
    pub async fn register_collector(&self, component: String) -> MetricsCollector {
        let collector = MetricsCollector::new(component.clone());
        let mut collectors = self.collectors.write().await;
        collectors.insert(component, collector.clone());
        collector
    }

    /// Get a collector by component name
    pub async fn get_collector(&self, component: &str) -> Option<MetricsCollector> {
        let collectors = self.collectors.read().await;
        collectors.get(component).cloned()
    }

    /// Register histogram help text
    pub async fn register_histogram_help(&self, name: String, help: String) {
        let mut help_texts = self.histogram_help.write().await;
        help_texts.insert(name, help);
    }

    /// Flush all histogram buffers
    pub async fn flush_all_histograms(&self) {
        let help_texts = self.histogram_help.read().await.clone();
        let collectors = self.collectors.read().await;
        
        for collector in collectors.values() {
            collector.flush_histograms(&help_texts).await;
        }
    }

    /// Get all metrics from all collectors
    pub async fn get_all_metrics(&self) -> HashMap<String, Metric> {
        let collectors = self.collectors.read().await;
        let mut all_metrics = HashMap::new();
        
        for collector in collectors.values() {
            let metrics = collector.get_metrics().await;
            all_metrics.extend(metrics);
        }
        
        all_metrics
    }

    /// Start auto-flushing histograms
    pub async fn start_auto_flush(&self) -> Result<()> {
        info!("Starting auto-flush for histograms with interval: {:?}", self.flush_interval);
        
        let registry = Arc::new(self.clone());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(registry.flush_interval);
            
            loop {
                interval.tick().await;
                registry.flush_all_histograms().await;
            }
        });
        
        Ok(())
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        let metrics = self.get_all_metrics().await;
        let mut output = String::new();
        
        for (name, metric) in metrics {
            // Add help text
            output.push_str(&format!("# HELP {} {}\n", name, metric.help));
            
            // Add type
            let metric_type = match metric.value {
                MetricValue::Counter(_) => "counter",
                MetricValue::Gauge(_) => "gauge",
                MetricValue::Histogram { .. } => "histogram",
            };
            output.push_str(&format!("# TYPE {} {}\n", name, metric_type));
            
            // Add value(s)
            match metric.value {
                MetricValue::Counter(v) => {
                    output.push_str(&format!("{}{} {}\n", name, format_tags(&metric.tags), v));
                }
                MetricValue::Gauge(v) => {
                    output.push_str(&format!("{}{} {}\n", name, format_tags(&metric.tags), v));
                }
                MetricValue::Histogram {
                    count,
                    sum,
                    ..
                } => {
                    output.push_str(&format!(
                        "{}_count{} {}\n",
                        name,
                        format_tags(&metric.tags),
                        count
                    ));
                    output.push_str(&format!(
                        "{}_sum{} {}\n",
                        name,
                        format_tags(&metric.tags),
                        sum
                    ));
                }
            }
            
            output.push('\n');
        }
        
        output
    }

    /// Get metrics summary
    pub async fn get_summary(&self) -> MetricsSummary {
        let metrics = self.get_all_metrics().await;
        
        let mut counter_count = 0;
        let mut gauge_count = 0;
        let mut histogram_count = 0;
        
        for metric in metrics.values() {
            match metric.value {
                MetricValue::Counter(_) => counter_count += 1,
                MetricValue::Gauge(_) => gauge_count += 1,
                MetricValue::Histogram { .. } => histogram_count += 1,
            }
        }
        
        let collectors = self.collectors.read().await;
        
        MetricsSummary {
            total_metrics: metrics.len(),
            counter_count,
            gauge_count,
            histogram_count,
            component_count: collectors.len(),
            timestamp: chrono::Utc::now(),
        }
    }
}

// Implement Clone for MetricsRegistry
impl Clone for MetricsRegistry {
    fn clone(&self) -> Self {
        Self {
            collectors: Arc::clone(&self.collectors),
            histogram_help: Arc::clone(&self.histogram_help),
            flush_interval: self.flush_interval,
        }
    }
}

/// Summary of metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSummary {
    pub total_metrics: usize,
    pub counter_count: usize,
    pub gauge_count: usize,
    pub histogram_count: usize,
    pub component_count: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Helper function to format tags for Prometheus
fn format_tags(tags: &HashMap<String, String>) -> String {
    if tags.is_empty() {
        return String::new();
    }
    
    let tag_pairs: Vec<String> = tags
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, v))
        .collect();
    
    format!("{{{}}}", tag_pairs.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new("test".to_string());
        
        collector.record_counter("requests", 100, "Total requests").await;
        collector.record_gauge("cpu_usage", 45.5, "CPU usage percentage").await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.len(), 2);
    }

    #[tokio::test]
    async fn test_histogram() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let hist = MetricValue::new_histogram(&values);
        
        if let MetricValue::Histogram { count, sum, min, max, avg } = hist {
            assert_eq!(count, 5);
            assert_eq!(sum, 15.0);
            assert_eq!(min, 1.0);
            assert_eq!(max, 5.0);
            assert_eq!(avg, 3.0);
        } else {
            panic!("Expected histogram");
        }
    }

    #[tokio::test]
    async fn test_metrics_registry() {
        let registry = MetricsRegistry::new(Duration::from_secs(60));
        let collector = registry.register_collector("test".to_string()).await;
        
        collector.record_counter("ops", 50, "Operations").await;
        
        let all_metrics = registry.get_all_metrics().await;
        assert_eq!(all_metrics.len(), 1);
    }
}

//! Utility functions and helpers for the dataplane

use chrono::{DateTime, Utc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use tracing::{debug, warn};

use crate::{DataplaneError, Result};

/// Retry configuration for operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// Retry a future with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut delay = config.initial_delay;
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        debug!(
            operation = operation_name,
            attempt = attempt,
            max_attempts = config.max_attempts,
            "Attempting operation"
        );

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(error) => {
                last_error = Some(error);
                
                if attempt < config.max_attempts {
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        delay_ms = delay.as_millis(),
                        error = %last_error.as_ref().unwrap(),
                        "Operation failed, retrying"
                    );
                    
                    sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_millis((delay.as_millis() as f64 * config.backoff_multiplier) as u64),
                        config.max_delay,
                    );
                } else {
                    warn!(
                        operation = operation_name,
                        attempt = attempt,
                        error = %last_error.as_ref().unwrap(),
                        "Operation failed after all retry attempts"
                    );
                }
            }
        }
    }

    Err(last_error.unwrap())
}

/// Execute an operation with a timeout
pub async fn with_timeout<F, T>(
    future: F,
    timeout_duration: Duration,
    operation_name: &str,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    match timeout(timeout_duration, future).await {
        Ok(result) => result,
        Err(_) => {
            warn!(
                operation = operation_name,
                timeout_ms = timeout_duration.as_millis(),
                "Operation timed out"
            );
            Err(DataplaneError::internal(format!(
                "Operation '{}' timed out after {}ms",
                operation_name,
                timeout_duration.as_millis()
            )))
        }
    }
}

/// Convert a SystemTime to a DateTime<Utc>
pub fn system_time_to_datetime(time: SystemTime) -> DateTime<Utc> {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos()).unwrap_or_else(Utc::now)
}

/// Convert a Unix timestamp to a DateTime<Utc>
pub fn timestamp_to_datetime(timestamp: u64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_else(Utc::now)
}

/// Convert a DateTime<Utc> to a Unix timestamp
pub fn datetime_to_timestamp(datetime: DateTime<Utc>) -> u64 {
    datetime.timestamp() as u64
}

/// Validate an Ethereum address
pub fn is_valid_ethereum_address(address: &str) -> bool {
    if !address.starts_with("0x") {
        return false;
    }
    
    if address.len() != 42 {
        return false;
    }
    
    address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate a transaction hash
pub fn is_valid_transaction_hash(hash: &str) -> bool {
    if !hash.starts_with("0x") {
        return false;
    }
    
    if hash.len() != 66 {
        return false;
    }
    
    hash[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Normalize an Ethereum address to lowercase
pub fn normalize_address(address: &str) -> Result<String> {
    if !is_valid_ethereum_address(address) {
        return Err(DataplaneError::InvalidAddress(address.to_string()));
    }
    Ok(address.to_lowercase())
}

/// Convert Wei to Ether (as f64)
pub fn wei_to_ether(wei: &str) -> Result<f64> {
    let wei_value: u128 = wei.parse()
        .map_err(|_| DataplaneError::internal(format!("Invalid Wei value: {}", wei)))?;
    
    Ok(wei_value as f64 / 1e18)
}

/// Convert Ether to Wei (as string to preserve precision)
pub fn ether_to_wei(ether: f64) -> String {
    let wei = (ether * 1e18) as u128;
    wei.to_string()
}

/// Convert Gwei to Wei
pub fn gwei_to_wei(gwei: f64) -> u64 {
    (gwei * 1e9) as u64
}

/// Convert Wei to Gwei
pub fn wei_to_gwei(wei: u64) -> f64 {
    wei as f64 / 1e9
}

/// Calculate percentage change between two values
pub fn percentage_change(old_value: f64, new_value: f64) -> f64 {
    if old_value == 0.0 {
        return 0.0;
    }
    ((new_value - old_value) / old_value) * 100.0
}

/// Calculate basis points between two values
pub fn basis_points_change(old_value: f64, new_value: f64) -> f64 {
    percentage_change(old_value, new_value) * 100.0
}

/// Format a large number with appropriate units (K, M, B)
pub fn format_large_number(value: f64) -> String {
    if value >= 1e9 {
        format!("{:.2}B", value / 1e9)
    } else if value >= 1e6 {
        format!("{:.2}M", value / 1e6)
    } else if value >= 1e3 {
        format!("{:.2}K", value / 1e3)
    } else {
        format!("{:.2}", value)
    }
}

/// Calculate exponential moving average
pub fn exponential_moving_average(current_ema: f64, new_value: f64, alpha: f64) -> f64 {
    alpha * new_value + (1.0 - alpha) * current_ema
}

/// Calculate simple moving average from a window of values
pub fn simple_moving_average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Calculate standard deviation from a window of values
pub fn standard_deviation(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    
    let mean = simple_moving_average(values);
    let variance = values.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / (values.len() - 1) as f64;
    
    variance.sqrt()
}

/// Calculate percentile from a sorted array of values
pub fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    
    if percentile <= 0.0 {
        return sorted_values[0];
    }
    
    if percentile >= 1.0 {
        return sorted_values[sorted_values.len() - 1];
    }
    
    let index = percentile * (sorted_values.len() - 1) as f64;
    let lower_index = index.floor() as usize;
    let upper_index = index.ceil() as usize;
    
    if lower_index == upper_index {
        sorted_values[lower_index]
    } else {
        let weight = index - lower_index as f64;
        sorted_values[lower_index] * (1.0 - weight) + sorted_values[upper_index] * weight
    }
}

/// Rate limiter using token bucket algorithm
#[derive(Debug)]
pub struct RateLimiter {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: std::time::Instant,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests_per_second: f64) -> Self {
        Self {
            tokens: max_requests_per_second,
            max_tokens: max_requests_per_second,
            refill_rate: max_requests_per_second,
            last_refill: std::time::Instant::now(),
        }
    }
    
    /// Try to acquire a token (non-blocking)
    pub fn try_acquire(&mut self) -> bool {
        self.refill();
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
    
    /// Wait until a token is available
    pub async fn acquire(&mut self) {
        loop {
            if self.try_acquire() {
                break;
            }
            
            // Calculate how long to wait for the next token
            let wait_time = Duration::from_secs_f64(1.0 / self.refill_rate);
            sleep(wait_time).await;
        }
    }
    
    fn refill(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

/// Circuit breaker for handling failures
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    last_failure_time: Option<std::time::Instant>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold,
            timeout,
            last_failure_time: None,
        }
    }
    
    /// Check if the circuit breaker allows the operation
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.timeout {
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
    
    /// Record a successful operation
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }
    
    /// Record a failed operation
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(std::time::Instant::now());
        
        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }
    
    /// Get the current state
    pub fn state(&self) -> CircuitState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_ethereum_address() {
        assert!(is_valid_ethereum_address("0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b"));
        assert!(is_valid_ethereum_address("0x0000000000000000000000000000000000000000"));
        assert!(!is_valid_ethereum_address("742d35Cc6634C0532925a3b8D4C9db96C4b4d8b"));
        assert!(!is_valid_ethereum_address("0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8"));
        assert!(!is_valid_ethereum_address("0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8bg"));
    }

    #[test]
    fn test_wei_to_ether() {
        assert_eq!(wei_to_ether("1000000000000000000").unwrap(), 1.0);
        assert_eq!(wei_to_ether("500000000000000000").unwrap(), 0.5);
        assert_eq!(wei_to_ether("0").unwrap(), 0.0);
    }

    #[test]
    fn test_percentage_change() {
        assert_eq!(percentage_change(100.0, 110.0), 10.0);
        assert_eq!(percentage_change(100.0, 90.0), -10.0);
        assert_eq!(percentage_change(0.0, 10.0), 0.0);
    }

    #[test]
    fn test_simple_moving_average() {
        assert_eq!(simple_moving_average(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
        assert_eq!(simple_moving_average(&[]), 0.0);
        assert_eq!(simple_moving_average(&[5.0]), 5.0);
    }

    #[test]
    fn test_percentile() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&values, 0.5), 3.0);
        assert_eq!(percentile(&values, 0.0), 1.0);
        assert_eq!(percentile(&values, 1.0), 5.0);
    }
}

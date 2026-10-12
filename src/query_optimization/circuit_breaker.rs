//! Query Circuit Breaker for N+1 prevention and automated protection (Issue #423).
//!
//! Tracks the ratio of total executed queries to unique query fingerprints within
//! an execution window / request lifecycle. If the ratio exceeds `max_n_plus_one_ratio`
//! (default 20:1), trips the circuit breaker, emits CRITICAL alerts, updates Prometheus
//! metrics, and aborts execution with HTTP 429 (Too Many Requests).

use crate::query_optimization::normalize::normalize;
use crate::query_optimization::nplus1::NPlusOnePolicy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Default maximum allowed ratio of total queries to unique fingerprints (20:1).
pub const DEFAULT_MAX_N_PLUS_ONE_RATIO: f64 = 20.0;

/// Default minimum total queries required before ratio evaluation can trip the breaker.
pub const DEFAULT_MIN_QUERIES_THRESHOLD: usize = 20;

/// Error returned when the N+1 circuit breaker trips.
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum CircuitBreakerError {
    #[error(
        "N+1 query circuit breaker tripped: ratio {ratio:.2} exceeds threshold {max_ratio:.2} \
         (total {total}, unique {unique})"
    )]
    NPlusOneTripped {
        ratio: f64,
        max_ratio: f64,
        total: usize,
        unique: usize,
    },
}

impl CircuitBreakerError {
    /// Associated HTTP status code (HTTP 429 Too Many Requests).
    pub fn status_code(&self) -> axum::http::StatusCode {
        axum::http::StatusCode::TOO_MANY_REQUESTS
    }
}

/// Prometheus metrics for N+1 circuit breaker monitoring.
#[derive(Debug, Default)]
pub struct CircuitBreakerMetrics {
    /// Counter: Total queries blocked by the circuit breaker.
    pub blocked_total: AtomicU64,
    /// Gauge: Current N+1 query ratio scaled by 1000 for atomic representation.
    pub current_ratio_millis: AtomicU64,
}

impl CircuitBreakerMetrics {
    /// Creates a new metrics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increments the blocked queries counter.
    pub fn record_blocked(&self) {
        self.blocked_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Updates the current N+1 ratio gauge.
    pub fn update_ratio(&self, ratio: f64) {
        let millis = (ratio.max(0.0) * 1000.0) as u64;
        self.current_ratio_millis.store(millis, Ordering::Relaxed);
    }

    /// Reads total blocked count.
    pub fn blocked_total(&self) -> u64 {
        self.blocked_total.load(Ordering::Relaxed)
    }

    /// Reads current ratio gauge value.
    pub fn current_ratio(&self) -> f64 {
        self.current_ratio_millis.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Renders metrics in Prometheus text exposition format.
    pub fn render_prometheus(&self) -> String {
        format!(
            "# HELP n_plus_one_queries_blocked_total Total queries blocked by N+1 circuit breaker\n\
             # TYPE n_plus_one_queries_blocked_total counter\n\
             n_plus_one_queries_blocked_total {}\n\
             # HELP n_plus_one_ratio_current Current N+1 query ratio (total queries / unique queries)\n\
             # TYPE n_plus_one_ratio_current gauge\n\
             n_plus_one_ratio_current {:.3}\n",
            self.blocked_total(),
            self.current_ratio()
        )
    }
}

/// Configuration for the query circuit breaker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Max allowed total / unique ratio before tripping (default 20.0).
    pub max_n_plus_one_ratio: f64,
    /// Minimum total queries required to trip (default 20).
    pub min_queries_threshold: usize,
    /// Enforcement policy (`Allow`, `Warn`, `Block`).
    pub policy: NPlusOnePolicy,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_n_plus_one_ratio: DEFAULT_MAX_N_PLUS_ONE_RATIO,
            min_queries_threshold: DEFAULT_MIN_QUERIES_THRESHOLD,
            policy: NPlusOnePolicy::Block,
        }
    }
}

/// Request-scoped circuit breaker tracking N+1 query multiplication.
#[derive(Debug, Clone)]
pub struct QueryCircuitBreaker {
    config: CircuitBreakerConfig,
    total_queries: usize,
    fingerprints: HashMap<String, usize>,
    tripped: bool,
    metrics: Arc<CircuitBreakerMetrics>,
}

impl Default for QueryCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryCircuitBreaker {
    /// Creates a circuit breaker with default settings (20:1 max ratio, Block policy).
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Creates a circuit breaker with a custom configuration.
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            total_queries: 0,
            fingerprints: HashMap::new(),
            tripped: false,
            metrics: Arc::new(CircuitBreakerMetrics::new()),
        }
    }

    /// Attaches an existing shared Prometheus metrics collector.
    pub fn with_metrics(mut self, metrics: Arc<CircuitBreakerMetrics>) -> Self {
        self.metrics = metrics;
        self
    }

    /// Returns the active policy.
    pub fn policy(&self) -> NPlusOnePolicy {
        self.config.policy
    }

    /// Overrides the policy (e.g. for endpoint-specific overrides).
    pub fn set_policy(&mut self, policy: NPlusOnePolicy) {
        self.config.policy = policy;
    }

    /// Returns the total queries recorded so far.
    pub fn total_queries(&self) -> usize {
        self.total_queries
    }

    /// Returns the number of distinct query fingerprints observed.
    pub fn unique_queries(&self) -> usize {
        self.fingerprints.len()
    }

    /// Returns the current N+1 ratio (`total / unique`).
    pub fn current_ratio(&self) -> f64 {
        if self.fingerprints.is_empty() {
            0.0
        } else {
            self.total_queries as f64 / self.fingerprints.len() as f64
        }
    }

    /// Whether the circuit breaker has tripped.
    pub fn is_tripped(&self) -> bool {
        self.tripped
    }

    /// Shared metrics handle.
    pub fn metrics(&self) -> Arc<CircuitBreakerMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Records an executed query and verifies if the N+1 ratio triggers the circuit breaker.
    pub fn record_query(&mut self, sql: &str) -> Result<(), CircuitBreakerError> {
        let fp = normalize(sql);
        self.total_queries += 1;
        *self.fingerprints.entry(fp).or_insert(0) += 1;

        let ratio = self.current_ratio();
        self.metrics.update_ratio(ratio);

        if self.total_queries >= self.config.min_queries_threshold
            && ratio > self.config.max_n_plus_one_ratio
        {
            self.tripped = true;

            // Log CRITICAL alert as mandated by requirements
            tracing::error!(
                ratio = ratio,
                max_ratio = self.config.max_n_plus_one_ratio,
                total = self.total_queries,
                unique = self.unique_queries(),
                "CRITICAL: N+1 query circuit breaker tripped: ratio {:.2} exceeds max {:.2}",
                ratio,
                self.config.max_n_plus_one_ratio
            );

            match self.config.policy {
                NPlusOnePolicy::Block => {
                    self.metrics.record_blocked();
                    return Err(CircuitBreakerError::NPlusOneTripped {
                        ratio,
                        max_ratio: self.config.max_n_plus_one_ratio,
                        total: self.total_queries,
                        unique: self.unique_queries(),
                    });
                }
                NPlusOnePolicy::Warn => {
                    tracing::warn!(
                        ratio = ratio,
                        "N+1 query ratio warning: ratio {:.2} exceeds threshold {:.2}",
                        ratio,
                        self.config.max_n_plus_one_ratio
                    );
                }
                NPlusOnePolicy::Allow => {}
            }
        }

        Ok(())
    }

    /// Resets the tracker for a new request.
    pub fn reset(&mut self) {
        self.total_queries = 0;
        self.fingerprints.clear();
        self.tripped = false;
    }
}

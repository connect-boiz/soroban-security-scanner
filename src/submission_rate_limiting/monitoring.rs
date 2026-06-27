//! Monitoring, statistics and threshold alerting.
//!
//! Backs the rate-limit monitoring dashboard: counters are updated on every
//! decision, a rolling list of recent violations is retained, and alerts are
//! raised when the blocked-request ratio crosses a configurable threshold.

use crate::submission_rate_limiting::config::Tier;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// A snapshot of cumulative rate-limiter statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitStats {
    /// Total decisions made.
    pub total_requests: u64,
    /// Requests allowed without friction.
    pub allowed: u64,
    /// Requests that were blocked.
    pub blocked: u64,
    /// Requests that were challenged (CAPTCHA).
    pub challenged: u64,
    /// Requests that bypassed limits (researchers / exempt admins).
    pub bypassed: u64,
}

impl RateLimitStats {
    /// Blocked requests as a fraction of all requests (0.0 when none).
    pub fn block_ratio(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.blocked as f64 / self.total_requests as f64
        }
    }
}

/// A recorded limit violation, surfaced on the dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violation {
    /// When the violation occurred.
    pub at: DateTime<Utc>,
    /// Source IP.
    pub ip: IpAddr,
    /// Tier of the caller.
    pub tier: Tier,
    /// The scope that was exceeded (e.g. "user", "ip", "global", "upload").
    pub scope: String,
    /// Human-readable reason.
    pub reason: String,
}

/// A raised alert when an alerting threshold is breached.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alert {
    /// When the alert fired.
    pub at: DateTime<Utc>,
    /// Alert message.
    pub message: String,
    /// The observed block ratio that triggered the alert.
    pub block_ratio: f64,
}

/// Thread-safe collector of statistics, violations and alerts.
pub struct Monitor {
    total: AtomicU64,
    allowed: AtomicU64,
    blocked: AtomicU64,
    challenged: AtomicU64,
    bypassed: AtomicU64,
    /// Maximum recent violations retained.
    max_violations: usize,
    violations: Mutex<Vec<Violation>>,
    /// Block-ratio threshold above which an alert is raised.
    alert_threshold: f64,
    /// Minimum sample size before alerting, to avoid noise on cold start.
    alert_min_samples: u64,
    alerts: Mutex<Vec<Alert>>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new(1000, 0.5, 50)
    }
}

impl Monitor {
    /// Creates a monitor retaining `max_violations` recent violations and
    /// alerting when the block ratio exceeds `alert_threshold` after at least
    /// `alert_min_samples` requests.
    pub fn new(max_violations: usize, alert_threshold: f64, alert_min_samples: u64) -> Self {
        Self {
            total: AtomicU64::new(0),
            allowed: AtomicU64::new(0),
            blocked: AtomicU64::new(0),
            challenged: AtomicU64::new(0),
            bypassed: AtomicU64::new(0),
            max_violations,
            violations: Mutex::new(Vec::new()),
            alert_threshold,
            alert_min_samples,
            alerts: Mutex::new(Vec::new()),
        }
    }

    /// Records an allowed request.
    pub fn record_allowed(&self) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.allowed.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a bypassed request (researcher / exempt admin).
    pub fn record_bypassed(&self) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.allowed.fetch_add(1, Ordering::Relaxed);
        self.bypassed.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a challenged request.
    pub fn record_challenged(&self) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.challenged.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a blocked request along with its violation detail. Returns an
    /// `Alert` if this block pushed the block ratio past the threshold.
    pub fn record_blocked(&self, violation: Violation) -> Option<Alert> {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.blocked.fetch_add(1, Ordering::Relaxed);

        {
            let mut v = self.violations.lock().expect("violations mutex poisoned");
            v.push(violation.clone());
            let overflow = v.len().saturating_sub(self.max_violations);
            if overflow > 0 {
                v.drain(0..overflow);
            }
        }

        self.maybe_alert(violation.at)
    }

    /// Evaluates the alert condition and records an alert if breached.
    fn maybe_alert(&self, at: DateTime<Utc>) -> Option<Alert> {
        let stats = self.stats();
        if stats.total_requests < self.alert_min_samples {
            return None;
        }
        let ratio = stats.block_ratio();
        if ratio > self.alert_threshold {
            let alert = Alert {
                at,
                message: format!(
                    "Block ratio {:.1}% exceeds threshold {:.1}%",
                    ratio * 100.0,
                    self.alert_threshold * 100.0
                ),
                block_ratio: ratio,
            };
            self.alerts
                .lock()
                .expect("alerts mutex poisoned")
                .push(alert.clone());
            Some(alert)
        } else {
            None
        }
    }

    /// Returns a snapshot of current statistics.
    pub fn stats(&self) -> RateLimitStats {
        RateLimitStats {
            total_requests: self.total.load(Ordering::Relaxed),
            allowed: self.allowed.load(Ordering::Relaxed),
            blocked: self.blocked.load(Ordering::Relaxed),
            challenged: self.challenged.load(Ordering::Relaxed),
            bypassed: self.bypassed.load(Ordering::Relaxed),
        }
    }

    /// Returns up to `limit` most-recent violations (newest last).
    pub fn recent_violations(&self, limit: usize) -> Vec<Violation> {
        let v = self.violations.lock().expect("violations mutex poisoned");
        let start = v.len().saturating_sub(limit);
        v[start..].to_vec()
    }

    /// Returns all raised alerts.
    pub fn alerts(&self) -> Vec<Alert> {
        self.alerts.lock().expect("alerts mutex poisoned").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn violation() -> Violation {
        Violation {
            at: at(),
            ip: "10.0.0.1".parse().unwrap(),
            tier: Tier::User,
            scope: "user".to_string(),
            reason: "per-user hourly limit exceeded".to_string(),
        }
    }

    #[test]
    fn stats_accumulate() {
        let m = Monitor::default();
        m.record_allowed();
        m.record_allowed();
        m.record_blocked(violation());
        let s = m.stats();
        assert_eq!(s.total_requests, 3);
        assert_eq!(s.allowed, 2);
        assert_eq!(s.blocked, 1);
        assert!((s.block_ratio() - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn violations_are_capped() {
        let m = Monitor::new(2, 0.9, 1);
        for _ in 0..5 {
            m.record_blocked(violation());
        }
        assert_eq!(m.recent_violations(10).len(), 2);
    }

    #[test]
    fn alert_fires_past_threshold_with_enough_samples() {
        let m = Monitor::new(100, 0.5, 4);
        // 3 allowed, then blocks until ratio > 0.5 with >=4 samples.
        m.record_allowed();
        m.record_allowed();
        m.record_allowed();
        assert!(m.record_blocked(violation()).is_none()); // 1/4 = 0.25
        assert!(m.record_blocked(violation()).is_none()); // 2/5 = 0.4
        let fired = m.record_blocked(violation()); // 3/6 = 0.5, not > 0.5
        assert!(fired.is_none());
        let fired = m.record_blocked(violation()); // 4/7 ~ 0.57 > 0.5
        assert!(fired.is_some());
        assert_eq!(m.alerts().len(), 1);
    }

    #[test]
    fn no_alert_before_min_samples() {
        let m = Monitor::new(100, 0.1, 100);
        for _ in 0..10 {
            assert!(m.record_blocked(violation()).is_none());
        }
    }
}

//! Rate-limit analytics for the usage dashboard.
//!
//! Aggregates per-endpoint and per-user request counts and outcomes (allowed /
//! throttled / challenged / bypassed), surfacing top consumers and the overall
//! throttle rate — the data behind the analytics dashboard and trend view.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The outcome of a rate-limit decision (for analytics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Allowed through.
    Allowed,
    /// Throttled (rate-limited).
    Throttled,
    /// Challenged with a CAPTCHA.
    Challenged,
    /// Bypassed (verified user).
    Bypassed,
}

/// Per-key counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeCounts {
    /// Allowed.
    pub allowed: u64,
    /// Throttled.
    pub throttled: u64,
    /// Challenged.
    pub challenged: u64,
    /// Bypassed.
    pub bypassed: u64,
}

impl OutcomeCounts {
    /// Total requests.
    pub fn total(&self) -> u64 {
        self.allowed + self.throttled + self.challenged + self.bypassed
    }

    fn record(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Allowed => self.allowed += 1,
            Outcome::Throttled => self.throttled += 1,
            Outcome::Challenged => self.challenged += 1,
            Outcome::Bypassed => self.bypassed += 1,
        }
    }
}

/// A dashboard snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    /// Total requests observed.
    pub total_requests: u64,
    /// Total throttled.
    pub total_throttled: u64,
    /// Overall throttle rate (0–1).
    pub throttle_rate: f64,
    /// Top endpoints by request volume: `(endpoint, total)`.
    pub top_endpoints: Vec<(String, u64)>,
    /// Top users by request volume: `(user, total)`.
    pub top_users: Vec<(String, u64)>,
}

/// Collects rate-limit usage analytics.
#[derive(Debug, Clone, Default)]
pub struct Analytics {
    by_endpoint: HashMap<String, OutcomeCounts>,
    by_user: HashMap<String, OutcomeCounts>,
    totals: OutcomeCounts,
}

impl Analytics {
    /// Creates an empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a request's outcome.
    pub fn record(&mut self, endpoint: &str, user: &str, outcome: Outcome) {
        self.by_endpoint
            .entry(endpoint.to_string())
            .or_default()
            .record(outcome);
        self.by_user
            .entry(user.to_string())
            .or_default()
            .record(outcome);
        self.totals.record(outcome);
    }

    /// Counts for an endpoint.
    pub fn endpoint(&self, endpoint: &str) -> Option<OutcomeCounts> {
        self.by_endpoint.get(endpoint).copied()
    }

    /// Counts for a user.
    pub fn user(&self, user: &str) -> Option<OutcomeCounts> {
        self.by_user.get(user).copied()
    }

    /// A dashboard snapshot with the top `n` endpoints/users by volume.
    pub fn snapshot(&self, top_n: usize) -> AnalyticsSnapshot {
        AnalyticsSnapshot {
            total_requests: self.totals.total(),
            total_throttled: self.totals.throttled,
            throttle_rate: if self.totals.total() == 0 {
                0.0
            } else {
                self.totals.throttled as f64 / self.totals.total() as f64
            },
            top_endpoints: top_by_volume(&self.by_endpoint, top_n),
            top_users: top_by_volume(&self.by_user, top_n),
        }
    }
}

fn top_by_volume(map: &HashMap<String, OutcomeCounts>, n: usize) -> Vec<(String, u64)> {
    let mut items: Vec<(String, u64)> = map.iter().map(|(k, v)| (k.clone(), v.total())).collect();
    // Highest volume first; ties broken by name for determinism.
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items.truncate(n);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_outcomes_per_dimension() {
        let mut a = Analytics::new();
        a.record("/api/x", "alice", Outcome::Allowed);
        a.record("/api/x", "alice", Outcome::Throttled);
        a.record("/api/y", "bob", Outcome::Allowed);

        assert_eq!(a.endpoint("/api/x").unwrap().total(), 2);
        assert_eq!(a.endpoint("/api/x").unwrap().throttled, 1);
        assert_eq!(a.user("alice").unwrap().total(), 2);
    }

    #[test]
    fn snapshot_throttle_rate_and_tops() {
        let mut a = Analytics::new();
        for _ in 0..8 {
            a.record("/api/hot", "heavy", Outcome::Allowed);
        }
        for _ in 0..2 {
            a.record("/api/hot", "heavy", Outcome::Throttled);
        }
        a.record("/api/cold", "light", Outcome::Allowed);

        let snap = a.snapshot(5);
        assert_eq!(snap.total_requests, 11);
        assert_eq!(snap.total_throttled, 2);
        assert!((snap.throttle_rate - 2.0 / 11.0).abs() < 1e-9);
        assert_eq!(snap.top_endpoints[0].0, "/api/hot");
        assert_eq!(snap.top_users[0].0, "heavy");
    }

    #[test]
    fn empty_snapshot() {
        let snap = Analytics::new().snapshot(5);
        assert_eq!(snap.total_requests, 0);
        assert_eq!(snap.throttle_rate, 0.0);
        assert!(snap.top_endpoints.is_empty());
    }
}

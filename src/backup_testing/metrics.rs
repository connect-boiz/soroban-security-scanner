//! Recovery Time Objective (RTO) and Recovery Point Objective (RPO) tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// RTO/RPO policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpoRtoPolicy {
    /// Maximum acceptable data loss window in minutes.
    pub rpo_minutes: u32,
    /// Maximum acceptable recovery time in minutes.
    pub rto_minutes: u32,
    /// Maximum backup age before considered stale.
    pub max_backup_age_hours: u32,
}

impl Default for RpoRtoPolicy {
    fn default() -> Self {
        Self {
            rpo_minutes: 60,
            rto_minutes: 240,
            max_backup_age_hours: 24,
        }
    }
}

/// Metrics collected during backup and recovery operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryMetrics {
    pub backup_duration_ms: u64,
    pub recovery_duration_ms: u64,
    pub backup_size_bytes: usize,
    pub last_backup_at: DateTime<Utc>,
    pub last_recovery_test_at: Option<DateTime<Utc>>,
    pub success_rate_pct: f64,
    pub policy: RpoRtoPolicy,
}

impl RecoveryMetrics {
    pub fn new(policy: RpoRtoPolicy) -> Self {
        Self {
            backup_duration_ms: 0,
            recovery_duration_ms: 0,
            backup_size_bytes: 0,
            last_backup_at: Utc::now(),
            last_recovery_test_at: None,
            success_rate_pct: 100.0,
            policy,
        }
    }

    /// Check if recovery duration meets RTO target.
    pub fn meets_rto(&self) -> bool {
        let rto_ms = self.policy.rto_minutes as u64 * 60 * 1000;
        self.recovery_duration_ms <= rto_ms
    }

    /// Check if backup frequency meets RPO target.
    pub fn meets_rpo(&self, now: DateTime<Utc>) -> bool {
        let age_minutes = (now - self.last_backup_at).num_minutes().unsigned_abs();
        age_minutes <= self.policy.rpo_minutes as u64
    }

    /// Check if backup success rate meets 100% target.
    pub fn meets_success_target(&self) -> bool {
        self.success_rate_pct >= 100.0
    }

    pub fn record_backup(&mut self, duration_ms: u64, size_bytes: usize) {
        self.backup_duration_ms = duration_ms;
        self.backup_size_bytes = size_bytes;
        self.last_backup_at = Utc::now();
    }

    pub fn record_recovery_test(&mut self, duration_ms: u64, success: bool) {
        self.recovery_duration_ms = duration_ms;
        self.last_recovery_test_at = Some(Utc::now());
        if !success {
            self.success_rate_pct = 99.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn default_rto_rpo_policy() {
        let policy = RpoRtoPolicy::default();
        assert_eq!(policy.rpo_minutes, 60);
        assert_eq!(policy.rto_minutes, 240);
    }

    #[test]
    fn recovery_within_rto_passes() {
        let policy = RpoRtoPolicy {
            rto_minutes: 60,
            ..Default::default()
        };
        let mut metrics = RecoveryMetrics::new(policy);
        metrics.recovery_duration_ms = 30 * 60 * 1000; // 30 minutes
        assert!(metrics.meets_rto());
    }

    #[test]
    fn backup_within_rpo_passes() {
        let policy = RpoRtoPolicy::default();
        let metrics = RecoveryMetrics::new(policy);
        let now = Utc::now();
        assert!(metrics.meets_rpo(now));
    }

    #[test]
    fn stale_backup_fails_rpo() {
        let policy = RpoRtoPolicy {
            rpo_minutes: 30,
            ..Default::default()
        };
        let mut metrics = RecoveryMetrics::new(policy);
        metrics.last_backup_at = Utc::now() - Duration::hours(2);
        assert!(!metrics.meets_rpo(Utc::now()));
    }
}

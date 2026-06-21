//! Backup retention policies with automatic cleanup.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Retention tier for backup lifecycle management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionTier {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RetentionTier {
    pub fn retention_days(&self) -> i64 {
        match self {
            Self::Hourly => 1,
            Self::Daily => 30,
            Self::Weekly => 90,
            Self::Monthly => 365,
            Self::Yearly => 2555, // ~7 years
        }
    }
}

/// Retention policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub tiers: Vec<(RetentionTier, u32)>,
    pub auto_cleanup_enabled: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            tiers: vec![
                (RetentionTier::Hourly, 24),
                (RetentionTier::Daily, 30),
                (RetentionTier::Weekly, 12),
                (RetentionTier::Monthly, 12),
                (RetentionTier::Yearly, 7),
            ],
            auto_cleanup_enabled: true,
        }
    }
}

/// A backup record tracked by the retention policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub tier: RetentionTier,
    pub created_at: DateTime<Utc>,
    pub size_bytes: usize,
}

impl RetentionPolicy {
    /// Determine which backups should be cleaned up based on age and tier limits.
    pub fn backups_to_cleanup<'a>(
        &self,
        records: &'a [BackupRecord],
        now: DateTime<Utc>,
    ) -> Vec<&'a BackupRecord> {
        if !self.auto_cleanup_enabled {
            return Vec::new();
        }
        records
            .iter()
            .filter(|record| {
                let age_days = (now - record.created_at).num_days();
                age_days > record.tier.retention_days()
            })
            .collect()
    }

    /// Count backups per tier that exceed the configured limit.
    pub fn excess_backups<'a>(&self, records: &'a [BackupRecord]) -> Vec<&'a BackupRecord> {
        let mut excess = Vec::new();
        for (tier, max_count) in &self.tiers {
            let tier_records: Vec<_> = records.iter().filter(|r| r.tier == *tier).collect();
            if tier_records.len() > *max_count as usize {
                let sorted: Vec<_> = {
                    let mut v: Vec<_> = tier_records;
                    v.sort_by_key(|r| r.created_at);
                    v
                };
                let to_remove = sorted.len() - *max_count as usize;
                excess.extend(sorted.into_iter().take(to_remove));
            }
        }
        excess
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_record(id: &str, tier: RetentionTier, age: Duration) -> BackupRecord {
        BackupRecord {
            id: id.into(),
            tier,
            created_at: Utc::now() - age,
            size_bytes: 1024,
        }
    }

    #[test]
    fn expired_backups_identified_for_cleanup() {
        let policy = RetentionPolicy::default();
        let records = vec![
            make_record("old-hourly", RetentionTier::Hourly, Duration::days(3)),
            make_record("new-hourly", RetentionTier::Hourly, Duration::hours(6)),
        ];
        let cleanup = policy.backups_to_cleanup(&records, Utc::now());
        assert_eq!(cleanup.len(), 1);
        assert_eq!(cleanup[0].id, "old-hourly");
    }

    #[test]
    fn excess_daily_backups_identified() {
        let policy = RetentionPolicy::default();
        let records: Vec<BackupRecord> = (0..35)
            .map(|i| {
                make_record(
                    &format!("daily-{i}"),
                    RetentionTier::Daily,
                    Duration::days(i),
                )
            })
            .collect();
        let excess = policy.excess_backups(&records);
        assert_eq!(excess.len(), 5); // 35 - 30 max daily
    }

    #[test]
    fn auto_cleanup_disabled_returns_empty() {
        let policy = RetentionPolicy {
            auto_cleanup_enabled: false,
            ..Default::default()
        };
        let records = vec![make_record(
            "old",
            RetentionTier::Hourly,
            Duration::days(10),
        )];
        assert!(policy.backups_to_cleanup(&records, Utc::now()).is_empty());
    }
}

//! Backup notification system for success/failure alerts.

use crate::backup_testing::types::{BackupResult, RecoveryResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Notification delivery channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Slack,
    PagerDuty,
    Webhook,
}

/// A backup-related alert notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupAlert {
    pub id: String,
    pub channel: NotificationChannel,
    pub title: String,
    pub message: String,
    pub severity: AlertSeverity,
    pub timestamp: DateTime<Utc>,
    pub delivered: bool,
}

/// Alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// In-memory notification service for backup success/failure alerts.
#[derive(Debug, Clone, Default)]
pub struct BackupNotificationService {
    alerts: Vec<BackupAlert>,
}

impl BackupNotificationService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_backup_complete(&mut self, result: &BackupResult) -> BackupAlert {
        let (severity, title) = if result.success {
            (AlertSeverity::Info, "Backup succeeded")
        } else {
            (AlertSeverity::Critical, "Backup failed")
        };
        let alert = BackupAlert {
            id: format!("alert-backup-{}", result.artifact_id),
            channel: NotificationChannel::Slack,
            title: title.into(),
            message: result.message.clone(),
            severity,
            timestamp: Utc::now(),
            delivered: true,
        };
        self.alerts.push(alert.clone());
        alert
    }

    pub fn on_recovery_complete(&mut self, result: &RecoveryResult) -> BackupAlert {
        let (severity, title) = if result.success && result.data_matches {
            (AlertSeverity::Info, "Recovery test succeeded")
        } else {
            (AlertSeverity::Critical, "Recovery test failed")
        };
        let alert = BackupAlert {
            id: format!("alert-recovery-{}", result.artifact_id),
            channel: NotificationChannel::PagerDuty,
            title: title.into(),
            message: result.message.clone(),
            severity,
            timestamp: Utc::now(),
            delivered: true,
        };
        self.alerts.push(alert.clone());
        alert
    }

    pub fn all_alerts(&self) -> &[BackupAlert] {
        &self.alerts
    }

    pub fn critical_alerts(&self) -> Vec<&BackupAlert> {
        self.alerts
            .iter()
            .filter(|a| a.severity == AlertSeverity::Critical)
            .collect()
    }

    pub fn undelivered(&self) -> Vec<&BackupAlert> {
        self.alerts.iter().filter(|a| !a.delivered).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_success_generates_info_alert() {
        let mut svc = BackupNotificationService::new();
        let result = BackupResult {
            artifact_id: "bak-001".into(),
            success: true,
            duration_ms: 100,
            message: "backup completed".into(),
            timestamp: Utc::now(),
        };
        let alert = svc.on_backup_complete(&result);
        assert_eq!(alert.severity, AlertSeverity::Info);
    }

    #[test]
    fn backup_failure_generates_critical_alert() {
        let mut svc = BackupNotificationService::new();
        let result = BackupResult {
            artifact_id: "bak-002".into(),
            success: false,
            duration_ms: 50,
            message: "disk full".into(),
            timestamp: Utc::now(),
        };
        let alert = svc.on_backup_complete(&result);
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert_eq!(svc.critical_alerts().len(), 1);
    }

    #[test]
    fn recovery_failure_generates_critical_alert() {
        let mut svc = BackupNotificationService::new();
        let result = RecoveryResult {
            artifact_id: "bak-003".into(),
            success: false,
            duration_ms: 200,
            data_matches: false,
            message: "checksum mismatch".into(),
            timestamp: Utc::now(),
        };
        let alert = svc.on_recovery_complete(&result);
        assert_eq!(alert.severity, AlertSeverity::Critical);
    }
}

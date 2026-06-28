//! Security-header monitoring and misconfiguration alerting.
//!
//! Observes the grade of header sets served by the application and raises an
//! alert when a response would grade below a configured threshold — catching
//! misconfigurations (e.g. a deploy that weakened the CSP) in production and CI.

use crate::security_headers::builder::SecurityHeaders;
use crate::security_headers::grade::{evaluate, Grade, GradeReport};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// An emitted misconfiguration alert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeaderAlert {
    /// The grade observed.
    pub grade: Grade,
    /// Score observed.
    pub score: u32,
    /// Weaknesses that triggered the alert.
    pub weaknesses: Vec<String>,
}

/// Monitoring counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderStats {
    /// Configurations observed.
    pub observed: u64,
    /// Configurations meeting the threshold.
    pub passing: u64,
    /// Configurations below the threshold (alerted).
    pub failing: u64,
}

/// Monitors header configurations against a minimum acceptable grade.
pub struct HeaderMonitor {
    min_grade: Grade,
    observed: AtomicU64,
    passing: AtomicU64,
    failing: AtomicU64,
    alerts: Mutex<Vec<HeaderAlert>>,
}

impl HeaderMonitor {
    /// Creates a monitor that alerts below `min_grade`.
    pub fn new(min_grade: Grade) -> Self {
        Self {
            min_grade,
            observed: AtomicU64::new(0),
            passing: AtomicU64::new(0),
            failing: AtomicU64::new(0),
            alerts: Mutex::new(Vec::new()),
        }
    }

    /// Evaluates and records a header set, returning an alert if it grades below
    /// the threshold.
    pub fn observe(&self, headers: &SecurityHeaders) -> Option<HeaderAlert> {
        let report: GradeReport = evaluate(headers);
        self.observed.fetch_add(1, Ordering::Relaxed);
        if report.grade >= self.min_grade {
            self.passing.fetch_add(1, Ordering::Relaxed);
            None
        } else {
            self.failing.fetch_add(1, Ordering::Relaxed);
            let alert = HeaderAlert {
                grade: report.grade,
                score: report.score,
                weaknesses: report.weaknesses,
            };
            self.alerts
                .lock()
                .expect("alerts poisoned")
                .push(alert.clone());
            Some(alert)
        }
    }

    /// Current stats.
    pub fn stats(&self) -> HeaderStats {
        HeaderStats {
            observed: self.observed.load(Ordering::Relaxed),
            passing: self.passing.load(Ordering::Relaxed),
            failing: self.failing.load(Ordering::Relaxed),
        }
    }

    /// All raised alerts.
    pub fn alerts(&self) -> Vec<HeaderAlert> {
        self.alerts.lock().expect("alerts poisoned").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_headers::csp::ContentSecurityPolicy;

    #[test]
    fn strong_config_passes_without_alert() {
        let m = HeaderMonitor::new(Grade::A);
        let alert = m.observe(&SecurityHeaders::secure_default("n"));
        assert!(alert.is_none());
        assert_eq!(m.stats().passing, 1);
    }

    #[test]
    fn weak_config_alerts() {
        let m = HeaderMonitor::new(Grade::APlus);
        let mut headers = SecurityHeaders::secure_default("n");
        headers.csp = ContentSecurityPolicy::new(); // weaken
        let alert = m.observe(&headers).unwrap();
        assert!(alert.grade < Grade::APlus);
        assert!(!alert.weaknesses.is_empty());
        assert_eq!(m.stats().failing, 1);
        assert_eq!(m.alerts().len(), 1);
    }
}

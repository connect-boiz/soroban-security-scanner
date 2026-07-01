//! Log-based alerting for error patterns and anomalies.
//!
//! Two complementary detectors: a **pattern** rule that fires when a configured
//! substring appears in error/warn messages, and an **error-rate** rule that
//! fires when the error rate over a rolling window exceeds a threshold.

use crate::observability::level::LogLevel;
use crate::observability::record::LogRecord;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A raised log alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogAlert {
    /// Stable rule id.
    pub rule: String,
    /// Human-readable detail.
    pub detail: String,
}

/// A substring pattern to watch for in error/warn messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternRule {
    /// Rule id.
    pub id: String,
    /// Case-sensitive substring to match.
    pub needle: String,
}

/// Configuration for log alerting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Patterns that trigger an immediate alert when seen.
    pub patterns: Vec<PatternRule>,
    /// Error-rate threshold (0.0–1.0) over the rolling window.
    pub error_rate_threshold: f64,
    /// Rolling window size (number of recent records considered).
    pub window: usize,
    /// Minimum records in the window before the rate rule can fire.
    pub min_samples: usize,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            patterns: vec![
                PatternRule {
                    id: "panic".to_string(),
                    needle: "panic".to_string(),
                },
                PatternRule {
                    id: "oom".to_string(),
                    needle: "out of memory".to_string(),
                },
            ],
            error_rate_threshold: 0.25,
            window: 100,
            min_samples: 20,
        }
    }
}

/// Stateful log alert evaluator over a rolling window.
pub struct LogAlerter {
    config: AlertConfig,
    recent_levels: VecDeque<LogLevel>,
}

impl LogAlerter {
    /// Creates an alerter.
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            recent_levels: VecDeque::new(),
        }
    }

    /// Feeds a record, returning any alerts it triggers.
    pub fn observe(&mut self, record: &LogRecord) -> Vec<LogAlert> {
        let mut alerts = Vec::new();

        // Pattern rules apply to warn/error messages.
        if matches!(record.level, LogLevel::Warn | LogLevel::Error) {
            for rule in &self.config.patterns {
                if record.message.contains(&rule.needle) {
                    alerts.push(LogAlert {
                        rule: format!("pattern:{}", rule.id),
                        detail: format!("matched '{}' in: {}", rule.needle, record.message),
                    });
                }
            }
        }

        // Rolling error-rate rule.
        self.recent_levels.push_back(record.level);
        while self.recent_levels.len() > self.config.window {
            self.recent_levels.pop_front();
        }
        if self.recent_levels.len() >= self.config.min_samples {
            let errors = self
                .recent_levels
                .iter()
                .filter(|l| **l == LogLevel::Error)
                .count();
            let rate = errors as f64 / self.recent_levels.len() as f64;
            if rate > self.config.error_rate_threshold {
                alerts.push(LogAlert {
                    rule: "error-rate".to_string(),
                    detail: format!(
                        "error rate {:.0}% over last {} logs exceeds {:.0}%",
                        rate * 100.0,
                        self.recent_levels.len(),
                        self.config.error_rate_threshold * 100.0
                    ),
                });
            }
        }

        alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(level: LogLevel, msg: &str) -> LogRecord {
        LogRecord::new(1, level, "t", msg)
    }

    #[test]
    fn pattern_alert_on_error_message() {
        let mut a = LogAlerter::new(AlertConfig::default());
        let alerts = a.observe(&rec(LogLevel::Error, "thread panic at unwrap"));
        assert!(alerts.iter().any(|x| x.rule == "pattern:panic"));
    }

    #[test]
    fn pattern_ignored_on_info_level() {
        let mut a = LogAlerter::new(AlertConfig::default());
        let alerts = a.observe(&rec(LogLevel::Info, "panic word in info is noise"));
        assert!(alerts.is_empty());
    }

    #[test]
    fn error_rate_alert_fires_over_threshold() {
        let cfg = AlertConfig {
            patterns: vec![],
            error_rate_threshold: 0.25,
            window: 100,
            min_samples: 4,
        };
        let mut a = LogAlerter::new(cfg);
        a.observe(&rec(LogLevel::Info, "x"));
        a.observe(&rec(LogLevel::Info, "x"));
        a.observe(&rec(LogLevel::Error, "x"));
        let alerts = a.observe(&rec(LogLevel::Error, "x")); // 2/4 = 50% > 25%
        assert!(alerts.iter().any(|x| x.rule == "error-rate"));
    }

    #[test]
    fn no_rate_alert_before_min_samples() {
        let mut a = LogAlerter::new(AlertConfig::default()); // min_samples 20
        for _ in 0..5 {
            assert!(a
                .observe(&rec(LogLevel::Error, "x"))
                .iter()
                .all(|x| x.rule != "error-rate"));
        }
    }
}

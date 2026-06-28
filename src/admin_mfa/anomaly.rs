//! Session anomaly detection for admin logins.
//!
//! Scores each login attempt against the account's recent history across four
//! signals — IP address, device fingerprint, geographic location and
//! time-of-day — producing a risk level that drives an automatic step-up MFA
//! challenge for suspicious attempts.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// How many recent logins to retain per account for baselining.
const HISTORY_LIMIT: usize = 20;

/// A single login signal set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginContext {
    /// Source IP address (string form, v4 or v6).
    pub ip: String,
    /// Stable device/browser fingerprint.
    pub device_fingerprint: String,
    /// Coarse location code (e.g. ISO country, or "US-CA").
    pub location: String,
    /// Hour of day in UTC (0–23) of the attempt.
    pub hour_utc: u8,
}

/// Risk classification for a login attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Matches the established baseline.
    Low,
    /// Some novelty; warrants a step-up challenge.
    Medium,
    /// Strong novelty across multiple signals; always challenge.
    High,
}

/// The result of scoring a login.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalyAssessment {
    /// Numeric score in `[0.0, 1.0]`.
    pub score: f64,
    /// Bucketed risk level.
    pub level: RiskLevel,
    /// Human-readable reasons contributing to the score.
    pub reasons: Vec<String>,
}

impl AnomalyAssessment {
    /// Whether this assessment should trigger an automatic MFA challenge.
    pub fn requires_challenge(&self) -> bool {
        self.level >= RiskLevel::Medium
    }
}

/// Tunable thresholds for the detector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnomalyConfig {
    /// Score at/above which risk is Medium.
    pub medium_threshold: f64,
    /// Score at/above which risk is High.
    pub high_threshold: f64,
    /// Hours of deviation from the usual login time treated as unusual.
    pub unusual_hour_delta: u8,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            medium_threshold: 0.3,
            high_threshold: 0.6,
            unusual_hour_delta: 6,
        }
    }
}

/// Detects anomalies against a rolling per-account history.
#[derive(Debug, Clone)]
pub struct AnomalyDetector {
    config: AnomalyConfig,
    history: VecDeque<LoginContext>,
}

impl AnomalyDetector {
    /// Creates a detector with the given configuration and no history.
    pub fn new(config: AnomalyConfig) -> Self {
        Self {
            config,
            history: VecDeque::new(),
        }
    }

    /// Number of historical logins retained.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Scores `ctx` against history **without** recording it.
    pub fn assess(&self, ctx: &LoginContext) -> AnomalyAssessment {
        // With no baseline yet, the first login is treated as elevated so the
        // account establishes MFA from the outset.
        if self.history.is_empty() {
            return AnomalyAssessment {
                score: self.config.medium_threshold,
                level: RiskLevel::Medium,
                reasons: vec!["no login history to baseline against".to_string()],
            };
        }

        let mut score: f64 = 0.0;
        let mut reasons = Vec::new();

        if !self.history.iter().any(|h| h.ip == ctx.ip) {
            score += 0.30;
            reasons.push("new IP address".to_string());
        }
        if !self
            .history
            .iter()
            .any(|h| h.device_fingerprint == ctx.device_fingerprint)
        {
            score += 0.30;
            reasons.push("unrecognized device".to_string());
        }
        if !self.history.iter().any(|h| h.location == ctx.location) {
            score += 0.30;
            reasons.push("new location".to_string());
        }
        if self.is_unusual_hour(ctx.hour_utc) {
            score += 0.15;
            reasons.push("unusual login hour".to_string());
        }

        let score = score.min(1.0);
        let level = if score >= self.config.high_threshold {
            RiskLevel::High
        } else if score >= self.config.medium_threshold {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        AnomalyAssessment {
            score,
            level,
            reasons,
        }
    }

    /// Records a (presumably accepted) login into the rolling history.
    pub fn record(&mut self, ctx: LoginContext) {
        self.history.push_back(ctx);
        while self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
    }

    /// Convenience: assess then record in one call.
    pub fn assess_and_record(&mut self, ctx: LoginContext) -> AnomalyAssessment {
        let assessment = self.assess(&ctx);
        self.record(ctx);
        assessment
    }

    fn is_unusual_hour(&self, hour: u8) -> bool {
        // Compare against the circular-mean-ish set of known hours: unusual if
        // it is more than `unusual_hour_delta` from every historical hour.
        let delta = self.config.unusual_hour_delta;
        self.history
            .iter()
            .all(|h| circular_hour_distance(h.hour_utc, hour) > delta)
    }
}

/// Distance between two hours on a 24-hour clock (0–12).
fn circular_hour_distance(a: u8, b: u8) -> u8 {
    let diff = (a as i16 - b as i16).unsigned_abs() as u8;
    diff.min(24 - diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(ip: &str, device: &str, location: &str, hour: u8) -> LoginContext {
        LoginContext {
            ip: ip.to_string(),
            device_fingerprint: device.to_string(),
            location: location.to_string(),
            hour_utc: hour,
        }
    }

    fn baseline() -> AnomalyDetector {
        let mut d = AnomalyDetector::new(AnomalyConfig::default());
        d.record(ctx("203.0.113.1", "device-A", "US-CA", 9));
        d.record(ctx("203.0.113.1", "device-A", "US-CA", 10));
        d
    }

    #[test]
    fn first_login_is_elevated() {
        let d = AnomalyDetector::new(AnomalyConfig::default());
        let a = d.assess(&ctx("203.0.113.1", "device-A", "US-CA", 9));
        assert!(a.requires_challenge());
    }

    #[test]
    fn familiar_login_is_low_risk() {
        let d = baseline();
        let a = d.assess(&ctx("203.0.113.1", "device-A", "US-CA", 9));
        assert_eq!(a.level, RiskLevel::Low);
        assert!(!a.requires_challenge());
    }

    #[test]
    fn new_ip_only_is_medium() {
        let d = baseline();
        let a = d.assess(&ctx("198.51.100.9", "device-A", "US-CA", 9));
        assert_eq!(a.level, RiskLevel::Medium);
        assert!(a.reasons.iter().any(|r| r.contains("IP")));
    }

    #[test]
    fn new_ip_device_and_location_is_high() {
        let d = baseline();
        let a = d.assess(&ctx("8.8.8.8", "device-Z", "RU-MOW", 3));
        assert_eq!(a.level, RiskLevel::High);
        assert!(a.requires_challenge());
    }

    #[test]
    fn unusual_hour_contributes() {
        let d = baseline(); // usual hours 9–10
        let a = d.assess(&ctx("203.0.113.1", "device-A", "US-CA", 2));
        assert!(a.reasons.iter().any(|r| r.contains("hour")));
    }

    #[test]
    fn history_is_capped() {
        let mut d = AnomalyDetector::new(AnomalyConfig::default());
        for i in 0..(HISTORY_LIMIT + 5) {
            d.record(ctx("203.0.113.1", "device-A", "US-CA", (i % 24) as u8));
        }
        assert_eq!(d.history_len(), HISTORY_LIMIT);
    }

    #[test]
    fn circular_distance_wraps_midnight() {
        assert_eq!(circular_hour_distance(23, 1), 2);
        assert_eq!(circular_hour_distance(0, 12), 12);
    }
}

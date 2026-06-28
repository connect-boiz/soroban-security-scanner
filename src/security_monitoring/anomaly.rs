//! ML-based security anomaly detection.
//!
//! An online statistical model (Welford running mean/variance) that learns the
//! normal rate of an activity per subject and flags observations that deviate
//! beyond a configurable number of standard deviations (z-score). This is the
//! unsupervised "is this behaviour unusual for *this* actor" detector that
//! complements the fixed-threshold rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the anomaly detector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnomalyConfig {
    /// z-score magnitude beyond which an observation is anomalous.
    pub z_threshold: f64,
    /// Minimum samples before scoring (avoids cold-start false positives).
    pub min_samples: u64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            z_threshold: 3.0,
            min_samples: 10,
        }
    }
}

/// Welford online statistics for one subject.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct RunningStats {
    count: u64,
    mean: f64,
    m2: f64,
}

impl RunningStats {
    fn update(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
    }

    fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }
}

/// The result of scoring an observation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnomalyScore {
    /// z-score of the observation against the subject's baseline.
    pub z: f64,
    /// Whether it exceeds the configured threshold.
    pub anomalous: bool,
}

/// Per-subject behavioural anomaly detector.
#[derive(Default)]
pub struct AnomalyDetector {
    config: AnomalyConfig,
    stats: HashMap<String, RunningStats>,
}

impl AnomalyDetector {
    /// Creates a detector.
    pub fn new(config: AnomalyConfig) -> Self {
        Self {
            config,
            stats: HashMap::new(),
        }
    }

    /// Scores `value` for `subject` against its baseline, then folds the value
    /// into the model. Returns `None` until enough samples exist.
    pub fn observe(&mut self, subject: &str, value: f64) -> Option<AnomalyScore> {
        let stats = self.stats.entry(subject.to_string()).or_default();

        let score = if stats.count >= self.config.min_samples {
            let sd = stats.stddev();
            // With zero variance, any change from the mean is maximally unusual.
            let z = if sd == 0.0 {
                if (value - stats.mean).abs() > f64::EPSILON {
                    f64::INFINITY
                } else {
                    0.0
                }
            } else {
                (value - stats.mean) / sd
            };
            Some(AnomalyScore {
                z,
                anomalous: z.abs() >= self.config.z_threshold,
            })
        } else {
            None
        };

        stats.update(value);
        score
    }

    /// Number of distinct subjects modelled.
    pub fn subjects(&self) -> usize {
        self.stats.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_start_returns_none_until_min_samples() {
        let mut d = AnomalyDetector::new(AnomalyConfig::default()); // min 10
        for _ in 0..10 {
            assert!(d.observe("alice", 1.0).is_none());
        }
        // 11th observation is scored.
        assert!(d.observe("alice", 1.0).is_some());
    }

    #[test]
    fn stable_behaviour_is_not_anomalous() {
        let mut d = AnomalyDetector::new(AnomalyConfig::default());
        for _ in 0..20 {
            d.observe("alice", 10.0);
        }
        // Slight noise around the mean is fine.
        let s = d.observe("alice", 10.0).unwrap();
        assert!(!s.anomalous);
    }

    #[test]
    fn spike_is_flagged() {
        let mut d = AnomalyDetector::new(AnomalyConfig::default());
        // Baseline around 5 requests with small variance.
        for i in 0..30 {
            d.observe("api", 5.0 + (i % 3) as f64 * 0.1);
        }
        // A sudden burst of 500 requests is a clear outlier.
        let s = d.observe("api", 500.0).unwrap();
        assert!(s.anomalous);
        assert!(s.z > 3.0);
    }

    #[test]
    fn zero_variance_then_change_is_infinite_z() {
        let mut d = AnomalyDetector::new(AnomalyConfig::default());
        for _ in 0..15 {
            d.observe("x", 1.0);
        }
        let s = d.observe("x", 2.0).unwrap();
        assert!(s.anomalous);
        assert!(s.z.is_infinite());
    }

    #[test]
    fn subjects_are_independent() {
        let mut d = AnomalyDetector::new(AnomalyConfig::default());
        for _ in 0..15 {
            d.observe("a", 1.0);
        }
        assert!(d.observe("b", 9999.0).is_none()); // b has no baseline yet
        assert_eq!(d.subjects(), 2);
    }
}

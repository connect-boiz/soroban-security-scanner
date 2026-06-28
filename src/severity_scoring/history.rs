//! Severity trend analysis and historical tracking.
//!
//! Records time-stamped severity samples per finding so the platform can show
//! how a vulnerability's risk has evolved (e.g. as TVL grows or a contract is
//! promoted from testnet to mainnet) and surface trend direction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::engine::SeverityScore;
use crate::Severity;

/// One recorded severity observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreSample {
    pub timestamp: u64,
    pub score: f64,
    pub severity: Severity,
}

/// Direction of a severity trend over the recorded window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Summary statistics for a finding's score history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub id: String,
    pub sample_count: usize,
    pub first_score: f64,
    pub latest_score: f64,
    pub min_score: f64,
    pub max_score: f64,
    pub average_score: f64,
    /// `latest - first`.
    pub delta: f64,
    pub direction: TrendDirection,
}

/// In-memory store of per-finding severity history.
#[derive(Debug, Clone, Default)]
pub struct SeverityHistory {
    samples: HashMap<String, Vec<ScoreSample>>,
    /// Scores within this absolute distance are treated as "unchanged" when
    /// classifying trend direction.
    stability_epsilon: f64,
}

impl SeverityHistory {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
            stability_epsilon: 0.1,
        }
    }

    /// Record a severity observation for `id` at `timestamp`.
    pub fn record(&mut self, id: impl Into<String>, score: &SeverityScore, timestamp: u64) {
        self.samples
            .entry(id.into())
            .or_default()
            .push(ScoreSample {
                timestamp,
                score: score.contextual_score,
                severity: score.severity,
            });
    }

    /// All samples for a finding, in insertion order.
    pub fn samples(&self, id: &str) -> Option<&[ScoreSample]> {
        self.samples.get(id).map(|v| v.as_slice())
    }

    /// Compute trend statistics for a finding, if it has any samples.
    pub fn trend(&self, id: &str) -> Option<TrendAnalysis> {
        let samples = self.samples.get(id)?;
        if samples.is_empty() {
            return None;
        }

        // Order by time so first/latest are meaningful even if recorded loosely.
        let mut ordered: Vec<&ScoreSample> = samples.iter().collect();
        ordered.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let first = ordered.first().unwrap().score;
        let latest = ordered.last().unwrap().score;
        let min = ordered
            .iter()
            .map(|s| s.score)
            .fold(f64::INFINITY, f64::min);
        let max = ordered
            .iter()
            .map(|s| s.score)
            .fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = ordered.iter().map(|s| s.score).sum();
        let average = sum / ordered.len() as f64;
        let delta = latest - first;

        let direction = if delta.abs() <= self.stability_epsilon {
            TrendDirection::Stable
        } else if delta > 0.0 {
            TrendDirection::Increasing
        } else {
            TrendDirection::Decreasing
        };

        Some(TrendAnalysis {
            id: id.to_string(),
            sample_count: ordered.len(),
            first_score: first,
            latest_score: latest,
            min_score: min,
            max_score: max,
            average_score: average,
            delta,
            direction,
        })
    }
}

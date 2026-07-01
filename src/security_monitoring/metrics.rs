//! Security metrics: MTTD, MTTR and posture score.
//!
//! Aggregates incident timing into the headline operational metrics and
//! computes a 0–100 security-posture score that degrades with open
//! high-severity incidents and slow detection/response.

use crate::security_monitoring::incident::{Incident, IncidentStatus};
use serde::{Deserialize, Serialize};

/// Aggregated security metrics over a set of incidents.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Total incidents considered.
    pub total_incidents: usize,
    /// Currently open (unresolved) incidents.
    pub open_incidents: usize,
    /// Mean time to detect, in seconds (over all incidents).
    pub mttd_secs: f64,
    /// Mean time to resolve, in seconds (over resolved incidents).
    pub mttr_secs: f64,
    /// Whether MTTD meets the <5 minute (300s) critical target.
    pub meets_mttd_target: bool,
    /// Security posture score, 0 (poor) – 100 (excellent).
    pub posture_score: f64,
}

/// The MTTD target for critical events, in seconds (5 minutes).
pub const MTTD_TARGET_SECS: f64 = 300.0;

/// Computes metrics from a slice of incidents.
pub fn compute_metrics(incidents: &[Incident]) -> SecurityMetrics {
    let total = incidents.len();
    if total == 0 {
        return SecurityMetrics {
            total_incidents: 0,
            open_incidents: 0,
            mttd_secs: 0.0,
            mttr_secs: 0.0,
            meets_mttd_target: true,
            posture_score: 100.0,
        };
    }

    let mttd_sum: i64 = incidents.iter().map(|i| i.mttd_secs()).sum();
    let mttd = mttd_sum as f64 / total as f64;

    let resolved: Vec<i64> = incidents.iter().filter_map(|i| i.mttr_secs()).collect();
    let mttr = if resolved.is_empty() {
        0.0
    } else {
        resolved.iter().sum::<i64>() as f64 / resolved.len() as f64
    };

    let open = incidents
        .iter()
        .filter(|i| i.status != IncidentStatus::Resolved)
        .count();

    SecurityMetrics {
        total_incidents: total,
        open_incidents: open,
        mttd_secs: mttd,
        mttr_secs: mttr,
        meets_mttd_target: mttd <= MTTD_TARGET_SECS,
        posture_score: posture_score(incidents, mttd),
    }
}

/// Posture score in 0–100: starts at 100 and is penalized for unresolved
/// high/critical incidents and for missing the MTTD target.
fn posture_score(incidents: &[Incident], mttd: f64) -> f64 {
    use crate::security_monitoring::event::SecuritySeverity;
    let mut score: f64 = 100.0;

    for inc in incidents {
        if inc.status != IncidentStatus::Resolved {
            score -= match inc.severity {
                SecuritySeverity::Critical => 20.0,
                SecuritySeverity::High => 10.0,
                SecuritySeverity::Medium => 4.0,
                SecuritySeverity::Low => 1.0,
                SecuritySeverity::Info => 0.0,
            };
        }
    }

    // Detection-speed penalty: up to 20 points as MTTD approaches/exceeds 2x target.
    if mttd > MTTD_TARGET_SECS {
        let overage = ((mttd - MTTD_TARGET_SECS) / MTTD_TARGET_SECS).min(1.0);
        score -= 20.0 * overage;
    }

    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_monitoring::detection::Finding;
    use crate::security_monitoring::event::SecuritySeverity;

    fn incident(sev: SecuritySeverity, first: i64, detected: i64) -> Incident {
        Incident::open(
            &Finding {
                rule: "r".to_string(),
                subject: "s".to_string(),
                severity: sev,
                detail: "d".to_string(),
                at: first,
            },
            first,
            detected,
        )
    }

    #[test]
    fn empty_is_perfect_posture() {
        let m = compute_metrics(&[]);
        assert_eq!(m.posture_score, 100.0);
        assert!(m.meets_mttd_target);
    }

    #[test]
    fn fast_detection_meets_target() {
        let incidents = vec![incident(SecuritySeverity::High, 1000, 1100)]; // 100s
        let m = compute_metrics(&incidents);
        assert_eq!(m.mttd_secs, 100.0);
        assert!(m.meets_mttd_target);
    }

    #[test]
    fn slow_detection_misses_target_and_lowers_posture() {
        let incidents = vec![incident(SecuritySeverity::High, 1000, 1000 + 600)]; // 600s > 300
        let m = compute_metrics(&incidents);
        assert!(!m.meets_mttd_target);
        assert!(m.posture_score < 90.0); // open high incident + slow detection
    }

    #[test]
    fn resolved_incidents_improve_posture_and_yield_mttr() {
        let mut inc = incident(SecuritySeverity::Critical, 1000, 1100);
        inc.resolve(1700);
        let m = compute_metrics(&[inc]);
        assert_eq!(m.open_incidents, 0);
        assert_eq!(m.mttr_secs, 600.0);
        // Resolved → no open-incident penalty; fast MTTD → full score.
        assert_eq!(m.posture_score, 100.0);
    }

    #[test]
    fn open_critical_incident_penalized() {
        let m = compute_metrics(&[incident(SecuritySeverity::Critical, 1000, 1100)]);
        assert_eq!(m.open_incidents, 1);
        assert!((m.posture_score - 80.0).abs() < 1e-9); // -20 for open critical
    }
}

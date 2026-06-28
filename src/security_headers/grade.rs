//! Security-header evaluation and grading.
//!
//! Scores a [`SecurityHeaders`] set the way an external evaluation tool would,
//! awarding an A+…F grade and listing concrete weaknesses — the basis for both
//! the "A+ rating" goal and the misconfiguration alerting.

use crate::security_headers::builder::SecurityHeaders;
use crate::security_headers::headers::FrameOptions;
use serde::{Deserialize, Serialize};

/// A letter grade for a header configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Grade {
    /// Failing.
    F,
    /// Significant gaps.
    C,
    /// Minor gaps.
    B,
    /// Strong.
    A,
    /// Best practice.
    APlus,
}

impl Grade {
    /// Stable label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Grade::F => "F",
            Grade::C => "C",
            Grade::B => "B",
            Grade::A => "A",
            Grade::APlus => "A+",
        }
    }
}

/// A grading report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradeReport {
    /// Score out of 100.
    pub score: u32,
    /// Letter grade.
    pub grade: Grade,
    /// Weaknesses found (empty for A+).
    pub weaknesses: Vec<String>,
}

/// Minimum HSTS max-age for full credit (6 months).
pub const MIN_HSTS_MAX_AGE: u64 = 15_768_000;

/// Evaluates a header set and returns a grade report.
pub fn evaluate(headers: &SecurityHeaders) -> GradeReport {
    let mut score: u32 = 0;
    let mut weaknesses = Vec::new();

    // CSP: 35 points, must be strict.
    if headers.csp.is_strict() {
        score += 35;
    } else {
        weaknesses.push(
            "CSP is not strict (missing default-src/object-src/frame-ancestors lockdown)"
                .to_string(),
        );
    }

    // HSTS: 25 points (max-age + includeSubDomains).
    if headers.hsts.max_age_secs >= MIN_HSTS_MAX_AGE && headers.hsts.include_subdomains {
        score += 25;
    } else {
        weaknesses.push("HSTS max-age too short or missing includeSubDomains".to_string());
    }

    // X-Frame-Options: 15 points.
    if matches!(
        headers.frame_options,
        FrameOptions::Deny | FrameOptions::SameOrigin
    ) {
        score += 15;
    }

    // X-Content-Type-Options is always nosniff in this type: 10 points.
    score += 10;

    // Referrer-Policy: 8 points.
    score += 8;

    // Permissions-Policy: 7 points if configured.
    if !headers.permissions_policy.is_empty() {
        score += 7;
    } else {
        weaknesses.push("Permissions-Policy is empty".to_string());
    }

    let grade = grade_for(score);
    GradeReport {
        score,
        grade,
        weaknesses,
    }
}

fn grade_for(score: u32) -> Grade {
    match score {
        100 => Grade::APlus,
        90..=99 => Grade::A,
        75..=89 => Grade::B,
        50..=74 => Grade::C,
        _ => Grade::F,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_default_scores_a_plus() {
        let report = evaluate(&SecurityHeaders::secure_default("nonce"));
        assert_eq!(report.score, 100);
        assert_eq!(report.grade, Grade::APlus);
        assert!(report.weaknesses.is_empty());
    }

    #[test]
    fn weak_csp_loses_points_and_is_flagged() {
        let mut headers = SecurityHeaders::secure_default("nonce");
        headers.csp = crate::security_headers::csp::ContentSecurityPolicy::new(); // empty → not strict
        let report = evaluate(&headers);
        assert!(report.score < 100);
        assert_ne!(report.grade, Grade::APlus);
        assert!(report.weaknesses.iter().any(|w| w.contains("CSP")));
    }

    #[test]
    fn short_hsts_flagged() {
        let mut headers = SecurityHeaders::secure_default("n");
        headers.hsts.max_age_secs = 100;
        let report = evaluate(&headers);
        assert!(report.weaknesses.iter().any(|w| w.contains("HSTS")));
        assert!(report.score <= 75);
    }

    #[test]
    fn grade_thresholds() {
        assert_eq!(grade_for(100), Grade::APlus);
        assert_eq!(grade_for(95), Grade::A);
        assert_eq!(grade_for(80), Grade::B);
        assert_eq!(grade_for(60), Grade::C);
        assert_eq!(grade_for(10), Grade::F);
    }
}

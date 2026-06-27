//! The dynamic severity engine.
//!
//! Combines the intrinsic CVSS v3.1 base score with the contextual multiplier
//! to produce a *contextual* severity score, a qualitative rating, and a full
//! breakdown of contributing factors. Also supports real-time recalculation
//! when a contract's state (and therefore its [`RiskContext`]) changes.

use serde::{Deserialize, Serialize};

use super::context::{FactorContribution, RiskContext};
use super::cvss::{roundup, CvssRating, CvssV31};
use crate::Severity;

/// A fully-computed severity assessment with a transparent breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeverityScore {
    /// Intrinsic CVSS v3.1 base score (0.0–10.0).
    pub cvss_base_score: f64,
    /// Qualitative CVSS rating of the base score.
    pub cvss_rating: CvssRating,
    /// Canonical CVSS vector string.
    pub cvss_vector: String,
    /// Aggregate contextual multiplier applied to the base score.
    pub context_multiplier: f64,
    /// Final context-adjusted score (0.0–10.0).
    pub contextual_score: f64,
    /// Qualitative rating of the contextual score.
    pub contextual_rating: CvssRating,
    /// Project-wide [`Severity`] label derived from the contextual score.
    pub severity: Severity,
    /// Per-factor contributions (CVSS base + each contextual factor).
    pub factors: Vec<FactorContribution>,
}

impl SeverityScore {
    /// Whether the contextual rating differs from the intrinsic CVSS rating —
    /// i.e. context materially changed the picture.
    pub fn context_shifted_rating(&self) -> bool {
        self.cvss_rating != self.contextual_rating
    }
}

/// Map a CVSS qualitative rating onto the crate-wide [`Severity`] enum, which
/// has no `None` variant (treated as `Low`).
pub fn rating_to_severity(rating: CvssRating) -> Severity {
    match rating {
        CvssRating::None | CvssRating::Low => Severity::Low,
        CvssRating::Medium => Severity::Medium,
        CvssRating::High => Severity::High,
        CvssRating::Critical => Severity::Critical,
    }
}

/// Stateless engine that turns CVSS metrics + context into a [`SeverityScore`].
#[derive(Debug, Clone, Default)]
pub struct SeverityEngine;

impl SeverityEngine {
    pub fn new() -> Self {
        Self
    }

    /// Compute a contextual severity score.
    pub fn score(&self, cvss: &CvssV31, context: &RiskContext) -> SeverityScore {
        let cvss_base = cvss.base_score();
        let multiplier = context.aggregate_multiplier();
        let contextual = roundup((cvss_base * multiplier).clamp(0.0, 10.0));
        let contextual_rating = CvssRating::from_score(contextual);

        // Build the factor breakdown: lead with the CVSS base as a factor, then
        // append each contextual factor.
        let mut factors = Vec::with_capacity(7);
        factors.push(FactorContribution {
            name: "cvss_base".to_string(),
            value: format!("{:.1}", cvss_base),
            multiplier: 1.0,
        });
        factors.extend(context.breakdown());

        SeverityScore {
            cvss_base_score: cvss_base,
            cvss_rating: cvss.rating(),
            cvss_vector: cvss.vector_string(),
            context_multiplier: multiplier,
            contextual_score: contextual,
            contextual_rating,
            severity: rating_to_severity(contextual_rating),
            factors,
        }
    }
}

/// A vulnerability finding tracked over time so its severity can be recomputed
/// as the contract's context changes (real-time recalculation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredFinding {
    /// Stable identifier of the finding.
    pub id: String,
    /// Intrinsic CVSS metrics (constant for a given finding).
    pub cvss: CvssV31,
    /// Current context (mutable as the deployment evolves).
    pub context: RiskContext,
    /// Most recently computed score.
    pub current: SeverityScore,
}

impl ScoredFinding {
    /// Create and score a new finding.
    pub fn new(id: impl Into<String>, cvss: CvssV31, context: RiskContext) -> Self {
        let current = SeverityEngine::new().score(&cvss, &context);
        Self {
            id: id.into(),
            cvss,
            context,
            current,
        }
    }

    /// Recalculate severity for a new context (e.g. after a contract-state
    /// change). Returns the outcome describing whether and how severity moved.
    pub fn recalculate(&mut self, new_context: RiskContext) -> RecalcOutcome {
        let previous = self.current.clone();
        self.context = new_context;
        self.current = SeverityEngine::new().score(&self.cvss, &new_context);

        let previous_score = previous.contextual_score;
        let new_score = self.current.contextual_score;
        RecalcOutcome {
            id: self.id.clone(),
            previous_severity: previous.severity,
            new_severity: self.current.severity,
            previous_score,
            new_score,
            changed: previous.severity != self.current.severity
                || (previous_score - new_score).abs() > f64::EPSILON,
            escalated: new_score > previous_score,
        }
    }
}

/// The result of a real-time severity recalculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecalcOutcome {
    pub id: String,
    pub previous_severity: Severity,
    pub new_severity: Severity,
    pub previous_score: f64,
    pub new_score: f64,
    /// Whether the score or severity changed at all.
    pub changed: bool,
    /// Whether the new score is higher than the previous (risk escalated).
    pub escalated: bool,
}

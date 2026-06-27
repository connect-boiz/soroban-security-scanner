//! CVSS v3.1 base-score engine.
//!
//! A faithful implementation of the FIRST CVSS v3.1 specification base-metric
//! group, including the exact `Roundup` function defined in the spec. The
//! engine produces a 0.0–10.0 base score, the qualitative rating, and the
//! canonical vector string.
//!
//! Reference: <https://www.first.org/cvss/v3.1/specification-document>

use serde::{Deserialize, Serialize};

/// Attack Vector (AV): how the vulnerability is exploited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackVector {
    Network,
    Adjacent,
    Local,
    Physical,
}

impl AttackVector {
    fn weight(&self) -> f64 {
        match self {
            AttackVector::Network => 0.85,
            AttackVector::Adjacent => 0.62,
            AttackVector::Local => 0.55,
            AttackVector::Physical => 0.20,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AttackVector::Network => "N",
            AttackVector::Adjacent => "A",
            AttackVector::Local => "L",
            AttackVector::Physical => "P",
        }
    }
}

/// Attack Complexity (AC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackComplexity {
    Low,
    High,
}

impl AttackComplexity {
    fn weight(&self) -> f64 {
        match self {
            AttackComplexity::Low => 0.77,
            AttackComplexity::High => 0.44,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AttackComplexity::Low => "L",
            AttackComplexity::High => "H",
        }
    }
}

/// Privileges Required (PR). Its weight depends on whether scope changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivilegesRequired {
    None,
    Low,
    High,
}

impl PrivilegesRequired {
    fn weight(&self, scope_changed: bool) -> f64 {
        match self {
            PrivilegesRequired::None => 0.85,
            PrivilegesRequired::Low => {
                if scope_changed {
                    0.68
                } else {
                    0.62
                }
            }
            PrivilegesRequired::High => {
                if scope_changed {
                    0.50
                } else {
                    0.27
                }
            }
        }
    }

    fn code(&self) -> &'static str {
        match self {
            PrivilegesRequired::None => "N",
            PrivilegesRequired::Low => "L",
            PrivilegesRequired::High => "H",
        }
    }
}

/// User Interaction (UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserInteraction {
    None,
    Required,
}

impl UserInteraction {
    fn weight(&self) -> f64 {
        match self {
            UserInteraction::None => 0.85,
            UserInteraction::Required => 0.62,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            UserInteraction::None => "N",
            UserInteraction::Required => "R",
        }
    }
}

/// Scope (S): whether an exploit can affect resources beyond the vulnerable
/// component's security authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Unchanged,
    Changed,
}

impl Scope {
    fn changed(&self) -> bool {
        matches!(self, Scope::Changed)
    }

    fn code(&self) -> &'static str {
        match self {
            Scope::Unchanged => "U",
            Scope::Changed => "C",
        }
    }
}

/// Impact metric value (used for Confidentiality, Integrity, Availability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Impact {
    None,
    Low,
    High,
}

impl Impact {
    fn weight(&self) -> f64 {
        match self {
            Impact::None => 0.0,
            Impact::Low => 0.22,
            Impact::High => 0.56,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Impact::None => "N",
            Impact::Low => "L",
            Impact::High => "H",
        }
    }
}

/// Qualitative severity rating per the CVSS v3.1 rating scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CvssRating {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl CvssRating {
    /// Map a base score to its qualitative rating (CVSS v3.1 §5).
    pub fn from_score(score: f64) -> Self {
        if score <= 0.0 {
            CvssRating::None
        } else if score < 4.0 {
            CvssRating::Low
        } else if score < 7.0 {
            CvssRating::Medium
        } else if score < 9.0 {
            CvssRating::High
        } else {
            CvssRating::Critical
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CvssRating::None => "NONE",
            CvssRating::Low => "LOW",
            CvssRating::Medium => "MEDIUM",
            CvssRating::High => "HIGH",
            CvssRating::Critical => "CRITICAL",
        }
    }
}

/// The CVSS v3.1 base-metric group for a single vulnerability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CvssV31 {
    pub attack_vector: AttackVector,
    pub attack_complexity: AttackComplexity,
    pub privileges_required: PrivilegesRequired,
    pub user_interaction: UserInteraction,
    pub scope: Scope,
    pub confidentiality: Impact,
    pub integrity: Impact,
    pub availability: Impact,
}

impl CvssV31 {
    /// Construct a metric group.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        attack_vector: AttackVector,
        attack_complexity: AttackComplexity,
        privileges_required: PrivilegesRequired,
        user_interaction: UserInteraction,
        scope: Scope,
        confidentiality: Impact,
        integrity: Impact,
        availability: Impact,
    ) -> Self {
        Self {
            attack_vector,
            attack_complexity,
            privileges_required,
            user_interaction,
            scope,
            confidentiality,
            integrity,
            availability,
        }
    }

    /// Impact Sub-Score (ISS).
    fn impact_sub_score(&self) -> f64 {
        let c = self.confidentiality.weight();
        let i = self.integrity.weight();
        let a = self.availability.weight();
        1.0 - ((1.0 - c) * (1.0 - i) * (1.0 - a))
    }

    /// Impact term, dependent on scope.
    fn impact(&self) -> f64 {
        let iss = self.impact_sub_score();
        if self.scope.changed() {
            7.52 * (iss - 0.029) - 3.25 * (iss - 0.02).powi(15)
        } else {
            6.42 * iss
        }
    }

    /// Exploitability term.
    fn exploitability(&self) -> f64 {
        8.22
            * self.attack_vector.weight()
            * self.attack_complexity.weight()
            * self
                .privileges_required
                .weight(self.scope.changed())
            * self.user_interaction.weight()
    }

    /// The CVSS v3.1 base score (0.0–10.0).
    pub fn base_score(&self) -> f64 {
        let impact = self.impact();
        if impact <= 0.0 {
            return 0.0;
        }
        let exploitability = self.exploitability();
        let raw = if self.scope.changed() {
            (1.08 * (impact + exploitability)).min(10.0)
        } else {
            (impact + exploitability).min(10.0)
        };
        roundup(raw)
    }

    /// Qualitative rating for the base score.
    pub fn rating(&self) -> CvssRating {
        CvssRating::from_score(self.base_score())
    }

    /// Canonical CVSS v3.1 vector string.
    pub fn vector_string(&self) -> String {
        format!(
            "CVSS:3.1/AV:{}/AC:{}/PR:{}/UI:{}/S:{}/C:{}/I:{}/A:{}",
            self.attack_vector.code(),
            self.attack_complexity.code(),
            self.privileges_required.code(),
            self.user_interaction.code(),
            self.scope.code(),
            self.confidentiality.code(),
            self.integrity.code(),
            self.availability.code(),
        )
    }
}

/// The CVSS v3.1 `Roundup` function (specification appendix A).
///
/// Rounds up to one decimal place, but only when there is a non-zero remainder
/// beyond the first decimal — implemented with integer arithmetic to avoid
/// floating-point drift, exactly as the spec mandates.
pub fn roundup(input: f64) -> f64 {
    let int_input = (input * 100_000.0).round() as i64;
    if int_input % 10_000 == 0 {
        int_input as f64 / 100_000.0
    } else {
        ((int_input as f64 / 10_000.0).floor() + 1.0) / 10.0
    }
}

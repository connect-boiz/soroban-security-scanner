//! Real-time, context-aware vulnerability severity scoring.
//!
//! Implements a CVSS-inspired scoring model extended with Soroban-specific
//! context factors: contract value at risk, deployment environment, and
//! exploitability in the current ledger state.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Base severity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BaseSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl BaseSeverity {
    /// CVSS base score range midpoints.
    pub fn base_score(self) -> f32 {
        match self {
            Self::None     => 0.0,
            Self::Low      => 2.5,
            Self::Medium   => 5.0,
            Self::High     => 7.5,
            Self::Critical => 9.5,
        }
    }

    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 9.0 => Self::Critical,
            s if s >= 7.0 => Self::High,
            s if s >= 4.0 => Self::Medium,
            s if s > 0.0  => Self::Low,
            _             => Self::None,
        }
    }
}

// ---------------------------------------------------------------------------
// Context factors
// ---------------------------------------------------------------------------

/// Deployment environment — affects exploitability multiplier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Production mainnet — highest risk.
    Production,
    /// Testnet or staging — moderate risk.
    Staging,
    /// Local development — lowest risk.
    Development,
}

impl Environment {
    fn multiplier(self) -> f32 {
        match self {
            Self::Production  => 1.0,
            Self::Staging     => 0.6,
            Self::Development => 0.3,
        }
    }
}

/// Input required to compute a contextualised severity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringInput {
    /// Static base severity from the scanner.
    pub base_severity:       BaseSeverity,
    /// Total USD value locked in the contract at scoring time.
    pub contract_value_usd:  f64,
    /// Deployment environment.
    pub environment:         Environment,
    /// Whether an exploit proof-of-concept exists publicly.
    pub exploit_available:   bool,
    /// Whether the vulnerability was found in access-controlled code.
    pub privileged_path:     bool,
}

// ---------------------------------------------------------------------------
// Scoring engine
// ---------------------------------------------------------------------------

/// Computed severity score with full breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityScore {
    /// Final 0-10 score.
    pub score:        f32,
    /// Derived severity label.
    pub severity:     BaseSeverity,
    /// Value-at-risk factor (0.0 - 2.0 boost).
    pub value_factor: f32,
    /// Environment multiplier applied.
    pub env_factor:   f32,
    /// Exploit availability bonus.
    pub exploit_bonus: f32,
}

/// Compute a real-time contextualised severity score.
pub fn compute_severity(input: &ScoringInput) -> SeverityScore {
    let base = input.base_severity.base_score();

    // Value-at-risk: logarithmic scale, max 2.0 boost at $10M+
    let value_factor: f32 = if input.contract_value_usd > 0.0 {
        (input.contract_value_usd as f32).log10().clamp(0.0, 7.0) / 3.5
    } else {
        0.0
    };

    let env_factor = input.environment.multiplier();

    // Public exploit available: +1.5 to base
    let exploit_bonus: f32 = if input.exploit_available { 1.5 } else { 0.0 };

    // Privileged path: reduces base by 20% (harder to reach)
    let priv_discount: f32 = if input.privileged_path { 0.8 } else { 1.0 };

    let raw = (base + value_factor + exploit_bonus) * env_factor * priv_discount;
    let score = raw.clamp(0.0, 10.0);

    SeverityScore {
        score: (score * 10.0).round() / 10.0,
        severity: BaseSeverity::from_score(score),
        value_factor,
        env_factor,
        exploit_bonus,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_with_exploit_and_production_is_max() {
        let input = ScoringInput {
            base_severity: BaseSeverity::Critical,
            contract_value_usd: 5_000_000.0,
            environment: Environment::Production,
            exploit_available: true,
            privileged_path: false,
        };
        let s = compute_severity(&input);
        assert!(s.score >= 9.5, "expected near-max score, got {}", s.score);
        assert_eq!(s.severity, BaseSeverity::Critical);
    }

    #[test]
    fn development_env_reduces_score() {
        let prod = ScoringInput {
            base_severity: BaseSeverity::High,
            contract_value_usd: 0.0,
            environment: Environment::Production,
            exploit_available: false,
            privileged_path: false,
        };
        let dev = ScoringInput { environment: Environment::Development, ..prod.clone() };
        assert!(compute_severity(&prod).score > compute_severity(&dev).score);
    }

    #[test]
    fn privileged_path_reduces_score() {
        let base = ScoringInput {
            base_severity: BaseSeverity::High,
            contract_value_usd: 100_000.0,
            environment: Environment::Production,
            exploit_available: false,
            privileged_path: false,
        };
        let priv_ = ScoringInput { privileged_path: true, ..base.clone() };
        assert!(compute_severity(&base).score > compute_severity(&priv_).score);
    }

    #[test]
    fn zero_value_no_boost() {
        let input = ScoringInput {
            base_severity: BaseSeverity::Medium,
            contract_value_usd: 0.0,
            environment: Environment::Production,
            exploit_available: false,
            privileged_path: false,
        };
        let s = compute_severity(&input);
        assert_eq!(s.value_factor, 0.0);
    }

    #[test]
    fn score_never_exceeds_10() {
        let input = ScoringInput {
            base_severity: BaseSeverity::Critical,
            contract_value_usd: 1_000_000_000.0,
            environment: Environment::Production,
            exploit_available: true,
            privileged_path: false,
        };
        assert!(compute_severity(&input).score <= 10.0);
    }
}

//! Dynamic, context-aware severity scoring for detected vulnerabilities.
//!
//! Implements issue #332. Replaces static severity labels with a CVSS
//! v3.1-based engine that adjusts for deployment context, supports real-time
//! recalculation, tracks trends, drives alerting, and can be augmented with a
//! machine-learned predictor fitted to historical expert assessments.
//!
//! # Acceptance criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | CVSS v3.1 scoring engine | [`cvss::CvssV31`] |
//! | Contextual risk factors (value, asset, environment, permissions) | [`context::RiskContext`] |
//! | Dynamic severity adjustment (exploitability & impact) | [`engine::SeverityEngine`] |
//! | Real-time recalculation on state change | [`engine::ScoredFinding::recalculate`] |
//! | Scoring API with detailed factor breakdown | [`engine::SeverityScore::factors`] |
//! | Severity trend analysis & historical tracking | [`history::SeverityHistory`] |
//! | Severity-based alerting & thresholds | [`alerting::AlertThresholds`] |
//! | ML model for severity prediction | [`ml::SeverityPredictor`] |
//! | Correlation with expert assessment | [`ml::pearson_correlation`] |
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::severity_scoring::*;
//!
//! let cvss = CvssV31::new(
//!     AttackVector::Network,
//!     AttackComplexity::Low,
//!     PrivilegesRequired::None,
//!     UserInteraction::None,
//!     Scope::Unchanged,
//!     Impact::High,
//!     Impact::High,
//!     Impact::High,
//! );
//! let context = RiskContext::new(
//!     ContractValueTier::Critical,
//!     AssetType::Stablecoin,
//!     DeploymentEnvironment::Mainnet,
//!     PermissionExposure::PublicPermissionless,
//! );
//!
//! let score = SeverityEngine::new().score(&cvss, &context);
//! assert_eq!(score.cvss_base_score, 9.8);
//! // High-value mainnet context pushes the contextual score to the ceiling.
//! assert_eq!(score.contextual_score, 10.0);
//! assert!(!score.factors.is_empty());
//! ```

pub mod alerting;
pub mod context;
pub mod cvss;
pub mod engine;
pub mod history;
pub mod ml;

#[cfg(test)]
mod tests;

pub use alerting::{AlertThresholds, NotificationUrgency, SeverityAlert};
pub use context::{
    AssetType, ContractValueTier, DeploymentEnvironment, ExploitMaturity, FactorContribution,
    MitigationLevel, PermissionExposure, RiskContext, MAX_CONTEXT_MULTIPLIER,
    MIN_CONTEXT_MULTIPLIER,
};
pub use cvss::{
    roundup, AttackComplexity, AttackVector, CvssRating, CvssV31, Impact, PrivilegesRequired, Scope,
    UserInteraction,
};
pub use engine::{
    rating_to_severity, RecalcOutcome, ScoredFinding, SeverityEngine, SeverityScore,
};
pub use history::{ScoreSample, SeverityHistory, TrendAnalysis, TrendDirection};
pub use ml::{features_from, pearson_correlation, SeverityPredictor, TrainConfig, FEATURE_COUNT};

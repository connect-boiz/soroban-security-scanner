//! Tests for the severity scoring system (#332).

use super::*;
use crate::Severity;

fn critical_cvss() -> CvssV31 {
    CvssV31::new(
        AttackVector::Network,
        AttackComplexity::Low,
        PrivilegesRequired::None,
        UserInteraction::None,
        Scope::Unchanged,
        Impact::High,
        Impact::High,
        Impact::High,
    )
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.05
}

// ── CVSS v3.1 engine ────────────────────────────────────────────────────────

#[test]
fn cvss_critical_network_vector_scores_9_8() {
    let cvss = critical_cvss();
    assert!(approx(cvss.base_score(), 9.8), "got {}", cvss.base_score());
    assert_eq!(cvss.rating(), CvssRating::Critical);
    assert_eq!(
        cvss.vector_string(),
        "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
    );
}

#[test]
fn cvss_scope_changed_all_high_scores_10() {
    let cvss = CvssV31::new(
        AttackVector::Network,
        AttackComplexity::Low,
        PrivilegesRequired::None,
        UserInteraction::None,
        Scope::Changed,
        Impact::High,
        Impact::High,
        Impact::High,
    );
    assert!(approx(cvss.base_score(), 10.0), "got {}", cvss.base_score());
    assert_eq!(cvss.rating(), CvssRating::Critical);
}

#[test]
fn cvss_low_vector_scores_1_8() {
    let cvss = CvssV31::new(
        AttackVector::Local,
        AttackComplexity::High,
        PrivilegesRequired::High,
        UserInteraction::Required,
        Scope::Unchanged,
        Impact::Low,
        Impact::None,
        Impact::None,
    );
    assert!(approx(cvss.base_score(), 1.8), "got {}", cvss.base_score());
    assert_eq!(cvss.rating(), CvssRating::Low);
}

#[test]
fn cvss_no_impact_scores_zero() {
    let cvss = CvssV31::new(
        AttackVector::Network,
        AttackComplexity::Low,
        PrivilegesRequired::None,
        UserInteraction::None,
        Scope::Unchanged,
        Impact::None,
        Impact::None,
        Impact::None,
    );
    assert_eq!(cvss.base_score(), 0.0);
    assert_eq!(cvss.rating(), CvssRating::None);
}

#[test]
fn roundup_matches_spec() {
    assert_eq!(roundup(4.0), 4.0);
    assert_eq!(roundup(4.02), 4.1);
    assert_eq!(roundup(6.0), 6.0);
    assert_eq!(roundup(0.0), 0.0);
}

#[test]
fn rating_boundaries() {
    assert_eq!(CvssRating::from_score(0.0), CvssRating::None);
    assert_eq!(CvssRating::from_score(3.9), CvssRating::Low);
    assert_eq!(CvssRating::from_score(4.0), CvssRating::Medium);
    assert_eq!(CvssRating::from_score(6.9), CvssRating::Medium);
    assert_eq!(CvssRating::from_score(7.0), CvssRating::High);
    assert_eq!(CvssRating::from_score(8.9), CvssRating::High);
    assert_eq!(CvssRating::from_score(9.0), CvssRating::Critical);
    assert_eq!(CvssRating::from_score(10.0), CvssRating::Critical);
}

// ── Contextual factors ──────────────────────────────────────────────────────

#[test]
fn context_value_tier_from_tvl() {
    assert_eq!(
        ContractValueTier::from_tvl_usd(0),
        ContractValueTier::Negligible
    );
    assert_eq!(
        ContractValueTier::from_tvl_usd(50_000),
        ContractValueTier::Low
    );
    assert_eq!(
        ContractValueTier::from_tvl_usd(500_000),
        ContractValueTier::Medium
    );
    assert_eq!(
        ContractValueTier::from_tvl_usd(5_000_000),
        ContractValueTier::High
    );
    assert_eq!(
        ContractValueTier::from_tvl_usd(50_000_000),
        ContractValueTier::Critical
    );
}

#[test]
fn context_multiplier_clamped_to_bounds() {
    // Maximal upward context exceeds the product cap and is clamped.
    let high = RiskContext::new(
        ContractValueTier::Critical,
        AssetType::Stablecoin,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicPermissionless,
    );
    assert_eq!(high.aggregate_multiplier(), MAX_CONTEXT_MULTIPLIER);

    // Maximal downward context is clamped to the floor.
    let low = RiskContext::new(
        ContractValueTier::Negligible,
        AssetType::TestToken,
        DeploymentEnvironment::Local,
        PermissionExposure::AdminOnly,
    );
    assert_eq!(low.aggregate_multiplier(), MIN_CONTEXT_MULTIPLIER);
}

#[test]
fn context_breakdown_has_all_factors() {
    let ctx = RiskContext::new(
        ContractValueTier::Medium,
        AssetType::Utility,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicAuthenticated,
    );
    let breakdown = ctx.breakdown();
    assert_eq!(breakdown.len(), 6);
    let names: Vec<&str> = breakdown.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"contract_value"));
    assert!(names.contains(&"asset_type"));
    assert!(names.contains(&"deployment_environment"));
    assert!(names.contains(&"permission_exposure"));
}

// ── Engine ──────────────────────────────────────────────────────────────────

#[test]
fn engine_high_context_keeps_critical() {
    let engine = SeverityEngine::new();
    let ctx = RiskContext::new(
        ContractValueTier::Critical,
        AssetType::Stablecoin,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicPermissionless,
    );
    let score = engine.score(&critical_cvss(), &ctx);
    assert_eq!(score.cvss_base_score, 9.8);
    assert_eq!(score.contextual_score, 10.0);
    assert_eq!(score.severity, Severity::Critical);
    // CVSS-base factor plus six context factors.
    assert_eq!(score.factors.len(), 7);
}

#[test]
fn engine_low_context_downgrades_severity() {
    let engine = SeverityEngine::new();
    let ctx = RiskContext::new(
        ContractValueTier::Negligible,
        AssetType::TestToken,
        DeploymentEnvironment::Local,
        PermissionExposure::AdminOnly,
    );
    let score = engine.score(&critical_cvss(), &ctx);
    // 9.8 * 0.40 (floor) -> ~4.0 Medium.
    assert!(score.contextual_score < score.cvss_base_score);
    assert!(score.context_shifted_rating());
    assert_eq!(score.severity, Severity::Medium);
}

// ── Real-time recalculation ─────────────────────────────────────────────────

#[test]
fn recalculation_escalates_on_promotion_to_mainnet() {
    // Same vuln on testnet vs mainnet with growing value.
    let testnet = RiskContext::new(
        ContractValueTier::Low,
        AssetType::Utility,
        DeploymentEnvironment::Testnet,
        PermissionExposure::Permissioned,
    );
    let mut finding = ScoredFinding::new("VULN-1", critical_cvss(), testnet);
    let before = finding.current.contextual_score;

    let mainnet = RiskContext::new(
        ContractValueTier::High,
        AssetType::Stablecoin,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicPermissionless,
    );
    let outcome = finding.recalculate(mainnet);

    assert!(outcome.changed);
    assert!(outcome.escalated);
    assert!(outcome.new_score > before);
}

#[test]
fn recalculation_no_change_is_reported() {
    let ctx = RiskContext::new(
        ContractValueTier::Medium,
        AssetType::Utility,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicAuthenticated,
    );
    let mut finding = ScoredFinding::new("VULN-2", critical_cvss(), ctx);
    let outcome = finding.recalculate(ctx);
    assert!(!outcome.changed);
    assert!(!outcome.escalated);
}

// ── History / trends ────────────────────────────────────────────────────────

#[test]
fn history_tracks_increasing_trend() {
    let engine = SeverityEngine::new();
    let mut history = SeverityHistory::new();

    let contexts = [
        RiskContext::new(
            ContractValueTier::Negligible,
            AssetType::TestToken,
            DeploymentEnvironment::Local,
            PermissionExposure::AdminOnly,
        ),
        RiskContext::new(
            ContractValueTier::Medium,
            AssetType::Utility,
            DeploymentEnvironment::Staging,
            PermissionExposure::Permissioned,
        ),
        RiskContext::new(
            ContractValueTier::Critical,
            AssetType::Stablecoin,
            DeploymentEnvironment::Mainnet,
            PermissionExposure::PublicPermissionless,
        ),
    ];
    for (i, ctx) in contexts.iter().enumerate() {
        let score = engine.score(&critical_cvss(), ctx);
        history.record("VULN-3", &score, 1000 + i as u64);
    }

    let trend = history.trend("VULN-3").unwrap();
    assert_eq!(trend.sample_count, 3);
    assert_eq!(trend.direction, TrendDirection::Increasing);
    assert!(trend.latest_score > trend.first_score);
    assert!(trend.delta > 0.0);
}

#[test]
fn history_missing_finding_returns_none() {
    let history = SeverityHistory::new();
    assert!(history.trend("nope").is_none());
}

// ── Alerting ────────────────────────────────────────────────────────────────

#[test]
fn alerting_critical_score_pages() {
    let thresholds = AlertThresholds::default();
    let engine = SeverityEngine::new();
    let ctx = RiskContext::new(
        ContractValueTier::Critical,
        AssetType::Stablecoin,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicPermissionless,
    );
    let score = engine.score(&critical_cvss(), &ctx);
    let alert = thresholds.evaluate("VULN-4", &score).unwrap();
    assert_eq!(alert.urgency, NotificationUrgency::Critical);
}

#[test]
fn alerting_low_score_is_silent() {
    let thresholds = AlertThresholds::default();
    let engine = SeverityEngine::new();
    let low_cvss = CvssV31::new(
        AttackVector::Local,
        AttackComplexity::High,
        PrivilegesRequired::High,
        UserInteraction::Required,
        Scope::Unchanged,
        Impact::Low,
        Impact::None,
        Impact::None,
    );
    let ctx = RiskContext::new(
        ContractValueTier::Negligible,
        AssetType::TestToken,
        DeploymentEnvironment::Local,
        PermissionExposure::AdminOnly,
    );
    let score = engine.score(&low_cvss, &ctx);
    assert!(thresholds.evaluate("VULN-5", &score).is_none());
}

#[test]
fn alerting_escalation_below_threshold_still_notifies() {
    let thresholds = AlertThresholds::default();
    let outcome = RecalcOutcome {
        id: "VULN-6".to_string(),
        previous_severity: Severity::Low,
        new_severity: Severity::Low,
        previous_score: 1.0,
        new_score: 3.0,
        changed: true,
        escalated: true,
    };
    let alert = thresholds.evaluate_recalc(&outcome).unwrap();
    assert_eq!(alert.urgency, NotificationUrgency::Info);
}

// ── ML predictor ────────────────────────────────────────────────────────────

fn training_dataset() -> Vec<(CvssV31, RiskContext)> {
    let cvss_variants = [
        critical_cvss(),
        CvssV31::new(
            AttackVector::Network,
            AttackComplexity::Low,
            PrivilegesRequired::Low,
            UserInteraction::None,
            Scope::Unchanged,
            Impact::High,
            Impact::Low,
            Impact::None,
        ),
        CvssV31::new(
            AttackVector::Adjacent,
            AttackComplexity::High,
            PrivilegesRequired::Low,
            UserInteraction::Required,
            Scope::Unchanged,
            Impact::Low,
            Impact::Low,
            Impact::Low,
        ),
        CvssV31::new(
            AttackVector::Local,
            AttackComplexity::Low,
            PrivilegesRequired::None,
            UserInteraction::None,
            Scope::Changed,
            Impact::High,
            Impact::High,
            Impact::Low,
        ),
    ];
    let value_tiers = [
        ContractValueTier::Negligible,
        ContractValueTier::Low,
        ContractValueTier::Medium,
        ContractValueTier::High,
        ContractValueTier::Critical,
    ];
    let envs = [
        DeploymentEnvironment::Local,
        DeploymentEnvironment::Testnet,
        DeploymentEnvironment::Staging,
        DeploymentEnvironment::Mainnet,
    ];

    let mut out = Vec::new();
    for cvss in &cvss_variants {
        for vt in &value_tiers {
            for env in &envs {
                let ctx = RiskContext::new(
                    *vt,
                    AssetType::Utility,
                    *env,
                    PermissionExposure::PublicAuthenticated,
                );
                out.push((*cvss, ctx));
            }
        }
    }
    out
}

#[test]
fn ml_predictor_correlates_with_engine_scores() {
    let engine = SeverityEngine::new();
    let dataset = training_dataset();

    // Build (features, target) pairs; target is the engine's contextual score,
    // standing in for an expert-assessed score.
    let samples: Vec<(Vec<f64>, f64)> = dataset
        .iter()
        .map(|(cvss, ctx)| {
            (
                features_from(cvss, ctx),
                engine.score(cvss, ctx).contextual_score,
            )
        })
        .collect();

    // Deterministic train/holdout split (every 7th sample is holdout).
    let mut train = Vec::new();
    let mut holdout = Vec::new();
    for (i, s) in samples.iter().enumerate() {
        if i % 7 == 0 {
            holdout.push(s.clone());
        } else {
            train.push(s.clone());
        }
    }

    let model = SeverityPredictor::train(&train, TrainConfig::default()).unwrap();

    let preds: Vec<f64> = holdout.iter().map(|(f, _)| model.predict(f)).collect();
    let actual: Vec<f64> = holdout.iter().map(|(_, y)| *y).collect();

    let r = pearson_correlation(&preds, &actual);
    assert!(
        r >= 0.95,
        "expected >=0.95 correlation with expert scores, got {}",
        r
    );
}

#[test]
fn ml_feature_vector_has_expected_length() {
    let f = features_from(
        &critical_cvss(),
        &RiskContext::new(
            ContractValueTier::Medium,
            AssetType::Utility,
            DeploymentEnvironment::Mainnet,
            PermissionExposure::PublicAuthenticated,
        ),
    );
    assert_eq!(f.len(), FEATURE_COUNT);
}

#[test]
fn pearson_correlation_basic() {
    let a = [1.0, 2.0, 3.0, 4.0];
    let b = [2.0, 4.0, 6.0, 8.0];
    assert!((pearson_correlation(&a, &b) - 1.0).abs() < 1e-9);
    // Degenerate inputs return 0.
    assert_eq!(pearson_correlation(&[1.0], &[1.0]), 0.0);
    assert_eq!(pearson_correlation(&[1.0, 1.0], &[1.0, 1.0]), 0.0);
}

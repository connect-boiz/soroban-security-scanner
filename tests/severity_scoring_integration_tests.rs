//! End-to-end integration tests for dynamic severity scoring (#332).
//!
//! Exercises the public API across the acceptance criteria: CVSS v3.1 scoring,
//! contextual adjustment, real-time recalculation, factor breakdown, trend
//! tracking, alerting thresholds, and the ML predictor's correlation with
//! expert-equivalent scores.

use soroban_security_scanner::severity_scoring::{
    features_from, pearson_correlation, AlertThresholds, AssetType, AttackComplexity, AttackVector,
    ContractValueTier, CvssV31, DeploymentEnvironment, Impact, NotificationUrgency,
    PermissionExposure, PrivilegesRequired, RiskContext, Scope, ScoredFinding, SeverityEngine,
    SeverityHistory, SeverityPredictor, TrainConfig, TrendDirection, UserInteraction,
};
use soroban_security_scanner::Severity;

fn worst_case_cvss() -> CvssV31 {
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

#[test]
fn cvss_and_context_produce_breakdown() {
    let engine = SeverityEngine::new();
    let ctx = RiskContext::new(
        ContractValueTier::from_tvl_usd(25_000_000),
        AssetType::Stablecoin,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicPermissionless,
    );
    let score = engine.score(&worst_case_cvss(), &ctx);

    assert_eq!(score.cvss_base_score, 9.8);
    assert_eq!(score.severity, Severity::Critical);
    assert!(score.cvss_vector.starts_with("CVSS:3.1/"));
    // The breakdown exposes the CVSS base plus every contextual factor.
    assert_eq!(score.factors.len(), 7);
    assert!(score.factors.iter().any(|f| f.name == "contract_value"));
}

#[test]
fn context_changes_severity_for_same_vulnerability() {
    let engine = SeverityEngine::new();
    let cvss = worst_case_cvss();

    let dev = engine.score(
        &cvss,
        &RiskContext::new(
            ContractValueTier::Negligible,
            AssetType::TestToken,
            DeploymentEnvironment::Local,
            PermissionExposure::AdminOnly,
        ),
    );
    let prod = engine.score(
        &cvss,
        &RiskContext::new(
            ContractValueTier::Critical,
            AssetType::Stablecoin,
            DeploymentEnvironment::Mainnet,
            PermissionExposure::PublicPermissionless,
        ),
    );

    assert!(prod.contextual_score > dev.contextual_score);
    assert_eq!(prod.severity, Severity::Critical);
    assert!(dev.context_shifted_rating());
}

#[test]
fn realtime_recalculation_and_alerting() {
    let thresholds = AlertThresholds::default();
    let mut finding = ScoredFinding::new(
        "VULN-INT-1",
        worst_case_cvss(),
        RiskContext::new(
            ContractValueTier::Low,
            AssetType::Utility,
            DeploymentEnvironment::Testnet,
            PermissionExposure::Permissioned,
        ),
    );

    let outcome = finding.recalculate(RiskContext::new(
        ContractValueTier::Critical,
        AssetType::Stablecoin,
        DeploymentEnvironment::Mainnet,
        PermissionExposure::PublicPermissionless,
    ));

    assert!(outcome.escalated);
    let alert = thresholds.evaluate_recalc(&outcome).unwrap();
    assert_eq!(alert.urgency, NotificationUrgency::Critical);
}

#[test]
fn trend_tracking_over_time() {
    let engine = SeverityEngine::new();
    let mut history = SeverityHistory::new();
    let cvss = worst_case_cvss();

    let stages = [
        (
            ContractValueTier::Negligible,
            DeploymentEnvironment::Local,
            1_000u64,
        ),
        (
            ContractValueTier::Medium,
            DeploymentEnvironment::Staging,
            2_000,
        ),
        (
            ContractValueTier::Critical,
            DeploymentEnvironment::Mainnet,
            3_000,
        ),
    ];
    for (tier, env, ts) in stages {
        let score = engine.score(
            &cvss,
            &RiskContext::new(
                tier,
                AssetType::Stablecoin,
                env,
                PermissionExposure::PublicPermissionless,
            ),
        );
        history.record("VULN-INT-1", &score, ts);
    }

    let trend = history.trend("VULN-INT-1").unwrap();
    assert_eq!(trend.direction, TrendDirection::Increasing);
    assert_eq!(trend.sample_count, 3);
}

#[test]
fn ml_predictor_achieves_high_correlation() {
    let engine = SeverityEngine::new();

    let cvss_set = [
        worst_case_cvss(),
        CvssV31::new(
            AttackVector::Adjacent,
            AttackComplexity::High,
            PrivilegesRequired::Low,
            UserInteraction::Required,
            Scope::Unchanged,
            Impact::Low,
            Impact::Low,
            Impact::None,
        ),
        CvssV31::new(
            AttackVector::Network,
            AttackComplexity::Low,
            PrivilegesRequired::Low,
            UserInteraction::None,
            Scope::Changed,
            Impact::High,
            Impact::Low,
            Impact::Low,
        ),
    ];
    let tiers = [
        ContractValueTier::Negligible,
        ContractValueTier::Low,
        ContractValueTier::Medium,
        ContractValueTier::High,
        ContractValueTier::Critical,
    ];
    let envs = [
        DeploymentEnvironment::Local,
        DeploymentEnvironment::Testnet,
        DeploymentEnvironment::Mainnet,
    ];

    let mut samples = Vec::new();
    for cvss in &cvss_set {
        for t in &tiers {
            for e in &envs {
                let ctx = RiskContext::new(
                    *t,
                    AssetType::Utility,
                    *e,
                    PermissionExposure::PublicAuthenticated,
                );
                let target = engine.score(cvss, &ctx).contextual_score;
                samples.push((features_from(cvss, &ctx), target));
            }
        }
    }

    let mut train = Vec::new();
    let mut test = Vec::new();
    for (i, s) in samples.iter().enumerate() {
        if i % 5 == 0 {
            test.push(s.clone());
        } else {
            train.push(s.clone());
        }
    }

    let model = SeverityPredictor::train(&train, TrainConfig::default()).unwrap();
    let preds: Vec<f64> = test.iter().map(|(f, _)| model.predict(f)).collect();
    let actual: Vec<f64> = test.iter().map(|(_, y)| *y).collect();

    let r = pearson_correlation(&preds, &actual);
    assert!(r >= 0.95, "correlation {} below 0.95 target", r);
}

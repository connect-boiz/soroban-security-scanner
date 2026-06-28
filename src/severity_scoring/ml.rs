//! Machine-learning severity predictor.
//!
//! A lightweight, dependency-free multiple-linear-regression model trained by
//! gradient descent on standardized features. It learns to predict a severity
//! score from features derived from a finding's CVSS metrics and context, so it
//! can be fitted against historical expert assessments and then used to predict
//! scores for new findings.
//!
//! The model is intentionally simple and deterministic (no randomness), making
//! it reproducible and easy to reason about. Feature standardization is folded
//! into the model so callers only ever pass raw feature vectors.

use serde::{Deserialize, Serialize};

use super::context::RiskContext;
use super::cvss::CvssV31;

/// Number of features produced by [`features_from`].
pub const FEATURE_COUNT: usize = 7;

/// Derive a fixed-length feature vector from a finding's CVSS metrics and
/// context. The interaction term (`base * multiplier`) lets a linear model
/// capture the multiplicative nature of contextual scoring.
pub fn features_from(cvss: &CvssV31, context: &RiskContext) -> Vec<f64> {
    let base = cvss.base_score();
    let mult = context.aggregate_multiplier();
    let breakdown = context.breakdown();
    // breakdown order: contract_value, asset_type, deployment_environment,
    // permission_exposure, exploit_maturity, mitigation.
    let value_mult = breakdown[0].multiplier;
    let asset_mult = breakdown[1].multiplier;
    let env_mult = breakdown[2].multiplier;
    let perm_mult = breakdown[3].multiplier;

    vec![
        base,
        mult,
        base * mult,
        value_mult,
        asset_mult,
        env_mult,
        perm_mult,
    ]
}

/// A trained severity-prediction model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeverityPredictor {
    weights: Vec<f64>,
    bias: f64,
    feature_means: Vec<f64>,
    feature_stds: Vec<f64>,
}

/// Hyperparameters for training.
#[derive(Debug, Clone, Copy)]
pub struct TrainConfig {
    pub epochs: usize,
    pub learning_rate: f64,
    pub l2: f64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            epochs: 3000,
            learning_rate: 0.05,
            l2: 1e-4,
        }
    }
}

impl SeverityPredictor {
    /// Train a predictor on `(features, target_score)` samples.
    ///
    /// Returns `None` if there are no samples or the feature dimensionality is
    /// inconsistent.
    pub fn train(samples: &[(Vec<f64>, f64)], config: TrainConfig) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let n_features = samples[0].0.len();
        if n_features == 0 || samples.iter().any(|(f, _)| f.len() != n_features) {
            return None;
        }

        // Standardize features for stable, learning-rate-insensitive descent.
        let n = samples.len() as f64;
        let mut means = vec![0.0; n_features];
        for (f, _) in samples {
            for (j, &x) in f.iter().enumerate() {
                means[j] += x;
            }
        }
        for m in means.iter_mut() {
            *m /= n;
        }
        let mut stds = vec![0.0; n_features];
        for (f, _) in samples {
            for (j, &x) in f.iter().enumerate() {
                let d = x - means[j];
                stds[j] += d * d;
            }
        }
        for s in stds.iter_mut() {
            *s = (*s / n).sqrt();
            if *s < 1e-12 {
                *s = 1.0; // avoid divide-by-zero for constant features
            }
        }

        let standardized: Vec<(Vec<f64>, f64)> = samples
            .iter()
            .map(|(f, y)| {
                let z: Vec<f64> = f
                    .iter()
                    .enumerate()
                    .map(|(j, &x)| (x - means[j]) / stds[j])
                    .collect();
                (z, *y)
            })
            .collect();

        // Gradient descent on MSE with L2 regularization.
        let mut weights = vec![0.0; n_features];
        let mut bias = 0.0;
        for _ in 0..config.epochs {
            let mut grad_w = vec![0.0; n_features];
            let mut grad_b = 0.0;
            for (z, y) in &standardized {
                let pred = dot(&weights, z) + bias;
                let err = pred - y;
                for (j, &zj) in z.iter().enumerate() {
                    grad_w[j] += err * zj;
                }
                grad_b += err;
            }
            for (j, gw) in grad_w.iter().enumerate() {
                // Mean gradient + L2 penalty (bias is not regularized).
                weights[j] -= config.learning_rate * (gw / n + config.l2 * weights[j]);
            }
            bias -= config.learning_rate * (grad_b / n);
        }

        Some(Self {
            weights,
            bias,
            feature_means: means,
            feature_stds: stds,
        })
    }

    /// Predict a severity score for a raw feature vector, clamped to 0.0–10.0.
    pub fn predict(&self, features: &[f64]) -> f64 {
        if features.len() != self.weights.len() {
            return 0.0;
        }
        let z: Vec<f64> = features
            .iter()
            .enumerate()
            .map(|(j, &x)| (x - self.feature_means[j]) / self.feature_stds[j])
            .collect();
        (dot(&self.weights, &z) + self.bias).clamp(0.0, 10.0)
    }

    /// Convenience: predict directly from CVSS metrics + context.
    pub fn predict_finding(&self, cvss: &CvssV31, context: &RiskContext) -> f64 {
        self.predict(&features_from(cvss, context))
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Pearson correlation coefficient between two equal-length series. Returns
/// `0.0` for degenerate inputs (differing lengths, fewer than two points, or
/// zero variance). Used to validate predictions against expert assessments.
pub fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.len() < 2 {
        return 0.0;
    }
    let n = a.len() as f64;
    let mean_a = a.iter().sum::<f64>() / n;
    let mean_b = b.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let da = x - mean_a;
        let db = y - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }
    if var_a <= 0.0 || var_b <= 0.0 {
        return 0.0;
    }
    cov / (var_a.sqrt() * var_b.sqrt())
}

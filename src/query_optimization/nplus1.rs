//! N+1 query-pattern detection.
//!
//! Given the queries executed while handling a single request, detects the
//! classic N+1 anti-pattern: one query followed by the *same* parameterized
//! query run many times (one per row). Reports the offending fingerprint and a
//! suggested fix (batch/JOIN).

use crate::query_optimization::normalize::normalize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default repetition count at/above which a repeated query is an N+1 pattern.
pub const DEFAULT_NPLUS1_THRESHOLD: usize = 5;

/// A detected N+1 pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NPlusOneFinding {
    /// The repeated query fingerprint.
    pub fingerprint: String,
    /// How many times it ran in the request.
    pub repetitions: usize,
    /// Suggested remediation.
    pub suggestion: String,
}

/// Analyzes the queries from one request for N+1 patterns.
///
/// `queries` is the ordered list of SQL executed while serving a request.
/// Any fingerprint executed `>= threshold` times is reported.
pub fn detect(queries: &[&str], threshold: usize) -> Vec<NPlusOneFinding> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for q in queries {
        let fp = normalize(q);
        let entry = counts.entry(fp.clone()).or_insert(0);
        if *entry == 0 {
            order.push(fp);
        }
        *entry += 1;
    }

    let mut findings = Vec::new();
    for fp in order {
        let reps = counts[&fp];
        if reps >= threshold {
            findings.push(NPlusOneFinding {
                suggestion: format!(
                    "query ran {reps} times in one request; replace the per-row lookup with a \
                     single JOIN or a batched `IN (...)` query"
                ),
                fingerprint: fp,
                repetitions: reps,
            });
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_n_plus_one_detected() {
        // One list query, then a per-row lookup 10 times.
        let mut queries: Vec<&str> = vec!["SELECT id FROM orders"];
        let per_row = [
            "SELECT * FROM users WHERE id = 1",
            "SELECT * FROM users WHERE id = 2",
            "SELECT * FROM users WHERE id = 3",
            "SELECT * FROM users WHERE id = 4",
            "SELECT * FROM users WHERE id = 5",
            "SELECT * FROM users WHERE id = 6",
        ];
        queries.extend_from_slice(&per_row);

        let findings = detect(&queries, DEFAULT_NPLUS1_THRESHOLD);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].fingerprint.contains("users"));
        assert_eq!(findings[0].repetitions, 6);
        assert!(findings[0].suggestion.contains("JOIN"));
    }

    #[test]
    fn below_threshold_is_clean() {
        let queries = [
            "SELECT * FROM t WHERE id = 1",
            "SELECT * FROM t WHERE id = 2",
        ];
        assert!(detect(&queries, DEFAULT_NPLUS1_THRESHOLD).is_empty());
    }

    #[test]
    fn distinct_queries_not_flagged() {
        let queries = [
            "SELECT * FROM a",
            "SELECT * FROM b",
            "SELECT * FROM c",
            "SELECT * FROM d",
            "SELECT * FROM e",
            "SELECT * FROM f",
        ];
        assert!(detect(&queries, DEFAULT_NPLUS1_THRESHOLD).is_empty());
    }

    #[test]
    fn findings_preserve_first_seen_order() {
        let mut q: Vec<&str> = Vec::new();
        for _ in 0..5 {
            q.push("SELECT * FROM x WHERE id = 1");
        }
        for _ in 0..5 {
            q.push("SELECT * FROM y WHERE id = 1");
        }
        let findings = detect(&q, 5);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].fingerprint.contains("x"));
        assert!(findings[1].fingerprint.contains("y"));
    }
}

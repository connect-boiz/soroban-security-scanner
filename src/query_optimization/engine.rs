//! The query optimizer: the façade an application instruments queries through.
//!
//! Every executed query flows through `record`, which updates latency metrics
//! and the slow-query monitor; the optimizer also drives index/N+1/plan
//! analysis and produces a performance-dashboard snapshot, including whether
//! the workload meets the <100 ms p95 target.

use crate::query_optimization::index::{IndexAdvisor, IndexRecommendation, QueryShape};
use crate::query_optimization::metrics::{LatencyStats, QueryMetrics};
use crate::query_optimization::nplus1::{detect as detect_nplus1, NPlusOneFinding};
use crate::query_optimization::slow_query::{SlowQuery, SlowQueryMonitor};
use serde::{Deserialize, Serialize};

/// A performance-dashboard snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Distinct query fingerprints observed.
    pub distinct_queries: usize,
    /// Overall latency stats, if any executions were recorded.
    pub overall: Option<LatencyStats>,
    /// Total slow queries (>threshold) seen.
    pub slow_queries: u64,
    /// Whether the workload meets the p95 latency target.
    pub meets_p95_target: bool,
}

/// The query optimizer / performance instrument.
pub struct QueryOptimizer {
    metrics: QueryMetrics,
    slow: SlowQueryMonitor,
    advisor: IndexAdvisor,
}

impl QueryOptimizer {
    /// Creates an optimizer with the given index advisor and slow-query monitor.
    pub fn new(advisor: IndexAdvisor, slow: SlowQueryMonitor) -> Self {
        Self {
            metrics: QueryMetrics::new(),
            slow,
            advisor,
        }
    }

    /// Records a query execution, returning a slow-query alert if it crossed
    /// the slow threshold.
    pub fn record(&mut self, sql: &str, duration_ms: u64, at: i64) -> Option<SlowQuery> {
        self.metrics.record(sql, duration_ms);
        self.slow.observe(sql, duration_ms, at)
    }

    /// Index recommendations for a query shape.
    pub fn recommend_indexes(&self, shape: &QueryShape) -> Vec<IndexRecommendation> {
        self.advisor.recommend(shape)
    }

    /// Detects N+1 patterns within one request's query list.
    pub fn detect_nplus1(
        &self,
        request_queries: &[&str],
        threshold: usize,
    ) -> Vec<NPlusOneFinding> {
        detect_nplus1(request_queries, threshold)
    }

    /// Latency stats for a specific query.
    pub fn stats_for(&self, sql: &str) -> Option<LatencyStats> {
        self.metrics.stats_for(sql)
    }

    /// A dashboard snapshot of current performance.
    pub fn report(&self) -> PerformanceReport {
        let overall = self.metrics.overall();
        let meets = overall
            .as_ref()
            .map(|s| s.meets_p95_target())
            .unwrap_or(true);
        PerformanceReport {
            distinct_queries: self.metrics.distinct_queries(),
            overall,
            slow_queries: self.slow.total_slow(),
            meets_p95_target: meets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_optimization::index::IndexDef;

    fn optimizer() -> QueryOptimizer {
        let mut advisor = IndexAdvisor::new();
        advisor.add_index(IndexDef::new("users", vec!["id".to_string()]));
        QueryOptimizer::new(advisor, SlowQueryMonitor::default())
    }

    #[test]
    fn records_and_reports_fast_workload() {
        let mut o = optimizer();
        for i in 0..100 {
            o.record(
                "SELECT * FROM users WHERE id = 1",
                20 + (i % 5),
                1000 + i as i64,
            );
        }
        let report = o.report();
        assert_eq!(report.distinct_queries, 1);
        assert_eq!(report.slow_queries, 0);
        assert!(report.meets_p95_target);
        assert!(report.overall.unwrap().p95_ms < 100);
    }

    #[test]
    fn slow_query_alerts_and_counts() {
        let mut o = optimizer();
        let alert = o.record("SELECT * FROM big WHERE x = 1", 1500, 1000);
        assert!(alert.is_some());
        assert_eq!(o.report().slow_queries, 1);
    }

    #[test]
    fn workload_misses_p95_when_slow() {
        let mut o = optimizer();
        // 90 fast + 10 slow: the nearest-rank p95 (index 94 of 100) lands in
        // the slow tail, so the workload misses the <100ms target.
        for i in 0..90 {
            o.record("SELECT 1", 10, 1000 + i);
        }
        for i in 0..10 {
            o.record("SELECT 1", 800, 2000 + i);
        }
        assert!(!o.report().meets_p95_target);
    }

    #[test]
    fn surfaces_index_and_nplus1_advice() {
        let o = optimizer();
        let shape = QueryShape {
            table: "users".to_string(),
            filter_columns: vec!["email".to_string()],
            join_columns: vec![],
        };
        assert_eq!(o.recommend_indexes(&shape).len(), 1);

        let mut reqs: Vec<&str> = Vec::new();
        for _ in 0..6 {
            reqs.push("SELECT * FROM addr WHERE user_id = 1");
        }
        assert_eq!(o.detect_nplus1(&reqs, 5).len(), 1);
    }
}

//! Database query optimization and performance management (issue #344).
//!
//! A self-contained toolkit for keeping database access fast: SQL fingerprinting,
//! percentile latency metrics, slow-query monitoring, N+1 detection, an index
//! advisor, execution-plan analysis, a TTL query-result cache, and benchmark
//! regression detection — surfaced through a `QueryOptimizer` performance
//! dashboard that tracks the <100 ms p95 target.
//!
//! # Acceptance-criteria mapping
//!
//! | Requirement | Where |
//! |-------------|-------|
//! | Indexing strategy / recommendations | [`index::IndexAdvisor`] |
//! | Query result caching | [`cache::QueryResultCache`] |
//! | Eliminate N+1 patterns | [`nplus1::detect`] |
//! | Slow-query monitoring + alerting (>1s) | [`slow_query::SlowQueryMonitor`] |
//! | Execution-plan analysis | [`plan::analyze`] |
//! | Benchmarking + regression testing | [`benchmark::detect_regressions`] |
//! | Performance dashboard / real-time metrics | [`engine::QueryOptimizer::report`], [`metrics`] |
//! | <100 ms p95 query latency | [`metrics::P95_TARGET_MS`], [`metrics::LatencyStats::meets_p95_target`] |
//! | Comprehensive performance testing | per-module tests + [`tests`] |
//!
//! Connection-pool tuning and read-replica routing live in the companion
//! `db_pool` module; this module focuses on query-level optimization.
//!
//! # Example
//!
//! ```
//! use soroban_security_scanner::query_optimization::*;
//!
//! let mut opt = QueryOptimizer::new(IndexAdvisor::new(), SlowQueryMonitor::default());
//! opt.record("SELECT * FROM users WHERE id = 1", 20, 1_700_000_000);
//! let slow = opt.record("SELECT * FROM big_table", 1500, 1_700_000_001);
//! assert!(slow.is_some()); // >1s slow-query alert
//! ```

pub mod benchmark;
pub mod cache;
pub mod engine;
pub mod index;
pub mod metrics;
pub mod normalize;
pub mod nplus1;
pub mod plan;
pub mod slow_query;

#[cfg(test)]
mod tests;

pub use benchmark::{detect_regressions, passes, Baseline, Regression};
pub use cache::{CacheStats, QueryResultCache};
pub use engine::{PerformanceReport, QueryOptimizer};
pub use index::{IndexAdvisor, IndexDef, IndexRecommendation, QueryShape};
pub use metrics::{latency_stats, LatencyStats, QueryMetrics, P95_TARGET_MS};
pub use normalize::normalize;
pub use nplus1::{detect as detect_nplus1, NPlusOneFinding, DEFAULT_NPLUS1_THRESHOLD};
pub use plan::{
    analyze as analyze_plan, is_efficient, total_cost, PlanFinding, PlanNode, ScanType,
};
pub use slow_query::{SlowQuery, SlowQueryMonitor, DEFAULT_SLOW_THRESHOLD_MS};

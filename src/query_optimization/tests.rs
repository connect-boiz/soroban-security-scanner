//! End-to-end integration tests: instrument a workload, catch slow queries and
//! N+1 patterns, get index/plan advice, serve from cache, and gate on
//! benchmark regressions.

use super::*;

#[test]
fn instrumented_workload_meets_p95_and_flags_slow() {
    let mut advisor = IndexAdvisor::new();
    advisor.add_index(IndexDef::new("users", vec!["id".to_string()]));
    let mut opt = QueryOptimizer::new(advisor, SlowQueryMonitor::default());

    // 100 fast indexed lookups...
    for i in 0..100 {
        opt.record(
            "SELECT * FROM users WHERE id = 1",
            15 + (i % 10),
            1000 + i as i64,
        );
    }
    // ...and one pathological full scan.
    let slow = opt.record("SELECT * FROM audit_log", 2300, 5000);
    assert!(slow.is_some());

    let report = opt.report();
    assert!(report.meets_p95_target); // the slow one is a single outlier
    assert_eq!(report.slow_queries, 1);
    assert_eq!(report.distinct_queries, 2);
}

#[test]
fn nplus1_pattern_is_detected_with_fix_suggestion() {
    let opt = QueryOptimizer::new(IndexAdvisor::new(), SlowQueryMonitor::default());
    // A request that lists orders then loads each order's user separately.
    let mut request: Vec<&str> = vec!["SELECT id, user_id FROM orders WHERE status = 'open'"];
    for _ in 0..8 {
        request.push("SELECT * FROM users WHERE id = 1");
    }
    let findings = opt.detect_nplus1(&request, DEFAULT_NPLUS1_THRESHOLD);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].suggestion.contains("JOIN"));
    assert_eq!(findings[0].repetitions, 8);
}

#[test]
fn index_advisor_recommends_for_unindexed_filter() {
    let mut advisor = IndexAdvisor::new();
    advisor.add_index(IndexDef::new("contracts", vec!["id".to_string()]));
    let opt = QueryOptimizer::new(advisor, SlowQueryMonitor::default());

    let shape = QueryShape {
        table: "contracts".to_string(),
        filter_columns: vec!["address".to_string()],
        join_columns: vec!["owner_id".to_string()],
    };
    let recs = opt.recommend_indexes(&shape);
    assert_eq!(recs.len(), 2); // address (filter) + owner_id (join)
}

#[test]
fn execution_plan_flags_seq_scan() {
    let plan = vec![
        PlanNode {
            table: "scans".to_string(),
            scan: ScanType::SeqScan,
            estimated_rows: 2_000_000,
            estimated_cost: 90_000,
        },
        PlanNode {
            table: "users".to_string(),
            scan: ScanType::IndexScan,
            estimated_rows: 1,
            estimated_cost: 4,
        },
    ];
    assert!(!is_efficient(&plan));
    let findings = analyze_plan(&plan);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].table, "scans");
}

#[test]
fn result_cache_serves_repeated_reads() {
    let mut cache = QueryResultCache::new();
    let sql = "SELECT name FROM users WHERE id = ?";
    let params = vec!["42".to_string()];
    // First read misses; subsequent reads hit until TTL.
    assert!(cache.get(sql, &params, 1000).is_none());
    cache.put(sql, &params, vec![vec!["alice".to_string()]], 1000, 300);
    for _ in 0..10 {
        assert!(cache.get(sql, &params, 1100).is_some());
    }
    assert!(cache.stats().hit_rate() > 0.9);
}

#[test]
fn benchmark_regression_gate() {
    let mut baseline = Baseline::new();
    baseline.set("user_lookup", 20);
    baseline.set("contract_scan_summary", 80);

    // A run within tolerance passes the gate.
    let good = vec![
        ("user_lookup".to_string(), 21),
        ("contract_scan_summary".to_string(), 85),
    ];
    assert!(passes(&baseline, &good, 0.20));

    // A run that doubled a query's latency is blocked.
    let bad = vec![("user_lookup".to_string(), 45)];
    let regs = detect_regressions(&baseline, &bad, 0.20);
    assert_eq!(regs.len(), 1);
    assert!(!passes(&baseline, &bad, 0.20));
}

#[test]
fn circuit_breaker_trips_on_excessive_ratio_and_blocks_execution() {
    let mut breaker = QueryCircuitBreaker::new();

    // 20 distinct queries: ratio is 1.0 (20/20)
    for i in 0..20 {
        let col = (b'a' + i as u8) as char;
        let sql = format!("SELECT {} FROM table", col);
        assert!(breaker.record_query(&sql).is_ok());
    }
    assert_eq!(breaker.total_queries(), 20);
    assert_eq!(breaker.unique_queries(), 20);
    assert!((breaker.current_ratio() - 1.0).abs() < 1e-9);

    breaker.reset();

    // 20 repeated identical queries: ratio is 20.0 (20/1)
    for i in 0..20 {
        let sql = format!("SELECT * FROM users WHERE id = {}", i);
        assert!(breaker.record_query(&sql).is_ok());
    }
    assert_eq!(breaker.total_queries(), 20);
    assert_eq!(breaker.unique_queries(), 1);
    assert!(!breaker.is_tripped());

    // 21st query: ratio becomes 21.0 > 20.0 -> trips circuit breaker!
    let err = breaker
        .record_query("SELECT * FROM users WHERE id = 999")
        .expect_err("must trip circuit breaker");
    assert_eq!(err.status_code(), axum::http::StatusCode::TOO_MANY_REQUESTS);
    assert!(breaker.is_tripped());
    assert_eq!(breaker.metrics().blocked_total(), 1);

    // Verify Prometheus metric rendering
    let prom = breaker.metrics().render_prometheus();
    assert!(prom.contains("n_plus_one_queries_blocked_total 1"));
    assert!(prom.contains("n_plus_one_ratio_current 21.000"));
}

#[test]
fn optimizer_record_with_breaker_stops_pathological_load() {
    let mut opt = QueryOptimizer::new(IndexAdvisor::new(), SlowQueryMonitor::default());

    // 20 repeated queries pass
    for i in 0..20 {
        let sql = format!("SELECT * FROM orders WHERE id = {}", i);
        let res = opt.record_with_breaker(&sql, 5, 1000 + i as i64);
        assert!(res.is_ok());
    }

    // 21st repeated query trips
    let res = opt.record_with_breaker("SELECT * FROM orders WHERE id = 21", 5, 1021);
    assert!(res.is_err());
    assert!(opt.circuit_breaker().is_tripped());

    let report = opt.report();
    assert!(report.circuit_breaker_tripped);
    assert!(report.n_plus_one_ratio > 20.0);
}

#[test]
fn circuit_breaker_warn_policy_does_not_block() {
    let mut breaker = QueryCircuitBreaker::with_config(CircuitBreakerConfig {
        max_n_plus_one_ratio: 20.0,
        min_queries_threshold: 20,
        policy: NPlusOnePolicy::Warn,
    });

    for i in 0..25 {
        let sql = format!("SELECT * FROM items WHERE id = {}", i);
        // Under Warn policy, query records without returning Err
        assert!(breaker.record_query(&sql).is_ok());
    }
    assert!(breaker.is_tripped());
    // Blocked counter remains 0 because policy was Warn
    assert_eq!(breaker.metrics().blocked_total(), 0);
}

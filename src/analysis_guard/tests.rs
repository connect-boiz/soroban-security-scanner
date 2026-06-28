//! End-to-end integration tests: hostile inputs are contained across the full
//! guard pipeline (validation → sandbox → result check), the queue manages
//! priority/resources, fuzzing finds no crashes in a robust parser, and
//! monitoring reflects the outcomes.

use super::*;

fn guard() -> AnalysisGuard {
    AnalysisGuard::new(GuardLimits::default())
}

fn clean_analyzer(source: &str) -> Result<(AnalysisResult, ResourceUsage), String> {
    let lines = source.lines().count().max(1);
    Ok((
        AnalysisResult {
            findings: vec![Finding {
                rule: "unchecked-call".to_string(),
                severity: "MEDIUM".to_string(),
                line: 1,
            }],
            source_lines: lines,
        },
        ResourceUsage {
            cpu_units: 500,
            memory_bytes: 4096,
            wall_ms: 120,
        },
    ))
}

#[test]
fn legitimate_contract_analyzes_cleanly() {
    let g = guard();
    let res = g
        .analyze(b"pub fn transfer() {\n  // ...\n}\n", clean_analyzer)
        .unwrap();
    assert_eq!(res.findings.len(), 1);
    assert_eq!(g.monitor().stats().succeeded, 1);
}

#[test]
fn hostile_inputs_are_all_contained() {
    let g = guard();

    // 1. Oversized input.
    let small = AnalysisGuard::new(GuardLimits {
        input: InputLimits {
            max_bytes: 8,
            ..InputLimits::default()
        },
        ..GuardLimits::default()
    });
    assert!(matches!(
        small.analyze(b"way too long input", clean_analyzer),
        Err(GuardError::Input(InputError::TooLarge { .. }))
    ));

    // 2. NUL/control bytes.
    assert!(matches!(
        g.analyze(b"fn x()\0{}", clean_analyzer),
        Err(GuardError::Input(InputError::ControlBytes))
    ));

    // 3. Unbalanced delimiters.
    assert!(matches!(
        g.analyze(b"fn x( {", clean_analyzer),
        Err(GuardError::Input(InputError::UnbalancedDelimiters))
    ));

    // 4. A parser that panics (crash) — contained.
    assert_eq!(
        g.analyze(b"fn ok() {}", |_| panic!("crash")).unwrap_err(),
        GuardError::Sandbox(SandboxError::Crashed)
    );

    // 5. A parser that hangs past the timeout (reported via wall_ms).
    assert!(matches!(
        g.analyze(b"fn ok() {}", |s| {
            let lines = s.lines().count().max(1);
            Ok((
                AnalysisResult {
                    findings: vec![],
                    source_lines: lines,
                },
                ResourceUsage {
                    cpu_units: 1,
                    memory_bytes: 1,
                    wall_ms: 10 * 60 * 1000,
                },
            ))
        }),
        Err(GuardError::Sandbox(SandboxError::Timeout { .. }))
    ));

    // The engine survived every hostile input; crash accounting is exact.
    let stats = g.monitor().stats();
    assert_eq!(stats.crashes, 1);
    assert_eq!(stats.timeouts, 1);
}

#[test]
fn ast_schema_blocks_malicious_trees() {
    // A deeply nested tree cannot exhaust the validator (iterative traversal),
    // and unknown node kinds are rejected.
    let kinds = ["Module", "Call", "Literal"];
    let mut node = AstNode::leaf("Literal");
    for _ in 0..10_000 {
        node = AstNode::node("Call", vec![node]);
    }
    let deep = AstNode::node("Module", vec![node]);
    let limits = AstLimits {
        max_depth: 50,
        ..AstLimits::default()
    };
    assert!(matches!(
        validate_ast(&deep, &limits, &kinds),
        Err(AstError::TooDeep { .. })
    ));

    let backdoor = AstNode::node("Module", vec![AstNode::leaf("EvalShellcode")]);
    assert!(matches!(
        validate_ast(&backdoor, &AstLimits::default(), &kinds),
        Err(AstError::UnknownKind { .. })
    ));
}

#[test]
fn queue_prioritizes_and_back_pressures() {
    let mut q = AnalysisQueue::new(100);
    q.submit(Priority::Low, 60).unwrap();
    q.submit(Priority::Critical, 60).unwrap();
    // Critical dispatches first.
    let critical = q.dispatch().unwrap().unwrap();
    assert_eq!(critical.priority, Priority::Critical);
    // Low can't fit until critical completes (back-pressure vs exhaustion).
    assert_eq!(q.dispatch(), Err(AdmissionError::AtCapacity));
    q.complete(&critical);
    assert!(q.dispatch().unwrap().is_some());
}

#[test]
fn fuzzing_a_robust_parser_finds_no_crashes() {
    // The guard's own input validator is the parser under test: feed it fuzzed
    // bytes and confirm it never panics (only returns Ok/Err).
    let parser = |bytes: &[u8]| -> Result<(), String> {
        validate_input(bytes, &InputLimits::default())
            .map(|_| ())
            .map_err(|e| format!("{e:?}"))
    };
    let seeds: &[&[u8]] = &[
        b"pub fn main() {}",
        b"contract Foo { fn bar() {} }",
        b"((((deeply))))",
        b"\xff\xfe\x00\x01",
    ];
    let report = fuzz_parser(seeds, 100, parser);
    assert!(report.is_crash_free(), "input validator must never panic");
    assert!(report.total > 400);
}

#[test]
fn result_consistency_detects_nondeterminism() {
    let a = AnalysisResult {
        findings: vec![Finding {
            rule: "r".to_string(),
            severity: "LOW".to_string(),
            line: 1,
        }],
        source_lines: 10,
    };
    let b = a.clone();
    assert!(is_consistent(&a, &b));
    // A second run that drops a finding is inconsistent → flagged.
    let c = AnalysisResult {
        findings: vec![],
        source_lines: 10,
    };
    assert!(!is_consistent(&a, &c));
}

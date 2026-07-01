//! The analysis guard: the safe entry point for analyzing untrusted contracts.
//!
//! Wires the pipeline together: validate & sanitize input → run the analysis in
//! the sandbox (crash-contained, resource- and timeout-bounded) → validate the
//! result for consistency → record monitoring. A malformed or hostile contract
//! can, at worst, be rejected — never crash or hang the engine.

use crate::analysis_guard::input::{self, InputError, ValidatedInput};
use crate::analysis_guard::limits::GuardLimits;
use crate::analysis_guard::monitoring::GuardMonitor;
use crate::analysis_guard::result_check::{self, AnalysisResult, ResultError};
use crate::analysis_guard::sandbox::{ResourceUsage, Sandbox, SandboxError};

/// Why an analysis was not completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardError {
    /// Input validation failed.
    Input(InputError),
    /// Sandboxed execution failed (crash, timeout, resource, or op error).
    Sandbox(SandboxError),
    /// The produced result failed consistency validation.
    Result(ResultError),
}

/// The safe analysis engine wrapper.
pub struct AnalysisGuard {
    limits: GuardLimits,
    monitor: GuardMonitor,
}

impl AnalysisGuard {
    /// Creates a guard with the given limits.
    pub fn new(limits: GuardLimits) -> Self {
        Self {
            limits,
            monitor: GuardMonitor::new(),
        }
    }

    /// The monitor handle.
    pub fn monitor(&self) -> &GuardMonitor {
        &self.monitor
    }

    /// Validates and sanitizes raw contract bytes without running analysis.
    pub fn validate_input(&self, bytes: &[u8]) -> Result<ValidatedInput, InputError> {
        input::validate(bytes, &self.limits.input)
    }

    /// Runs the full guarded pipeline: validate input, execute `analyze` in the
    /// sandbox, then validate the result. `analyze` receives the sanitized
    /// source and returns `(result, usage)` or an error string.
    pub fn analyze<F>(&self, bytes: &[u8], analyze: F) -> Result<AnalysisResult, GuardError>
    where
        F: FnOnce(&str) -> Result<(AnalysisResult, ResourceUsage), String>,
    {
        // 1. Input validation + sanitization.
        let validated = match input::validate(bytes, &self.limits.input) {
            Ok(v) => v,
            Err(e) => {
                self.monitor.record_validation_rejection();
                return Err(GuardError::Input(e));
            }
        };

        // 2. Sandboxed, crash-contained, resource/timeout-bounded execution.
        let sandbox = Sandbox::new(self.limits.resources);
        let source = validated.source;
        let result = match sandbox.run(|| analyze(&source)) {
            Ok(r) => r,
            Err(e) => {
                self.monitor.record_sandbox_error(&e);
                return Err(GuardError::Sandbox(e));
            }
        };

        // 3. Result consistency validation.
        if let Err(e) = result_check::validate(&result) {
            self.monitor.record_validation_rejection();
            return Err(GuardError::Result(e));
        }

        self.monitor.record_success();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis_guard::result_check::Finding;

    fn guard() -> AnalysisGuard {
        AnalysisGuard::new(GuardLimits::default())
    }

    fn ok_result(source: &str) -> Result<(AnalysisResult, ResourceUsage), String> {
        let lines = source.lines().count().max(1);
        Ok((
            AnalysisResult {
                findings: vec![Finding {
                    rule: "reentrancy".to_string(),
                    severity: "HIGH".to_string(),
                    line: 1,
                }],
                source_lines: lines,
            },
            ResourceUsage {
                cpu_units: 100,
                memory_bytes: 1000,
                wall_ms: 50,
            },
        ))
    }

    #[test]
    fn happy_path_succeeds() {
        let g = guard();
        let res = g.analyze(b"pub fn main() {}\n", ok_result).unwrap();
        assert_eq!(res.findings.len(), 1);
        assert_eq!(g.monitor().stats().succeeded, 1);
        assert!(g.monitor().is_crash_free());
    }

    #[test]
    fn invalid_input_is_rejected_before_analysis() {
        let g = guard();
        let mut ran = false;
        let err = g
            .analyze(b"abc\0def", |s| {
                ran = true;
                ok_result(s)
            })
            .unwrap_err();
        assert!(matches!(err, GuardError::Input(InputError::ControlBytes)));
        assert!(!ran, "analysis must not run on invalid input");
        assert_eq!(g.monitor().stats().rejected_validation, 1);
    }

    #[test]
    fn parser_crash_is_contained() {
        let g = guard();
        let err = g
            .analyze(b"pub fn main() {}", |_| panic!("parser bug"))
            .unwrap_err();
        assert_eq!(err, GuardError::Sandbox(SandboxError::Crashed));
        assert_eq!(g.monitor().stats().crashes, 1);
        // The guard itself remains usable and crash accounting is correct.
        assert!(!g.monitor().is_crash_free());
    }

    #[test]
    fn timeout_is_enforced() {
        let g = guard();
        let err = g
            .analyze(b"pub fn main() {}", |s| {
                let lines = s.lines().count().max(1);
                Ok((
                    AnalysisResult {
                        findings: vec![],
                        source_lines: lines,
                    },
                    // Over the 5-minute budget.
                    ResourceUsage {
                        cpu_units: 1,
                        memory_bytes: 1,
                        wall_ms: 6 * 60 * 1000,
                    },
                ))
            })
            .unwrap_err();
        assert!(matches!(
            err,
            GuardError::Sandbox(SandboxError::Timeout { .. })
        ));
        assert_eq!(g.monitor().stats().timeouts, 1);
    }

    #[test]
    fn inconsistent_result_is_rejected() {
        let g = guard();
        // Finding references a line beyond the (1-line) source.
        let err = g
            .analyze(b"x", |_| {
                Ok((
                    AnalysisResult {
                        findings: vec![Finding {
                            rule: "x".to_string(),
                            severity: "HIGH".to_string(),
                            line: 999,
                        }],
                        source_lines: 1,
                    },
                    ResourceUsage::default(),
                ))
            })
            .unwrap_err();
        assert!(matches!(
            err,
            GuardError::Result(ResultError::LineOutOfRange { .. })
        ));
    }
}

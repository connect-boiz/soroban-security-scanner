//! Sandboxed execution for untrusted analysis work.
//!
//! Runs an analysis operation with three protections:
//! - **Crash containment** — the op runs inside `catch_unwind`, so a parser
//!   panic becomes a recoverable `Crashed` error instead of taking down the
//!   process.
//! - **Resource enforcement** — the op reports the CPU/memory/wall-time it
//!   consumed; the sandbox rejects the result if any dimension exceeds the
//!   budget.
//! - **Timeout** — wall-time over the budget (5-minute default) is rejected.

use crate::analysis_guard::limits::ResourceBudget;
use serde::{Deserialize, Serialize};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Resources an analysis operation reports having consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU units consumed.
    pub cpu_units: u64,
    /// Peak memory in bytes.
    pub memory_bytes: u64,
    /// Wall time elapsed in milliseconds.
    pub wall_ms: u64,
}

/// Why a sandboxed run failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxError {
    /// The operation panicked (parser crash, etc.) — contained.
    Crashed,
    /// CPU budget exceeded.
    CpuExceeded {
        /// Reported usage.
        used: u64,
        /// Budget.
        limit: u64,
    },
    /// Memory budget exceeded.
    MemoryExceeded {
        /// Reported usage.
        used: u64,
        /// Budget.
        limit: u64,
    },
    /// Wall-time budget (timeout) exceeded.
    Timeout {
        /// Reported wall time.
        used_ms: u64,
        /// Budget.
        limit_ms: u64,
    },
    /// The operation itself returned an error.
    OperationFailed(String),
}

/// A sandbox enforcing a resource budget.
#[derive(Debug, Clone, Copy)]
pub struct Sandbox {
    budget: ResourceBudget,
}

impl Sandbox {
    /// Creates a sandbox with the given budget.
    pub fn new(budget: ResourceBudget) -> Self {
        Self { budget }
    }

    /// Runs `op`, returning its output only if it neither crashed nor exceeded
    /// any budget. `op` returns `(output, usage)` on success.
    pub fn run<T, F>(&self, op: F) -> Result<T, SandboxError>
    where
        F: FnOnce() -> Result<(T, ResourceUsage), String>,
    {
        // Contain panics: a crashing parser must not abort the process.
        let result = catch_unwind(AssertUnwindSafe(op)).map_err(|_| SandboxError::Crashed)?;
        let (output, usage) = result.map_err(SandboxError::OperationFailed)?;
        self.enforce(usage)?;
        Ok(output)
    }

    /// Checks reported usage against the budget.
    fn enforce(&self, usage: ResourceUsage) -> Result<(), SandboxError> {
        if usage.wall_ms > self.budget.max_wall_ms {
            return Err(SandboxError::Timeout {
                used_ms: usage.wall_ms,
                limit_ms: self.budget.max_wall_ms,
            });
        }
        if usage.cpu_units > self.budget.max_cpu_units {
            return Err(SandboxError::CpuExceeded {
                used: usage.cpu_units,
                limit: self.budget.max_cpu_units,
            });
        }
        if usage.memory_bytes > self.budget.max_memory_bytes {
            return Err(SandboxError::MemoryExceeded {
                used: usage.memory_bytes,
                limit: self.budget.max_memory_bytes,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_budget() -> ResourceBudget {
        ResourceBudget {
            max_cpu_units: 1000,
            max_memory_bytes: 1_000_000,
            max_wall_ms: 5000,
        }
    }

    #[test]
    fn successful_run_within_budget() {
        let sb = Sandbox::new(small_budget());
        let out = sb
            .run(|| {
                Ok((
                    42,
                    ResourceUsage {
                        cpu_units: 10,
                        memory_bytes: 100,
                        wall_ms: 5,
                    },
                ))
            })
            .unwrap();
        assert_eq!(out, 42);
    }

    #[test]
    fn panic_is_contained_as_crash() {
        let sb = Sandbox::new(small_budget());
        let r: Result<i32, _> = sb.run(|| panic!("parser exploded"));
        assert_eq!(r.unwrap_err(), SandboxError::Crashed);
    }

    #[test]
    fn timeout_enforced() {
        let sb = Sandbox::new(small_budget());
        let r: Result<i32, _> = sb.run(|| {
            Ok((
                1,
                ResourceUsage {
                    cpu_units: 1,
                    memory_bytes: 1,
                    wall_ms: 9999,
                },
            ))
        });
        assert!(matches!(r, Err(SandboxError::Timeout { .. })));
    }

    #[test]
    fn cpu_and_memory_enforced() {
        let sb = Sandbox::new(small_budget());
        let cpu: Result<i32, _> = sb.run(|| {
            Ok((
                1,
                ResourceUsage {
                    cpu_units: 2000,
                    memory_bytes: 1,
                    wall_ms: 1,
                },
            ))
        });
        assert!(matches!(cpu, Err(SandboxError::CpuExceeded { .. })));
        let mem: Result<i32, _> = sb.run(|| {
            Ok((
                1,
                ResourceUsage {
                    cpu_units: 1,
                    memory_bytes: 9_000_000,
                    wall_ms: 1,
                },
            ))
        });
        assert!(matches!(mem, Err(SandboxError::MemoryExceeded { .. })));
    }

    #[test]
    fn operation_error_is_propagated() {
        let sb = Sandbox::new(small_budget());
        let r: Result<i32, _> = sb.run(|| Err("invalid contract".to_string()));
        assert_eq!(
            r.unwrap_err(),
            SandboxError::OperationFailed("invalid contract".to_string())
        );
    }
}

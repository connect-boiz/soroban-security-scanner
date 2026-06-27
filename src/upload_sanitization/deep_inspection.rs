//! Deep content inspection for embedded malicious patterns.
//!
//! Scans the file body for indicators that don't belong in legitimate contract
//! code: path-traversal sequences, embedded executables, suspicious shell/eval
//! invocations, and obfuscation signals. Each finding carries a severity so
//! the pipeline can decide between rejecting and quarantining.

use serde::{Deserialize, Serialize};

/// Severity of an inspection finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Worth a human's attention; quarantine.
    Suspicious,
    /// Almost certainly malicious; reject.
    Malicious,
}

/// A single inspection finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Short machine-readable rule id.
    pub rule: String,
    /// Human-readable explanation.
    pub detail: String,
    /// Byte offset where the pattern was found.
    pub offset: usize,
    /// Severity of the finding.
    pub severity: Severity,
}

/// Aggregated inspection result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionReport {
    /// All findings, in discovery order.
    pub findings: Vec<Finding>,
}

impl InspectionReport {
    /// Whether anything was flagged.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Whether any finding is `Malicious`.
    pub fn has_malicious(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Malicious)
    }

    /// The highest severity observed, if any.
    pub fn max_severity(&self) -> Option<Severity> {
        self.findings.iter().map(|f| f.severity).max()
    }
}

/// A byte-pattern rule.
struct Rule {
    id: &'static str,
    needle: &'static [u8],
    detail: &'static str,
    severity: Severity,
}

/// Patterns that should never appear in WASM or Rust contract uploads.
const RULES: &[Rule] = &[
    Rule {
        id: "path-traversal",
        needle: b"../",
        detail: "path traversal sequence",
        severity: Severity::Suspicious,
    },
    Rule {
        id: "path-traversal-win",
        needle: b"..\\",
        detail: "windows path traversal sequence",
        severity: Severity::Suspicious,
    },
    Rule {
        id: "embedded-elf",
        needle: b"\x7fELF",
        detail: "embedded ELF executable",
        severity: Severity::Malicious,
    },
    Rule {
        id: "embedded-mz",
        needle: b"MZ\x90\x00",
        detail: "embedded PE/DOS executable",
        severity: Severity::Malicious,
    },
    Rule {
        id: "shell-rm-rf",
        needle: b"rm -rf",
        detail: "destructive shell command",
        severity: Severity::Malicious,
    },
    Rule {
        id: "reverse-shell",
        needle: b"/dev/tcp/",
        detail: "bash reverse-shell device",
        severity: Severity::Malicious,
    },
    Rule {
        id: "shell-eval",
        needle: b"eval(",
        detail: "dynamic code evaluation",
        severity: Severity::Suspicious,
    },
    Rule {
        id: "powershell-enc",
        needle: b"powershell -enc",
        detail: "encoded PowerShell payload",
        severity: Severity::Malicious,
    },
    Rule {
        id: "process-spawn",
        needle: b"std::process::Command",
        detail: "process spawning in source",
        severity: Severity::Suspicious,
    },
    Rule {
        id: "wget-pipe-sh",
        needle: b"| sh",
        detail: "pipe-to-shell pattern",
        severity: Severity::Suspicious,
    },
];

/// Inspects file bytes for malicious patterns.
pub fn inspect(bytes: &[u8]) -> InspectionReport {
    let mut findings = Vec::new();
    for rule in RULES {
        if let Some(offset) = find_subsequence(bytes, rule.needle) {
            findings.push(Finding {
                rule: rule.id.to_string(),
                detail: rule.detail.to_string(),
                offset,
                severity: rule.severity,
            });
        }
    }
    // Obfuscation heuristic: very long lines suggest minified/packed payloads.
    if let Some(offset) = overlong_line(bytes, 5000) {
        findings.push(Finding {
            rule: "overlong-line".to_string(),
            detail: "abnormally long line (possible obfuscation/packing)".to_string(),
            offset,
            severity: Severity::Suspicious,
        });
    }
    InspectionReport { findings }
}

/// Naive substring search returning the first match offset.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

/// Returns the start offset of the first line longer than `max`, if any.
fn overlong_line(bytes: &[u8], max: usize) -> Option<usize> {
    let mut line_start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            if i - line_start > max {
                return Some(line_start);
            }
            line_start = i + 1;
        }
    }
    if bytes.len() - line_start > max {
        Some(line_start)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_source_has_no_findings() {
        let src = b"pub fn add(a: i32, b: i32) -> i32 { a + b }\n";
        let report = inspect(src);
        assert!(report.is_clean());
        assert!(!report.has_malicious());
    }

    #[test]
    fn detects_path_traversal() {
        let report = inspect(b"include_bytes!(\"../../etc/passwd\")");
        assert!(!report.is_clean());
        assert_eq!(report.max_severity(), Some(Severity::Suspicious));
        assert!(report.findings.iter().any(|f| f.rule == "path-traversal"));
    }

    #[test]
    fn detects_embedded_executable_as_malicious() {
        let mut data = b"// harmless looking\n".to_vec();
        data.extend_from_slice(b"\x7fELF\x02\x01");
        let report = inspect(&data);
        assert!(report.has_malicious());
    }

    #[test]
    fn detects_destructive_shell() {
        let report = inspect(b"system(\"rm -rf /\")");
        assert!(report.has_malicious());
        assert!(report.findings.iter().any(|f| f.rule == "shell-rm-rf"));
    }

    #[test]
    fn detects_overlong_line() {
        let mut data = vec![b'a'; 6000];
        data.push(b'\n');
        let report = inspect(&data);
        assert!(report.findings.iter().any(|f| f.rule == "overlong-line"));
    }

    #[test]
    fn offset_points_to_match() {
        let report = inspect(b"abc/dev/tcp/10.0.0.1/4444");
        let f = report.findings.iter().find(|f| f.rule == "reverse-shell").unwrap();
        assert_eq!(f.offset, 3);
    }
}

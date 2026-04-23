use crate::error::{ServiceError, ServiceResult};
use crate::models::{CodePatch, VulnerabilityReport};
use std::collections::HashMap;

pub struct GitDiffFormatter {
    context_lines: usize,
}

impl GitDiffFormatter {
    pub fn new() -> Self {
        Self {
            context_lines: 3,
        }
    }
    
    pub fn with_context(mut self, context_lines: usize) -> Self {
        self.context_lines = context_lines;
        self
    }
    
    pub fn create_patch_diff(
        &self,
        patch: &CodePatch,
        vulnerability: &VulnerabilityReport,
    ) -> ServiceResult<String> {
        let mut diff = String::new();
        
        // Add diff header
        diff.push_str(&format!(
            "--- a/{}\n",
            vulnerability.file_path
        ));
        diff.push_str(&format!(
            "+++ b/{}\n",
            vulnerability.file_path
        ));
        
        // Parse original and patched code
        let original_lines: Vec<&str> = patch.original_code.lines().collect();
        let patched_lines: Vec<&str> = patch.patched_code.lines().collect();
        
        // Create unified diff
        let hunks = self.create_hunks(&original_lines, &patched_lines);
        
        for hunk in hunks {
            diff.push_str(&self.format_hunk(&hunk, &vulnerability.file_path));
        }
        
        Ok(diff)
    }
    
    pub fn create_full_file_diff(
        &self,
        original_content: &str,
        patched_content: &str,
        file_path: &str,
    ) -> ServiceResult<String> {
        let mut diff = String::new();
        
        // Add diff header
        diff.push_str(&format!("--- a/{}\n", file_path));
        diff.push_str(&format!("+++ b/{}\n", file_path));
        
        // Parse lines
        let original_lines: Vec<&str> = original_content.lines().collect();
        let patched_lines: Vec<&str> = patched_content.lines().collect();
        
        // Create unified diff
        let hunks = self.create_hunks(&original_lines, &patched_lines);
        
        for hunk in hunks {
            diff.push_str(&self.format_hunk(&hunk, file_path));
        }
        
        Ok(diff)
    }
    
    fn create_hunks(&self, original: &[&str], patched: &[&str]) -> Vec<DiffHunk> {
        let mut hunks = Vec::new();
        let mut i = 0;
        let mut j = 0;
        
        while i < original.len() || j < patched.len() {
            // Find the next difference
            let (start_original, start_patched) = self.find_next_difference(original, patched, i, j);
            
            if start_original >= original.len() && start_patched >= patched.len() {
                break;
            }
            
            // Find the end of the difference
            let (end_original, end_patched) = self.find_difference_end(original, patched, start_original, start_patched);
            
            // Create hunk
            let hunk_start = std::cmp::max(
                0,
                start_original.saturating_sub(self.context_lines)
            );
            let hunk_end = std::cmp::min(
                original.len(),
                end_original + self.context_lines
            );
            
            let mut hunk_lines = Vec::new();
            
            // Add context lines before the change
            for line in hunk_start..start_original {
                hunk_lines.push(DiffLine::Context(original[line].to_string()));
            }
            
            // Add removed lines
            for line in start_original..end_original {
                hunk_lines.push(DiffLine::Removed(original[line].to_string()));
            }
            
            // Add added lines
            for line in start_patched..end_patched {
                hunk_lines.push(DiffLine::Added(patched[line].to_string()));
            }
            
            // Add context lines after the change
            for line in end_original..hunk_end {
                hunk_lines.push(DiffLine::Context(original[line].to_string()));
            }
            
            hunks.push(DiffHunk {
                old_start: hunk_start + 1,
                old_lines: (hunk_end - hunk_start) as i32,
                new_start: hunk_start + 1,
                new_lines: (hunk_end - hunk_start + (end_patched - start_patched) - (end_original - start_original)) as i32,
                lines: hunk_lines,
            });
            
            i = hunk_end;
            j = end_patched;
        }
        
        hunks
    }
    
    fn find_next_difference(
        &self,
        original: &[&str],
        patched: &[&str],
        start_orig: usize,
        start_patch: usize,
    ) -> (usize, usize) {
        let mut i = start_orig;
        let mut j = start_patch;
        
        while i < original.len() && j < patched.len() {
            if original[i] != patched[j] {
                return (i, j);
            }
            i += 1;
            j += 1;
        }
        
        (i, j)
    }
    
    fn find_difference_end(
        &self,
        original: &[&str],
        patched: &[&str],
        start_orig: usize,
        start_patch: usize,
    ) -> (usize, usize) {
        let mut i = start_orig;
        let mut j = start_patch;
        
        // Skip all different lines
        while i < original.len() && j < patched.len() && original[i] != patched[j] {
            i += 1;
            j += 1;
        }
        
        // If one file is shorter, continue to the end
        while i < original.len() && (j >= patched.len() || original[i] != patched[j]) {
            i += 1;
        }
        
        while j < patched.len() && (i >= original.len() || original[i] != patched[j]) {
            j += 1;
        }
        
        (i, j)
    }
    
    fn format_hunk(&self, hunk: &DiffHunk, file_path: &str) -> String {
        let mut output = String::new();
        
        // Add hunk header
        output.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.old_start,
            hunk.old_lines,
            hunk.new_start,
            hunk.new_lines
        ));
        
        // Add hunk lines
        for line in &hunk.lines {
            match line {
                DiffLine::Context(content) => {
                    output.push_str(&format!(" {}\n", content));
                },
                DiffLine::Added(content) => {
                    output.push_str(&format!("+{}\n", content));
                },
                DiffLine::Removed(content) => {
                    output.push_str(&format!("-{}\n", content));
                },
            }
        }
        
        output
    }
    
    pub fn create_patch_file(
        &self,
        patches: Vec<(&CodePatch, &VulnerabilityReport)>,
    ) -> ServiceResult<String> {
        let mut patch_file = String::new();
        
        for (patch, vulnerability) in patches {
            patch_file.push_str(&self.create_patch_diff(patch, vulnerability)?);
            patch_file.push('\n');
        }
        
        Ok(patch_file)
    }
    
    pub fn apply_patch(&self, diff_content: &str, target_dir: &str) -> ServiceResult<String> {
        use std::process::Command;
        
        // Create a temporary patch file
        let temp_file = std::env::temp_dir().join("remediation.patch");
        std::fs::write(&temp_file, diff_content)?;
        
        // Apply the patch using git apply
        let output = Command::new("git")
            .args(["apply", temp_file.to_str().unwrap()])
            .current_dir(target_dir)
            .output()?;
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);
        
        if output.status.success() {
            Ok("Patch applied successfully".to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(ServiceError::GitError(
                git2::Error::from_str(&format!("Failed to apply patch: {}", error))
            ))
        }
    }
    
    pub fn validate_patch(&self, diff_content: &str) -> ServiceResult<bool> {
        use std::process::Command;
        
        // Create a temporary patch file
        let temp_file = std::env::temp_dir().join("validation.patch");
        std::fs::write(&temp_file, diff_content)?;
        
        // Validate the patch using git apply --check
        let output = Command::new("git")
            .args(["apply", "--check", temp_file.to_str().unwrap()])
            .output()?;
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);
        
        Ok(output.status.success())
    }
    
    pub fn get_patch_summary(&self, diff_content: &str) -> PatchSummary {
        let lines: Vec<&str> = diff_content.lines().collect();
        let mut added_lines = 0;
        let mut removed_lines = 0;
        let mut files_changed = std::collections::HashSet::new();
        
        for line in lines {
            if line.starts_with("+++ b/") {
                let file_path = line.strip_prefix("+++ b/").unwrap_or("");
                files_changed.insert(file_path.to_string());
            } else if line.starts_with("+") && !line.starts_with("+++") {
                added_lines += 1;
            } else if line.starts_with("-") && !line.starts_with("---") {
                removed_lines += 1;
            }
        }
        
        PatchSummary {
            files_changed: files_changed.len(),
            lines_added: added_lines,
            lines_removed: removed_lines,
        }
    }
}

#[derive(Debug)]
struct DiffHunk {
    old_start: usize,
    old_lines: i32,
    new_start: usize,
    new_lines: i32,
    lines: Vec<DiffLine>,
}

#[derive(Debug)]
enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

#[derive(Debug)]
pub struct PatchSummary {
    pub files_changed: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::VulnerabilityReport;
    
    #[test]
    fn test_simple_diff() {
        let formatter = GitDiffFormatter::new();
        
        let patch = CodePatch {
            original_code: "let x = 1;\nlet y = 2;\nlet z = x + y;".to_string(),
            patched_code: "let x = 1;\nlet y = 2;\nlet z = x.checked_add(y).unwrap_or(0);".to_string(),
            explanation: "Added overflow protection".to_string(),
            security_improvements: vec!["Prevented integer overflow".to_string()],
        };
        
        let vulnerability = VulnerabilityReport {
            id: "test-1".to_string(),
            file_path: "src/contract.rs".to_string(),
            vulnerability_type: "IntegerOverflow".to_string(),
            severity: "High".to_string(),
            title: "Integer Overflow".to_string(),
            description: "Potential integer overflow".to_string(),
            code_snippet: "let z = x + y;".to_string(),
            line_number: 3,
            sarif_report: None,
        };
        
        let diff = formatter.create_patch_diff(&patch, &vulnerability).unwrap();
        
        assert!(diff.contains("--- a/src/contract.rs"));
        assert!(diff.contains("+++ b/src/contract.rs"));
        assert!(diff.contains("-let z = x + y;"));
        assert!(diff.contains("+let z = x.checked_add(y).unwrap_or(0);"));
    }
    
    #[test]
    fn test_patch_summary() {
        let formatter = GitDiffFormatter::new();
        
        let diff = r#"
--- a/src/contract.rs
+++ b/src/contract.rs
@@ -1,3 +1,3 @@
 let x = 1;
 let y = 2;
-let z = x + y;
+let z = x.checked_add(y).unwrap_or(0);
"#;
        
        let summary = formatter.get_patch_summary(diff);
        
        assert_eq!(summary.files_changed, 1);
        assert_eq!(summary.lines_added, 1);
        assert_eq!(summary.lines_removed, 1);
    }
}

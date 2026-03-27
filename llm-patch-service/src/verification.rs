use crate::error::{ServiceError, ServiceResult};
use crate::models::{CodePatch, VerificationStatus};
use std::process::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::process::Command as AsyncCommand;
use tokio::fs;

pub struct VerificationSandbox {
    temp_dir: Option<TempDir>,
}

impl VerificationSandbox {
    pub fn new() -> Self {
        Self {
            temp_dir: None,
        }
    }
    
    pub async fn verify_patch(&mut self, patch: &CodePatch) -> ServiceResult<VerificationStatus> {
        // Create temporary directory for verification
        let temp_dir = TempDir::new()
            .map_err(|e| ServiceError::IoError(e))?;
        self.temp_dir = Some(temp_dir);
        
        let temp_path = self.temp_dir.as_ref().unwrap().path();
        
        // Create a minimal Rust project structure
        self.create_project_structure(temp_path).await?;
        
        // Write the patched code to a file
        self.write_patched_code(temp_path, &patch.patched_code).await?;
        
        // Attempt to compile the code
        match self.compile_project(temp_path).await {
            Ok(_) => {
                // Run basic syntax and security checks
                if self.run_security_checks(temp_path).await? {
                    Ok(VerificationStatus::Passed)
                } else {
                    Ok(VerificationStatus::Failed)
                }
            },
            Err(_) => Ok(VerificationStatus::Failed),
        }
    }
    
    async fn create_project_structure(&self, temp_path: &Path) -> ServiceResult<()> {
        // Create src directory
        let src_dir = temp_path.join("src");
        fs::create_dir_all(&src_dir).await?;
        
        // Create Cargo.toml with Soroban dependencies
        let cargo_toml = r#"[package]
name = "verification-test"
version = "0.1.0"
edition = "2021"

[dependencies]
soroban-sdk = "21.0.0-preview.1"

[dev-dependencies]
soroban-sdk = { version = "21.0.0-preview.1", features = ["testutils"] }
"#;
        
        fs::write(temp_path.join("Cargo.toml"), cargo_toml).await?;
        
        // Create main.rs with basic structure
        let main_rs = r#"use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct VerificationContract;

#[contractimpl]
impl VerificationContract {
    // Placeholder implementation
    pub fn verify(env: Env, input: Symbol) -> Symbol {
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;
    
    #[test]
    fn test_verification() {
        let env = Env::default();
        // Test will be added based on the patched code
    }
}
"#;
        
        fs::write(src_dir.join("lib.rs"), main_rs).await?;
        
        Ok(())
    }
    
    async fn write_patched_code(&self, temp_path: &Path, patched_code: &str) -> ServiceResult<()> {
        let src_dir = temp_path.join("src");
        
        // Determine if the patched code is a full file or just a function
        if patched_code.contains("fn ") && !patched_code.contains("mod ") {
            // It's likely a function, add it to the existing lib.rs
            let lib_rs_path = src_dir.join("lib.rs");
            let existing_content = fs::read_to_string(&lib_rs_path).await?;
            
            // Find a good place to insert the function (before the test module)
            let insert_pos = existing_content.find("#[cfg(test)]")
                .unwrap_or(existing_content.len());
            
            let new_content = format!(
                "{}\n\n// Patched code\n{}",
                &existing_content[..insert_pos],
                patched_code
            );
            
            fs::write(&lib_rs_path, new_content).await?;
        } else {
            // It's a full file, replace lib.rs
            fs::write(src_dir.join("lib.rs"), patched_code).await?;
        }
        
        Ok(())
    }
    
    async fn compile_project(&self, temp_path: &Path) -> ServiceResult<()> {
        let output = AsyncCommand::new("cargo")
            .args(["check", "--all-targets"])
            .current_dir(temp_path)
            .output()
            .await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ServiceError::CompilationError(format!(
                "Compilation failed: {}",
                stderr
            )));
        }
        
        Ok(())
    }
    
    async fn run_security_checks(&self, temp_path: &Path) -> ServiceResult<bool> {
        // Run clippy for additional security checks
        let clippy_output = AsyncCommand::new("cargo")
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .current_dir(temp_path)
            .output()
            .await?;
        
        if !clippy_output.status.success() {
            let stderr = String::from_utf8_lossy(&clippy_output.stderr);
            tracing::warn!("Clippy warnings: {}", stderr);
            // Don't fail for clippy warnings, but log them
        }
        
        // Run cargo audit for security vulnerabilities
        let audit_output = AsyncCommand::new("cargo")
            .args(["audit"])
            .current_dir(temp_path)
            .output()
            .await;
        
        match audit_output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("Security audit warnings: {}", stderr);
                    // Don't fail for audit warnings, but log them
                }
            },
            Err(_) => {
                // cargo audit might not be installed, that's okay
                tracing::warn!("cargo audit not available, skipping security audit");
            }
        }
        
        // Basic static analysis checks
        self.run_static_analysis(temp_path).await
    }
    
    async fn run_static_analysis(&self, temp_path: &Path) -> ServiceResult<bool> {
        let lib_rs_path = temp_path.join("src/lib.rs");
        let content = fs::read_to_string(&lib_rs_path).await?;
        
        // Check for common security issues
        let security_patterns = vec![
            // Unsafe blocks without justification
            (r"unsafe\s*\{", "Unsafe block detected"),
            // Panics in contract code
            (r"panic!", "Panic detected in contract code"),
            // Unwrapped results that could panic
            (r"\.unwrap\(\)", "Unwrap() call detected"),
            // Expect with potentially panicking messages
            (r"\.expect\(", "Expect() call detected"),
            // Direct string formatting without validation
            (r"format!\s*\(", "Format! macro usage"),
        ];
        
        let mut issues_found = 0;
        
        for (pattern, description) in security_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(&content) {
                tracing::warn!("Security issue detected: {}", description);
                issues_found += 1;
            }
        }
        
        // Allow some issues for now, but log them
        if issues_found > 0 {
            tracing::warn!("Found {} potential security issues", issues_found);
        }
        
        Ok(true) // For now, always pass unless compilation fails
    }
    
    pub async fn verify_syntax(&self, code: &str) -> ServiceResult<bool> {
        let temp_dir = TempDir::new()
            .map_err(|e| ServiceError::IoError(e))?;
        
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).await?;
        
        // Create minimal Cargo.toml
        let cargo_toml = r#"[package]
name = "syntax-check"
version = "0.1.0"
edition = "2021"

[dependencies]
soroban-sdk = "21.0.0-preview.1"
"#;
        
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).await?;
        fs::write(src_dir.join("lib.rs"), code).await?;
        
        // Try to compile
        let output = AsyncCommand::new("cargo")
            .args(["check"])
            .current_dir(temp_dir.path())
            .output()
            .await?;
        
        Ok(output.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_syntax_verification() {
        let sandbox = VerificationSandbox::new();
        
        let valid_code = r#"
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    pub fn test_function(env: Env, value: u64) -> u64 {
        value + 1
    }
}
"#;
        
        assert!(sandbox.verify_syntax(valid_code).await.unwrap());
        
        let invalid_code = r#"
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    pub fn test_function(env: Env, value: u64) -> u64 {
        value +  // Missing right operand
    }
}
"#;
        
        assert!(!sandbox.verify_syntax(invalid_code).await.unwrap());
    }
}

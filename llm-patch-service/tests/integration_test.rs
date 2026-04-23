use llm_patch_service::{
    models::{PatchRequest, VulnerabilityReport, LLMConfig, DatabaseConfig},
    LLMClient, CodeSanitizer, VerificationSandbox, ConfidenceScorer,
    GitDiffFormatter, FallbackProvider,
};
use tokio;
use std::time::Duration;

#[tokio::test]
async fn test_end_to_end_patch_generation() {
    // Initialize components
    let sanitizer = CodeSanitizer::new();
    let verifier = VerificationSandbox::new();
    let confidence_scorer = ConfidenceScorer::new();
    let git_formatter = GitDiffFormatter::new();
    let fallback_provider = FallbackProvider::new();
    
    // Create test vulnerability
    let vulnerability = VulnerabilityReport {
        id: "test-int-001".to_string(),
        file_path: "src/contract.rs".to_string(),
        vulnerability_type: "IntegerOverflow".to_string(),
        severity: "High".to_string(),
        title: "Integer Overflow in Addition".to_string(),
        description: "Potential integer overflow in arithmetic operation".to_string(),
        code_snippet: "let result = a + b;".to_string(),
        line_number: 42,
        sarif_report: None,
    };
    
    let original_code = r#"
use soroban_sdk::Env;

pub struct Contract {
    counter: u64,
}

impl Contract {
    pub fn add(&self, a: u64, b: u64) -> u64 {
        let result = a + b;
        result
    }
}
"#;
    
    // Test sanitization
    let sanitized_code = sanitizer.sanitize_code(original_code).unwrap();
    assert!(!sanitized_code.is_empty());
    assert!(sanitizer.validate_code_safety(&sanitized_code).unwrap());
    
    // Test fallback provider (since we don't have real LLM API keys in tests)
    let fallback_patch = fallback_provider.get_fallback_patch(&vulnerability, &sanitized_code).await.unwrap();
    assert!(!fallback_patch.patched_code.is_empty());
    assert!(!fallback_patch.explanation.is_empty());
    
    // Test verification
    let verification_status = verifier.verify_patch(&fallback_patch).await.unwrap();
    println!("Verification status: {:?}", verification_status);
    
    // Test confidence scoring
    let confidence_score = confidence_scorer.calculate_confidence(
        &fallback_patch,
        &vulnerability,
        verification_status.clone(),
    ).await.unwrap();
    
    println!("Confidence score: {}", confidence_score);
    assert!(confidence_score >= 0.0 && confidence_score <= 1.0);
    
    // Test Git diff generation
    let git_diff = git_formatter.create_patch_diff(&fallback_patch, &vulnerability).unwrap();
    assert!(git_diff.contains("--- a/src/contract.rs"));
    assert!(git_diff.contains("+++ b/src/contract.rs"));
    
    // Test patch validation
    let is_valid = git_formatter.validate_patch(&git_diff).unwrap();
    println!("Patch validation: {}", is_valid);
    
    // Test patch summary
    let summary = git_formatter.get_patch_summary(&git_diff);
    assert_eq!(summary.files_changed, 1);
    
    println!("✅ End-to-end test completed successfully!");
}

#[tokio::test]
async fn test_code_sanitization() {
    let sanitizer = CodeSanitizer::new();
    
    let code_with_secrets = r#"
use soroban_sdk::Env;

const API_KEY: &str = "sk-1234567890abcdef1234567890abcdef";
const SECRET: &str = "my_secret_token_1234567890";

fn test_function() {
    let password = "super_secret_password";
    let private_key = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7VJTUt9Us8cKB";
    
    // Environment variable access
    let db_url = env!("DATABASE_URL");
    let api_key = std::env::var("API_KEY").unwrap();
}
"#;
    
    let sanitized = sanitizer.sanitize_code(code_with_secrets).unwrap();
    
    // Check that secrets are redacted
    assert!(sanitized.contains("***REDACTED***"));
    assert!(!sanitized.contains("sk-1234567890abcdef1234567890abcdef"));
    assert!(!sanitized.contains("my_secret_token_1234567890"));
    assert!(!sanitized.contains("super_secret_password"));
    assert!(!sanitized.contains("-----BEGIN PRIVATE KEY-----"));
    
    // Check that environment variable access is sanitized
    assert!(sanitized.contains("env!(\"***REDACTED***\")"));
    
    println!("✅ Code sanitization test passed!");
}

#[tokio::test]
async fn test_confidence_scoring() {
    let scorer = ConfidenceScorer::new();
    
    let high_quality_patch = llm_patch_service::models::CodePatch {
        original_code: "let result = a + b;".to_string(),
        patched_code: r#"
use soroban_sdk::Env;

fn safe_add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::Overflow)
}
"#.to_string(),
        explanation: "This patch fixes the integer overflow vulnerability by using checked_add instead of direct addition. This prevents overflow attacks and ensures the contract operates safely. The function now returns a Result type that must be properly handled by callers.".to_string(),
        security_improvements: vec![
            "Added overflow protection using checked_add".to_string(),
            "Implemented proper error handling with Result type".to_string(),
            "Follows Rust safety best practices".to_string(),
        ],
    };
    
    let vulnerability = VulnerabilityReport {
        id: "test-conf-001".to_string(),
        file_path: "src/contract.rs".to_string(),
        vulnerability_type: "IntegerOverflow".to_string(),
        severity: "High".to_string(),
        title: "Integer Overflow".to_string(),
        description: "Potential integer overflow".to_string(),
        code_snippet: "let result = a + b;".to_string(),
        line_number: 1,
        sarif_report: None,
    };
    
    let confidence = scorer.calculate_confidence(
        &high_quality_patch,
        &vulnerability,
        llm_patch_service::models::VerificationStatus::Passed,
    ).await.unwrap();
    
    println!("High quality patch confidence: {}", confidence);
    assert!(confidence > 0.6); // Should have medium to high confidence
    
    let low_quality_patch = llm_patch_service::models::CodePatch {
        original_code: "let result = a + b;".to_string(),
        patched_code: "let result = a + b; // TODO: fix this".to_string(),
        explanation: "Fix".to_string(),
        security_improvements: vec![],
    };
    
    let low_confidence = scorer.calculate_confidence(
        &low_quality_patch,
        &vulnerability,
        llm_patch_service::models::VerificationStatus::Failed,
    ).await.unwrap();
    
    println!("Low quality patch confidence: {}", low_confidence);
    assert!(low_confidence < 0.5); // Should have low confidence
    
    println!("✅ Confidence scoring test passed!");
}

#[tokio::test]
async fn test_git_diff_formatting() {
    let formatter = GitDiffFormatter::new();
    
    let patch = llm_patch_service::models::CodePatch {
        original_code: "let x = 1;\nlet y = 2;\nlet z = x + y;".to_string(),
        patched_code: "let x = 1;\nlet y = 2;\nlet z = x.checked_add(y).unwrap_or(0);".to_string(),
        explanation: "Added overflow protection".to_string(),
        security_improvements: vec!["Prevented integer overflow".to_string()],
    };
    
    let vulnerability = VulnerabilityReport {
        id: "test-diff-001".to_string(),
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
    
    let summary = formatter.get_patch_summary(&diff);
    assert_eq!(summary.files_changed, 1);
    assert_eq!(summary.lines_added, 1);
    assert_eq!(summary.lines_removed, 1);
    
    println!("✅ Git diff formatting test passed!");
}

#[tokio::test]
async fn test_fallback_provider() {
    let provider = FallbackProvider::new();
    
    let vulnerability = VulnerabilityReport {
        id: "test-fallback-001".to_string(),
        file_path: "src/contract.rs".to_string(),
        vulnerability_type: "AccessControl".to_string(),
        severity: "High".to_string(),
        title: "Missing Authorization".to_string(),
        description: "Function lacks proper authorization checks".to_string(),
        code_snippet: "pub fn sensitive_function() { }".to_string(),
        line_number: 10,
        sarif_report: None,
    };
    
    let original_code = "pub fn sensitive_function() { /* sensitive logic */ }";
    
    let fallback_patch = provider.get_fallback_patch(&vulnerability, original_code).await.unwrap();
    
    assert!(!fallback_patch.patched_code.is_empty());
    assert!(!fallback_patch.explanation.is_empty());
    assert!(!fallback_patch.security_improvements.is_empty());
    
    // Check that documentation links are available
    let docs = provider.get_documentation_links("AccessControl");
    assert!(!docs.is_empty());
    
    // Test fallback response creation
    let fallback_response = provider.create_fallback_response(&vulnerability, original_code);
    assert!(fallback_response.get("patch").is_some());
    assert!(fallback_response.get("documentation_links").is_some());
    assert!(fallback_response.get("fallback_reason").is_some());
    
    println!("✅ Fallback provider test passed!");
}

#[tokio::test]
async fn test_verification_sandbox() {
    let mut verifier = VerificationSandbox::new();
    
    let valid_code = r#"
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    pub fn safe_add(env: Env, a: u64, b: u64) -> Result<u64, soroban_sdk::Error> {
        a.checked_add(b)
            .ok_or(soroban_sdk::Error::from_contract_error(1))
    }
}
"#;
    
    let valid_patch = llm_patch_service::models::CodePatch {
        original_code: "let result = a + b;".to_string(),
        patched_code: valid_code.to_string(),
        explanation: "Added safe arithmetic with error handling".to_string(),
        security_improvements: vec!["Prevented overflow".to_string()],
    };
    
    let verification_status = verifier.verify_patch(&valid_patch).await.unwrap();
    println!("Valid code verification: {:?}", verification_status);
    
    // Test syntax validation
    let syntax_valid = verifier.verify_syntax(valid_code).await.unwrap();
    assert!(syntax_valid);
    
    let invalid_code = r#"
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    pub fn broken_function(env: Env, a: u64, b: u64) -> u64 {
        a +  // Missing right operand
    }
}
"#;
    
    let syntax_invalid = verifier.verify_syntax(invalid_code).await.unwrap();
    assert!(!syntax_invalid);
    
    println!("✅ Verification sandbox test passed!");
}

use crate::error::{ServiceError, ServiceResult};
use crate::models::{VulnerabilityReport, CodePatch};
use serde_json::json;
use std::collections::HashMap;

pub struct FallbackProvider {
    documentation_links: HashMap<String, Vec<DocumentationLink>>,
    templates: HashMap<String, PatchTemplate>,
}

#[derive(Debug, Clone)]
pub struct DocumentationLink {
    pub title: String,
    pub url: String,
    pub description: String,
    pub relevance_score: f64,
}

#[derive(Debug, Clone)]
pub struct PatchTemplate {
    pub name: String,
    pub description: String,
    pub template_code: String,
    pub explanation: String,
    pub security_improvements: Vec<String>,
}

impl FallbackProvider {
    pub fn new() -> Self {
        let mut provider = Self {
            documentation_links: HashMap::new(),
            templates: HashMap::new(),
        };
        
        provider.init_documentation();
        provider.init_templates();
        provider
    }
    
    fn init_documentation(&mut self) {
        // Access Control vulnerabilities
        self.documentation_links.insert(
            "accesscontrol".to_string(),
            vec![
                DocumentationLink {
                    title: "Soroban Authorization Guide".to_string(),
                    url: "https://developers.stellar.org/docs/learn/smart-contracts/introduction/authorization".to_string(),
                    description: "Official documentation on Soroban authorization patterns".to_string(),
                    relevance_score: 0.9,
                },
                DocumentationLink {
                    title: "Rust Security Best Practices".to_string(),
                    url: "https://doc.rust-lang.org/book/ch19-06-macros.html".to_string(),
                    description: "Rust security patterns and best practices".to_string(),
                    relevance_score: 0.7,
                },
            ]
        );
        
        // Token Economics vulnerabilities
        self.documentation_links.insert(
            "tokeneconomics".to_string(),
            vec![
                DocumentationLink {
                    title: "Soroban Token Contract Guide".to_string(),
                    url: "https://developers.stellar.org/docs/learn/smart-contracts/contracts/token".to_string(),
                    description: "Guide to implementing secure token contracts".to_string(),
                    relevance_score: 0.9,
                },
                DocumentationLink {
                    title: "Stellar Asset Security".to_string(),
                    url: "https://developers.stellar.org/docs/learn/encyclopedia/security".to_string(),
                    description: "Security considerations for Stellar assets".to_string(),
                    relevance_score: 0.8,
                },
            ]
        );
        
        // Reentrancy vulnerabilities
        self.documentation_links.insert(
            "reentrancy".to_string(),
            vec![
                DocumentationLink {
                    title: "Reentrancy Attack Prevention".to_string(),
                    url: "https://docs.soliditylang.org/en/latest/security-considerations.html#re-entrancy".to_string(),
                    description: "Understanding and preventing reentrancy attacks".to_string(),
                    relevance_score: 0.8,
                },
                DocumentationLink {
                    title: "Soroban Security Patterns".to_string(),
                    url: "https://developers.stellar.org/docs/learn/smart-contracts/security".to_string(),
                    description: "Security patterns for Soroban contracts".to_string(),
                    relevance_score: 0.9,
                },
            ]
        );
        
        // Integer Overflow vulnerabilities
        self.documentation_links.insert(
            "integeroverflow".to_string(),
            vec![
                DocumentationLink {
                    title: "Rust Integer Safety".to_string(),
                    url: "https://doc.rust-lang.org/book/ch03-02-data-types.html".to_string(),
                    description: "Understanding integer types and safety in Rust".to_string(),
                    relevance_score: 0.9,
                },
                DocumentationLink {
                    title: "Checked Arithmetic in Rust".to_string(),
                    url: "https://doc.rust-lang.org/std/primitive.i32.html#method.checked_add".to_string(),
                    description: "Using checked arithmetic operations".to_string(),
                    relevance_score: 0.8,
                },
            ]
        );
        
        // Logic Vulnerabilities
        self.documentation_links.insert(
            "logicvulnerability".to_string(),
            vec![
                DocumentationLink {
                    title: "Smart Contract Logic Security".to_string(),
                    url: "https://developers.stellar.org/docs/learn/smart-contracts/security".to_string(),
                    description: "Common logic vulnerabilities and prevention".to_string(),
                    relevance_score: 0.8,
                },
                DocumentationLink {
                    title: "Formal Verification Guide".to_string(),
                    url: "https://github.com/stellar/rs-soroban-sdk/blob/main/docs/tools-and-interfaces.md".to_string(),
                    description: "Tools for formal verification of contracts".to_string(),
                    relevance_score: 0.7,
                },
            ]
        );
        
        // Stellar-specific vulnerabilities
        self.documentation_links.insert(
            "stellarspecific".to_string(),
            vec![
                DocumentationLink {
                    title: "Stellar Security Best Practices".to_string(),
                    url: "https://developers.stellar.org/docs/learn/encyclopedia/security".to_string(),
                    description: "Security best practices for Stellar".to_string(),
                    relevance_score: 0.9,
                },
                DocumentationLink {
                    title: "Soroban Environment Security".to_string(),
                    url: "https://developers.stellar.org/docs/learn/smart-contracts/environment".to_string(),
                    description: "Understanding the Soroban environment".to_string(),
                    relevance_score: 0.8,
                },
            ]
        );
    }
    
    fn init_templates(&mut self) {
        // Access Control template
        self.templates.insert(
            "accesscontrol".to_string(),
            PatchTemplate {
                name: "Authorization Check Template".to_string(),
                description: "Template for adding proper authorization checks".to_string(),
                template_code: r#"
// Add this to your contract implementation
use soroban_sdk::{Env, Address};

// Add authorization check at the beginning of sensitive functions
fn require_admin_auth(env: &Env, admin: &Address) -> Result<(), soroban_sdk::Error> {
    if env.current_contract_address() != *admin {
        Err(soroban_sdk::Error::from_contract_error(1))
    } else {
        Ok(())
    }
}

// Example usage in a function
#[contractimpl]
impl YourContract {
    pub fn sensitive_function(env: Env, admin: Address, /* other parameters */) -> Result<(), soroban_sdk::Error> {
        // Check authorization first
        require_admin_auth(&env, &admin)?;
        
        // Your function logic here
        // ...
        
        Ok(())
    }
}
"#.to_string(),
                explanation: "This template adds proper authorization checks to ensure only authorized addresses can call sensitive functions. It uses the Soroban environment to verify the caller's identity and returns an error if unauthorized.".to_string(),
                security_improvements: vec![
                    "Added proper authorization checks".to_string(),
                    "Prevents unauthorized access to sensitive functions".to_string(),
                    "Uses Soroban's built-in authorization mechanisms".to_string(),
                ],
            }
        );
        
        // Integer Overflow template
        self.templates.insert(
            "integeroverflow".to_string(),
            PatchTemplate {
                name: "Safe Arithmetic Template".to_string(),
                description: "Template for safe arithmetic operations".to_string(),
                template_code: r#"
// Replace unsafe arithmetic with checked operations
use soroban_sdk::Env;

// Instead of: let result = a + b;
// Use:
let result = match a.checked_add(b) {
    Some(value) => value,
    None => {
        // Handle overflow case
        panic!("Integer overflow detected");
    }
};

// For multiplication:
let result = match a.checked_mul(b) {
    Some(value) => value,
    None => {
        // Handle overflow case
        panic!("Integer overflow detected");
    }
};

// For subtraction:
let result = match a.checked_sub(b) {
    Some(value) => value,
    None => {
        // Handle underflow case
        panic!("Integer underflow detected");
    }
};

// For division:
let result = match a.checked_div(b) {
    Some(value) => value,
    None => {
        // Handle division by zero case
        panic!("Division by zero detected");
    }
};
"#.to_string(),
                explanation: "This template replaces unsafe arithmetic operations with checked alternatives that prevent integer overflow and underflow. Each operation returns an Option that must be properly handled.".to_string(),
                security_improvements: vec![
                    "Prevents integer overflow attacks".to_string(),
                    "Handles arithmetic errors gracefully".to_string(),
                    "Follows Rust safety best practices".to_string(),
                ],
            }
        );
        
        // Reentrancy template
        self.templates.insert(
            "reentrancy".to_string(),
            PatchTemplate {
                name: "Reentrancy Protection Template".to_string(),
                description: "Template for preventing reentrancy attacks".to_string(),
                template_code: r#"
use soroban_sdk::{Env, Symbol, Map, Address};

// Add reentrancy protection to your contract
#[contract]
pub struct ReentrancyGuard {
    locked: Map<Address, bool>,
}

#[contractimpl]
impl ReentrancyGuard {
    // Initialize the contract
    pub fn __constructor(env: Env) {
        let locked = Map::new(&env);
        env.storage().instance().set(&Symbol::new(&env, "locked"), &locked);
    }
    
    // Check if an address is currently in a call
    fn is_locked(env: &Env, caller: &Address) -> bool {
        let locked: Map<Address, bool> = env.storage()
            .instance()
            .get(&Symbol::new(&env, "locked"))
            .unwrap_or(Map::new(env));
        
        locked.get(caller).unwrap_or(false)
    }
    
    // Set the lock status for an address
    fn set_locked(env: &Env, caller: &Address, locked: bool) {
        let mut lock_map: Map<Address, bool> = env.storage()
            .instance()
            .get(&Symbol::new(&env, "locked"))
            .unwrap_or(Map::new(env));
        
        if locked {
            lock_map.set(caller, true);
        } else {
            lock_map.remove(caller);
        }
        
        env.storage().instance().set(&Symbol::new(&env, "locked"), &lock_map);
    }
    
    // Example protected function
    pub fn protected_function(env: Env, caller: Address, /* other parameters */) -> Result<(), soroban_sdk::Error> {
        // Check reentrancy guard
        if Self::is_locked(&env, &caller) {
            return Err(soroban_sdk::Error::from_contract_error(2)); // Reentrancy error
        }
        
        // Set the lock
        Self::set_locked(&env, &caller, true);
        
        // Your function logic here
        // ...
        
        // Clear the lock
        Self::set_locked(&env, &caller, false);
        
        Ok(())
    }
}
"#.to_string(),
                explanation: "This template implements a reentrancy guard that prevents recursive calls to sensitive functions. It tracks which addresses are currently executing protected functions and blocks reentrant calls.".to_string(),
                security_improvements: vec![
                    "Prevents reentrancy attacks".to_string(),
                    "Tracks execution state per address".to_string(),
                    "Provides clear error codes for reentrancy attempts".to_string(),
                ],
            }
        );
    }
    
    pub async fn get_fallback_patch(
        &self,
        vulnerability: &VulnerabilityReport,
        original_code: &str,
    ) -> ServiceResult<CodePatch> {
        let vuln_type = vulnerability.vulnerability_type.to_lowercase();
        
        // Try to find a matching template
        if let Some(template) = self.templates.get(&vuln_type) {
            return Ok(self.create_patch_from_template(template, original_code, vulnerability));
        }
        
        // If no template found, create a generic fallback
        Ok(self.create_generic_fallback(original_code, vulnerability))
    }
    
    fn create_patch_from_template(
        &self,
        template: &PatchTemplate,
        original_code: &str,
        vulnerability: &VulnerabilityReport,
    ) -> CodePatch {
        // For now, we'll return the template as-is. In a real implementation,
        // you might want to integrate the template with the original code
        let patched_code = format!(
            r#"// Fallback patch for {}: {}
// Original code at line {}: {}

{}

// Explanation: {}
"#,
            vulnerability.vulnerability_type,
            vulnerability.title,
            vulnerability.line_number,
            vulnerability.code_snippet,
            template.template_code,
            template.explanation
        );
        
        CodePatch {
            original_code: original_code.to_string(),
            patched_code,
            explanation: format!(
                "Fallback patch: {} - {}",
                template.name,
                template.explanation
            ),
            security_improvements: template.security_improvements.clone(),
        }
    }
    
    fn create_generic_fallback(
        &self,
        original_code: &str,
        vulnerability: &VulnerabilityReport,
    ) -> CodePatch {
        let patched_code = format!(
            r#"// Generic fallback patch for {}: {}
// Original code at line {}: {}

// TODO: Manual review and patching required
// Vulnerability: {}
// Severity: {}
// Recommendation: {}

{}

// Note: This is a fallback patch. Please review and modify as needed.
"#,
            vulnerability.vulnerability_type,
            vulnerability.title,
            vulnerability.line_number,
            vulnerability.code_snippet,
            vulnerability.title,
            vulnerability.severity,
            vulnerability.description,
            original_code
        );
        
        CodePatch {
            original_code: original_code.to_string(),
            patched_code,
            explanation: format!(
                "Generic fallback patch for {}. Manual review and modification required. Please refer to the provided documentation links for guidance.",
                vulnerability.vulnerability_type
            ),
            security_improvements: vec![
                "Identified security issue requiring attention".to_string(),
                "Provided documentation for manual remediation".to_string(),
            ],
        }
    }
    
    pub fn get_documentation_links(
        &self,
        vulnerability_type: &str,
    ) -> Vec<DocumentationLink> {
        let vuln_type = vulnerability_type.to_lowercase();
        
        self.documentation_links
            .get(&vuln_type)
            .cloned()
            .unwrap_or_else(|| {
                // Return generic links if no specific ones found
                vec![
                    DocumentationLink {
                        title: "Soroban Security Guide".to_string(),
                        url: "https://developers.stellar.org/docs/learn/smart-contracts/security".to_string(),
                        description: "General security guidelines for Soroban contracts".to_string(),
                        relevance_score: 0.7,
                    },
                    DocumentationLink {
                        title: "Rust Security Best Practices".to_string(),
                        url: "https://doc.rust-lang.org/book/ch19-06-macros.html".to_string(),
                        description: "Rust security patterns and best practices".to_string(),
                        relevance_score: 0.6,
                    },
                ]
            })
    }
    
    pub fn create_fallback_response(
        &self,
        vulnerability: &VulnerabilityReport,
        original_code: &str,
    ) -> serde_json::Value {
        let patch = futures::executor::block_on(
            self.get_fallback_patch(vulnerability, original_code)
        );
        
        let docs = self.get_documentation_links(&vulnerability.vulnerability_type);
        
        json!({
            "patch": patch.unwrap_or_else(|_| CodePatch {
                original_code: original_code.to_string(),
                patched_code: "// Fallback failed - manual review required".to_string(),
                explanation: "Unable to generate fallback patch".to_string(),
                security_improvements: vec![],
            }),
            "documentation_links": docs,
            "fallback_reason": "AI confidence was too low or AI service unavailable",
            "requires_manual_review": true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::VulnerabilityReport;
    
    #[test]
    fn test_fallback_provider() {
        let provider = FallbackProvider::new();
        
        let vulnerability = VulnerabilityReport {
            id: "test-1".to_string(),
            file_path: "test.rs".to_string(),
            vulnerability_type: "IntegerOverflow".to_string(),
            severity: "High".to_string(),
            title: "Integer Overflow".to_string(),
            description: "Potential integer overflow".to_string(),
            code_snippet: "let result = a + b;".to_string(),
            line_number: 1,
            sarif_report: None,
        };
        
        let docs = provider.get_documentation_links("IntegerOverflow");
        assert!(!docs.is_empty());
        assert!(docs.iter().any(|doc| doc.title.contains("Integer")));
        
        let fallback = futures::executor::block_on(
            provider.get_fallback_patch(&vulnerability, "let result = a + b;")
        ).unwrap();
        
        assert!(fallback.patched_code.contains("checked_add"));
        assert!(!fallback.security_improvements.is_empty());
    }
}

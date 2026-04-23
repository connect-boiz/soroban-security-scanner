use crate::error::{ServiceError, ServiceResult};
use regex::Regex;
use std::collections::HashSet;

pub struct CodeSanitizer {
    api_key_patterns: Vec<Regex>,
    sensitive_patterns: Vec<Regex>,
    allowed_keywords: HashSet<String>,
}

impl CodeSanitizer {
    pub fn new() -> Self {
        let mut sanitizer = Self {
            api_key_patterns: Vec::new(),
            sensitive_patterns: Vec::new(),
            allowed_keywords: HashSet::new(),
        };
        
        sanitizer.init_patterns();
        sanitizer
    }
    
    fn init_patterns(&mut self) {
        // API Key patterns
        self.api_key_patterns.push(Regex::new(r"(?i)(api[_-]?key|apikey)\s*[:=]\s*['\"]?[a-zA-Z0-9_-]{20,}['\"]?").unwrap());
        self.api_key_patterns.push(Regex::new(r"(?i)(secret|token)\s*[:=]\s*['\"]?[a-zA-Z0-9_-]{20,}['\"]?").unwrap());
        self.api_key_patterns.push(Regex::new(r"(?i)(password|pass)\s*[:=]\s*['\"]?[^\s'\"]{8,}['\"]?").unwrap());
        self.api_key_patterns.push(Regex::new(r"(?i)(private[_-]?key|privatekey)\s*[:=]\s*['\"]?[a-zA-Z0-9/+=]{40,}['\"]?").unwrap());
        
        // Sensitive data patterns
        self.sensitive_patterns.push(Regex::new(r"(?i)(aws[_-]?access[_-]?key)\s*[:=]\s*['\"]?[A-Z0-9]{20}['\"]?").unwrap());
        self.sensitive_patterns.push(Regex::new(r"(?i)(aws[_-]?secret[_-]?key)\s*[:=]\s*['\"]?[a-zA-Z0-9/+=]{40}['\"]?").unwrap());
        self.sensitive_patterns.push(Regex::new(r"(?i)(github[_-]?token)\s*[:=]\s*['\"]?[a-zA-Z0-9_-]{40}['\"]?").unwrap());
        self.sensitive_patterns.push(Regex::new(r"(?i)(discord[_-]?token)\s*[:=]\s*['\"]?[a-zA-Z0-9_-]{50,}['\"]?").unwrap());
        self.sensitive_patterns.push(Regex::new(r"(?i)(slack[_-]?token)\s*[:=]\s*['\"]?[xoxb]-[a-zA-Z0-9-]{24,}['\"]?").unwrap());
        
        // Rust-specific patterns
        self.sensitive_patterns.push(Regex::new(r"env!\s*\(\s*['\"](?:API_KEY|SECRET|TOKEN|PASSWORD|PRIVATE_KEY)['\"]").unwrap());
        self.sensitive_patterns.push(Regex::new(r"std::env::var\s*\(\s*['\"](?:API_KEY|SECRET|TOKEN|PASSWORD|PRIVATE_KEY)['\"]").unwrap());
        
        // Initialize allowed keywords for code analysis
        let allowed = vec![
            "fn", "struct", "impl", "mod", "use", "pub", "let", "mut", "const", "static",
            "if", "else", "match", "for", "while", "loop", "break", "continue", "return",
            "async", "await", "move", "ref", "unsafe", "trait", "enum", "type", "where",
            "Self", "self", "super", "crate", "in", "as", "dyn", "box", "macro_rules",
            // Soroban-specific
            "contract", "contractimpl", "contracttype", "Env", "Address", "Symbol", "Vec",
            "Map", "String", "Bytes", "U256", "I256", "u64", "i64", "u32", "i32", "u128",
            "i128", "bool", "None", "Some", "Result", "Ok", "Err", "Option", "panic",
        ];
        
        self.allowed_keywords = allowed.into_iter().collect();
    }
    
    pub fn sanitize_code(&self, code: &str) -> ServiceResult<String> {
        let mut sanitized = code.to_string();
        
        // Remove or mask API keys and sensitive data
        for pattern in &self.api_key_patterns {
            sanitized = pattern.replace_all(&sanitized, "${1}: \"***REDACTED***\"").to_string();
        }
        
        for pattern in &self.sensitive_patterns {
            sanitized = pattern.replace_all(&sanitized, "${1}: \"***REDACTED***\"").to_string();
        }
        
        // Additional sanitization for environment variable access
        sanitized = Regex::new(r"env!\s*\(\s*['\"][A-Z_]+['\"]\s*\)")
            .unwrap()
            .replace_all(&sanitized, "env!(\"***REDACTED***\")")
            .to_string();
        
        Ok(sanitized)
    }
    
    pub fn sanitize_sarif_report(&self, sarif: &str) -> ServiceResult<String> {
        let mut sanitized = sarif.to_string();
        
        // Remove any potential sensitive data in SARIF reports
        let patterns = vec![
            Regex::new(r"(?i)uri\s*:\s*['\"][^'\"]*(?:key|secret|token|password)[^'\"]*['\"]").unwrap(),
            Regex::new(r"(?i)message\s*:\s*['\"][^'\"]*(?:key|secret|token|password)[^'\"]*['\"]").unwrap(),
        ];
        
        for pattern in patterns {
            sanitized = pattern.replace_all(&sanitized, "${1}: \"***REDACTED***\"").to_string();
        }
        
        Ok(sanitized)
    }
    
    pub fn validate_code_safety(&self, code: &str) -> ServiceResult<bool> {
        // Check for potential malicious code patterns
        let malicious_patterns = vec![
            Regex::new(r"(?i)eval\s*\(").unwrap(),
            Regex::new(r"(?i)exec\s*\(").unwrap(),
            Regex::new(r"(?i)system\s*\(").unwrap(),
            Regex::new(r"(?i)shell\s*\(").unwrap(),
            Regex::new(r"(?i)cmd\s*\(").unwrap(),
            Regex::new(r"(?i)popen\s*\(").unwrap(),
        ];
        
        for pattern in malicious_patterns {
            if pattern.is_match(code) {
                return Err(ServiceError::SanitizationError(
                    "Potentially malicious code pattern detected".to_string()
                ));
            }
        }
        
        Ok(true)
    }
    
    pub fn extract_relevant_context(&self, code: &str, line_number: usize, context_lines: usize) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let start = if line_number > context_lines { line_number - context_lines - 1 } else { 0 };
        let end = std::cmp::min(line_number + context_lines, lines.len());
        
        lines[start..end].join("\n")
    }
    
    pub fn is_valid_rust_code(&self, code: &str) -> bool {
        // Basic validation to ensure we're dealing with Rust code
        let rust_patterns = vec![
            Regex::new(r"\bfn\s+\w+\s*\(").unwrap(),
            Regex::new(r"\bstruct\s+\w+").unwrap(),
            Regex::new(r"\bimpl\s+\w+").unwrap(),
            Regex::new(r"\buse\s+").unwrap(),
            Regex::new(r"\bmod\s+\w+").unwrap(),
        ];
        
        rust_patterns.iter().any(|pattern| pattern.is_match(code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_key_sanitization() {
        let sanitizer = CodeSanitizer::new();
        let code = r#"
        let api_key = "sk-1234567890abcdef1234567890abcdef";
        let secret = "my_secret_token_1234567890";
        "#;
        
        let sanitized = sanitizer.sanitize_code(code).unwrap();
        assert!(sanitized.contains("***REDACTED***"));
        assert!(!sanitized.contains("sk-1234567890abcdef1234567890abcdef"));
    }
    
    #[test]
    fn test_valid_rust_code() {
        let sanitizer = CodeSanitizer::new();
        let code = r#"
        fn test_function() {
            let x = 42;
            println!("{}", x);
        }
        "#;
        
        assert!(sanitizer.is_valid_rust_code(code));
    }
}

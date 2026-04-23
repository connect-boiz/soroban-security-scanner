use crate::error::{ServiceError, ServiceResult};
use crate::models::{VulnerabilityReport, PatchRequest, CodePatch, LLMConfig};
use serde_json::{json, Value};
use std::time::Duration;

pub struct LLMClient {
    client: reqwest::Client,
    config: LLMConfig,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
            
        Self { client, config }
    }
    
    pub async fn generate_patch(&self, request: &PatchRequest) -> ServiceResult<CodePatch> {
        let prompt = self.build_secure_prompt(request)?;
        let response = self.call_llm(&prompt).await?;
        self.parse_patch_response(response, &request.original_code)
    }
    
    fn build_secure_prompt(&self, request: &PatchRequest) -> ServiceResult<String> {
        let base_prompt = r#"
You are a senior Rust security expert specializing in Soroban smart contract security. 
Your task is to analyze the provided vulnerability and generate a secure code patch.

IMPORTANT SECURITY GUIDELINES:
1. Never expose or transmit sensitive data, API keys, or secrets
2. Focus only on the security vulnerability identified
3. Ensure the patch follows Rust best practices and Soroban security patterns
4. Provide clear explanations of the security improvements made
5. Do not introduce new vulnerabilities or side effects

VULNERABILITY ANALYSIS REQUEST:
"#;
        
        let vulnerability_context = format!(r#"
Vulnerability Type: {}
Severity: {}
Title: {}
Description: {}
File: {}
Line Number: {}

Code Snippet:
```rust
{}
```

Additional Context: {}
"#,
            request.vulnerability.vulnerability_type,
            request.vulnerability.severity,
            request.vulnerability.title,
            request.vulnerability.description,
            request.vulnerability.file_path,
            request.vulnerability.line_number,
            request.vulnerability.code_snippet,
            request.context.as_deref().unwrap_or("None")
        );
        
        let patch_instructions = r#"

Please provide:
1. A secure patched version of the vulnerable code
2. A detailed explanation of the security improvements made
3. A list of specific security issues addressed

Format your response as JSON:
{
  "patched_code": "secure version of the code",
  "explanation": "detailed explanation of changes",
  "security_improvements": ["list of improvements"]
}

Focus on maintaining functionality while improving security.
"#;
        
        if let Some(sarif) = &request.vulnerability.sarif_report {
            let sarif_context = format!(r#"

SARIF Analysis Report:
{}
"#, serde_json::to_string_pretty(sarif).unwrap_or_default());
            
            Ok(format!("{}{}{}{}", base_prompt, vulnerability_context, sarif_context, patch_instructions))
        } else {
            Ok(format!("{}{}{}", base_prompt, vulnerability_context, patch_instructions))
        }
    }
    
    async fn call_llm(&self, prompt: &str) -> ServiceResult<Value> {
        let request_body = match self.config.provider.to_lowercase().as_str() {
            "openai" => self.build_openai_request(prompt)?,
            "anthropic" => self.build_anthropic_request(prompt)?,
            "ollama" => self.build_ollama_request(prompt)?,
            _ => return Err(ServiceError::ConfigurationError(
                format!("Unsupported LLM provider: {}", self.config.provider)
            )),
        };
        
        let url = self.get_provider_url()?;
        
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request_body)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceError::LLMError(
                reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("LLM API error: {} - {}", response.status(), error_text)
                ))
            ));
        }
        
        let response_json: Value = response.json().await?;
        Ok(response_json)
    }
    
    fn build_openai_request(&self, prompt: &str) -> ServiceResult<Value> {
        Ok(json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a senior Rust security expert specializing in Soroban smart contract security."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "response_format": {
                "type": "json_object"
            }
        }))
    }
    
    fn build_anthropic_request(&self, prompt: &str) -> ServiceResult<Value> {
        Ok(json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
    }
    
    fn build_ollama_request(&self, prompt: &str) -> ServiceResult<Value> {
        Ok(json!({
            "model": self.config.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens
            }
        }))
    }
    
    fn get_provider_url(&self) -> ServiceResult<String> {
        match self.config.provider.to_lowercase().as_str() {
            "openai" => Ok("https://api.openai.com/v1/chat/completions".to_string()),
            "anthropic" => Ok("https://api.anthropic.com/v1/messages".to_string()),
            "ollama" => Ok(format!("{}/api/generate", self.config.api_key)), // api_key used as base URL for ollama
            _ => Err(ServiceError::ConfigurationError(
                format!("Unsupported LLM provider: {}", self.config.provider)
            )),
        }
    }
    
    fn parse_patch_response(&self, response: Value, original_code: &str) -> ServiceResult<CodePatch> {
        let content = match self.config.provider.to_lowercase().as_str() {
            "openai" => {
                response.get("choices")
                    .and_then(|choices| choices.get(0))
                    .and_then(|choice| choice.get("message"))
                    .and_then(|message| message.get("content"))
                    .and_then(|content| content.as_str())
                    .ok_or_else(|| ServiceError::LLMError(
                        reqwest::Error::from(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Invalid response format from OpenAI"
                        ))
                    ))?
            },
            "anthropic" => {
                response.get("content")
                    .and_then(|content| content.get(0))
                    .and_then(|message| message.get("text"))
                    .and_then(|text| text.as_str())
                    .ok_or_else(|| ServiceError::LLMError(
                        reqwest::Error::from(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Invalid response format from Anthropic"
                        ))
                    ))?
            },
            "ollama" => {
                response.get("response")
                    .and_then(|response| response.as_str())
                    .ok_or_else(|| ServiceError::LLMError(
                        reqwest::Error::from(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Invalid response format from Ollama"
                        ))
                    ))?
            },
            _ => return Err(ServiceError::ConfigurationError(
                format!("Unsupported LLM provider: {}", self.config.provider)
            )),
        };
        
        // Parse JSON response
        let patch_data: Value = serde_json::from_str(content)
            .map_err(|e| ServiceError::SerializationError(e))?;
        
        let patched_code = patch_data.get("patched_code")
            .and_then(|code| code.as_str())
            .ok_or_else(|| ServiceError::LLMError(
                reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Missing patched_code in response"
                ))
            ))?;
        
        let explanation = patch_data.get("explanation")
            .and_then(|exp| exp.as_str())
            .unwrap_or("No explanation provided");
        
        let security_improvements: Vec<String> = patch_data.get("security_improvements")
            .and_then(|improvements| improvements.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(CodePatch {
            original_code: original_code.to_string(),
            patched_code: patched_code.to_string(),
            explanation: explanation.to_string(),
            security_improvements,
        })
    }
    
    pub async fn test_connection(&self) -> ServiceResult<bool> {
        let test_prompt = "Respond with a simple JSON: {\"status\": \"ok\"}";
        let _ = self.call_llm(test_prompt).await?;
        Ok(true)
    }
}

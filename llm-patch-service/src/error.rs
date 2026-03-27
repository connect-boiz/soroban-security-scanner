use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("LLM API error: {0}")]
    LLMError(#[from] reqwest::Error),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Code compilation failed: {0}")]
    CompilationError(String),
    
    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Sanitization failed: {0}")]
    SanitizationError(String),
    
    #[error("Verification failed: {0}")]
    VerificationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;

# LLM Patch Service

A secure microservice that utilizes Large Language Models (LLMs) to analyze identified vulnerabilities and generate secure Rust code patches for Soroban smart contracts.

## 🚀 Features

### Core Functionality
- **Secure Prompt Pipeline**: Sends vulnerable code snippets and SARIF reports to LLMs with comprehensive security measures
- **Code Sanitization Layer**: Prevents sensitive API keys and credentials from being sent to AI providers
- **Verification Sandbox**: Automatically checks if AI-generated patches compile and pass basic security analysis
- **Confidence Scoring**: Provides confidence scores based on static analysis of patched code
- **Remediation Database**: Stores successful remediations for fine-tuning future suggestions
- **Fallback Mechanism**: Provides standard documentation links when AI confidence is low
- **One-Click Apply**: Formats patches as standard Git diffs for easy application

### Security Features
- **Input Sanitization**: Removes API keys, secrets, and sensitive data before sending to LLMs
- **Malicious Code Detection**: Identifies and blocks potentially malicious patterns
- **Compilation Verification**: Ensures generated patches compile successfully
- **Security Analysis**: Runs clippy and cargo audit for additional security checks
- **Access Control**: Proper authorization checks for all endpoints

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Vulnerability  │    │   Code           │    │   LLM           │
│   Report        │───▶│   Sanitization   │───▶│   Analysis      │
│                 │    │   Layer          │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                                                         ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Git Diff      │    │   Verification   │    │   Confidence    │
│   Formatter     │◀───│   Sandbox        │◀───│   Scoring       │
│                 │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   One-Click     │    │   Security       │    │   Database      │
│   Apply         │    │   Verification   │    │   Storage       │
│                 │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## 📋 Requirements

- Rust 1.75+
- PostgreSQL 12+
- Docker & Docker Compose (optional)
- LLM API access (OpenAI, Anthropic, or Ollama)

## 🚀 Quick Start

### 1. Environment Setup

Copy the example environment file:
```bash
cp .env.example .env
```

Edit `.env` with your configuration:
```bash
# LLM Configuration
LLM_PROVIDER=openai
LLM_API_KEY=your_openai_api_key_here
LLM_MODEL=gpt-4

# Database
DATABASE_URL=postgresql://username:password@localhost:5432/llm_patch_service
```

### 2. Database Setup

Create the database and run migrations:
```bash
createdb llm_patch_service
cargo run --bin llm-patch-service -- migrate
```

### 3. Run the Service

```bash
cargo run --bin llm-patch-service
```

The service will start on `http://localhost:8080`

### 4. Docker Deployment

```bash
# Build the image
docker build -t llm-patch-service .

# Run with environment variables
docker run -p 8080:8080 \
  -e LLM_API_KEY=your_key_here \
  -e DATABASE_URL=your_db_url \
  llm-patch-service
```

## 📡 API Endpoints

### Generate Patch
```http
POST /patch
Content-Type: application/json

{
  "vulnerability": {
    "id": "vuln-123",
    "file_path": "src/contract.rs",
    "vulnerability_type": "IntegerOverflow",
    "severity": "High",
    "title": "Integer Overflow in Addition",
    "description": "Potential integer overflow in arithmetic operation",
    "code_snippet": "let result = a + b;",
    "line_number": 42
  },
  "original_code": "fn add(a: u64, b: u64) -> u64 { a + b }",
  "context": "This is a token contract function"
}
```

**Response:**
```json
{
  "id": "remediation-456",
  "vulnerability_id": "vuln-123",
  "patch": {
    "original_code": "fn add(a: u64, b: u64) -> u64 { a + b }",
    "patched_code": "fn add(a: u64, b: u64) -> Result<u64, Error> { a.checked_add(b) }",
    "explanation": "Added overflow protection using checked_add",
    "security_improvements": ["Prevents integer overflow attacks"]
  },
  "confidence_score": 0.85,
  "verification_status": "Passed",
  "git_diff": "--- a/src/contract.rs\n+++ b/src/contract.rs\n@@ -1,3 +1,3 @@\n-fn add(a: u64, b: u64) -> u64 { a + b }\n+fn add(a: u64, b: u64) -> Result<u64, Error> { a.checked_add(b) }\n",
  "fallback_provided": false,
  "created_at": "2024-03-27T10:30:00Z"
}
```

### Apply Patch
```http
POST /patch/{remediation_id}/apply
Content-Type: application/json

{
  "target_dir": "/path/to/your/project"
}
```

### Get Remediation History
```http
GET /history/{vulnerability_id}
```

### Get Service Stats
```http
GET /stats
```

### Health Check
```http
GET /health
```

## 🔧 Configuration

### LLM Providers

#### OpenAI
```bash
LLM_PROVIDER=openai
LLM_API_KEY=sk-...
LLM_MODEL=gpt-4
```

#### Anthropic Claude
```bash
LLM_PROVIDER=anthropic
LLM_API_KEY=sk-ant-...
LLM_MODEL=claude-3-sonnet-20240229
```

#### Ollama (Local)
```bash
LLM_PROVIDER=ollama
LLM_API_KEY=http://localhost:11434
LLM_MODEL=llama2
```

### Security Settings

- `MIN_CONFIDENCE_THRESHOLD`: Minimum confidence score (default: 0.4)
- `ENABLE_SANITIZATION`: Enable code sanitization (default: true)
- `ENABLE_VERIFICATION`: Enable patch verification (default: true)

## 🔒 Security Features

### Code Sanitization

The service automatically removes:
- API keys and secrets
- Database credentials
- Environment variable access
- Sensitive configuration data

### Verification Sandbox

Each patch is verified for:
- **Compilation Success**: Code must compile without errors
- **Security Analysis**: Clippy linting and cargo audit
- **Static Analysis**: Checks for common security issues

### Confidence Scoring

Confidence is calculated based on:
- **Verification Status** (30%): Compilation and security check results
- **Code Quality** (25%): Rust best practices and patterns
- **Security Improvements** (20%): Specific security enhancements
- **Explanation Quality** (15%): Detail and clarity of explanations
- **Vulnerability Complexity** (10%): Difficulty of the fix

## 📊 Monitoring

### Metrics Available

- Total remediations generated
- Applied remediations
- Average confidence scores
- Success rates
- Verification pass rates

### Logging

The service provides structured logging with:
- Request/response details
- Error tracking
- Performance metrics
- Security events

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html
```

## 🔧 Development

### Project Structure

```
src/
├── main.rs              # Service entry point and API endpoints
├── lib.rs               # Library exports
├── models.rs            # Data structures and types
├── sanitization.rs      # Code sanitization layer
├── llm_client.rs        # LLM integration
├── verification.rs      # Patch verification sandbox
├── confidence.rs        # Confidence scoring system
├── database.rs          # Database operations
├── git_diff.rs          # Git diff formatting
├── fallback.rs          # Fallback mechanisms
└── error.rs             # Error handling
```

### Adding New LLM Providers

1. Update `LLMClient::build_request()` in `llm_client.rs`
2. Add provider-specific URL in `LLMClient::get_provider_url()`
3. Update response parsing in `LLMClient::parse_patch_response()`

### Adding New Vulnerability Types

1. Update patterns in `sanitization.rs`
2. Add templates in `fallback.rs`
3. Update confidence scoring in `confidence.rs`

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 🆘 Support

For issues and questions:
- Create an issue in the repository
- Check the documentation
- Review the logs for detailed error information

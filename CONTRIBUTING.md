# Contributing to Soroban Security Scanner

Thank you for your interest in contributing to the Soroban Security Scanner! This document provides guidelines and information for contributors.

## 🚀 Getting Started

### Prerequisites

- Node.js 18+
- Rust 1.70+
- Soroban CLI
- Docker & Docker Compose
- PostgreSQL 15+
- Redis 7+

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/soroban-security-scanner.git
   cd soroban-security-scanner
   ```

2. **Install Dependencies**
   ```bash
   # Frontend
   cd frontend
   npm install
   
   # Backend
   cd ../backend
   cargo build
   
   # Core Scanner
   cd ../core-scanner
   cargo build
   
   # Smart Contracts
   cd ../contracts
   cargo build
   ```

3. **Set up Environment**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Start Development Environment**
   ```bash
   docker-compose up -d
   ```

5. **Run Database Migrations**
   ```bash
   psql -h localhost -U scanner_user -d soroban_scanner -f scripts/init-db.sql
   ```

## 🏗️ Project Structure

```
soroban-security-scanner/
├── frontend/                 # Next.js web application
├── backend/                  # Rust API server
├── core-scanner/            # Security analysis engine
├── contracts/               # Soroban smart contracts
├── docs/                    # Documentation
├── scripts/                 # Development scripts
└── .github/                 # GitHub workflows and templates
```

## 🤝 How to Contribute

### Reporting Bugs

- Use the [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md) template
- Include reproduction steps, expected behavior, and actual behavior
- Provide relevant code snippets and error messages

### Suggesting Features

- Use the [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md) template
- Describe the use case and proposed solution
- Consider implementation complexity and impact

### Submitting Vulnerability Reports

- **For security vulnerabilities**: Follow our responsible disclosure policy
- **For contract vulnerabilities**: Use the platform's reporting system
- **For tool issues**: Create an issue with the "vulnerability" label

### Code Contributions

1. **Create a Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Follow Coding Standards**
   - Rust: Use `cargo fmt` and `cargo clippy`
   - TypeScript/React: Use ESLint and Prettier
   - Write tests for new functionality

3. **Commit Changes**
   ```bash
   git commit -m "feat: add new vulnerability detection pattern"
   ```

4. **Push and Create PR**
   ```bash
   git push origin feature/your-feature-name
   # Create Pull Request on GitHub
   ```

## 📝 Development Guidelines

### Code Style

#### Rust
- Use `cargo fmt` for formatting
- Run `cargo clippy` before committing
- Write comprehensive tests
- Document public APIs with `///` comments

#### TypeScript/React
- Use TypeScript strictly
- Follow React best practices
- Use functional components with hooks
- Write unit tests with Jest

#### Smart Contracts
- Follow Soroban contract patterns
- Include comprehensive test coverage
- Document contract interfaces
- Use proper error handling

### Testing

```bash
# Backend tests
cd backend && cargo test

# Core scanner tests
cd core-scanner && cargo test

# Smart contract tests
cd contracts && cargo test

# Frontend tests
cd frontend && npm test
```

### Documentation

- Update README.md for significant changes
- Add inline documentation for complex logic
- Update API documentation for backend changes
- Document new vulnerability patterns

## 🐛 Bug Reports

When reporting bugs, please include:

- **Environment**: OS, Rust version, Node.js version
- **Reproduction**: Steps to reproduce the issue
- **Expected**: What should happen
- **Actual**: What actually happened
- **Logs**: Relevant error messages or logs

## ✨ Feature Requests

When requesting features, please include:

- **Use Case**: Why this feature is needed
- **Proposal**: How you envision the feature working
- **Alternatives**: Considered alternatives and why they weren't chosen
- **Impact**: Who would benefit from this feature

## 🔒 Security Vulnerability Disclosure

For security vulnerabilities in our platform:

1. **Do not** open a public issue
2. Email us at security@soroban-scanner.dev
3. Include detailed description and reproduction steps
4. We'll respond within 48 hours
5. We'll work with you to fix and disclose responsibly

## 📊 Vulnerability Patterns

When adding new vulnerability detection patterns:

1. **Pattern Definition**: Add to `core-scanner/src/patterns.rs`
2. **Test Cases**: Create comprehensive tests
3. **Documentation**: Document the vulnerability type
4. **Severity Assessment**: Assign appropriate severity level
5. **False Positive Check**: Minimize false positives

## 🚀 Deployment

### Development
```bash
docker-compose up -d
```

### Production
```bash
# Set production environment variables
export NODE_ENV=production
export RUST_LOG=info

# Deploy with Docker
docker-compose -f docker-compose.prod.yml up -d
```

## 📋 Review Process

All contributions go through review:

1. **Automated Checks**: CI/CD pipeline runs tests and linting
2. **Code Review**: Maintainers review code changes
3. **Security Review**: Security-sensitive changes get extra review
4. **Documentation**: Documentation is updated as needed
5. **Testing**: Adequate test coverage is required

## 🎯 Priority Areas

We're currently focusing on:

1. **New Vulnerability Patterns**: Expanding detection capabilities
2. **Performance**: Improving scan speed and accuracy
3. **User Experience**: Making the platform more user-friendly
4. **Integration**: Better toolchain integration
5. **Documentation**: Improving guides and API docs

## 🏆 Recognition

Contributors are recognized through:

- **Contributors List**: Listed in README.md
- **Release Notes**: Mentioned in release notes
- **Reputation System**: On-chain reputation for vulnerability reports
- **Bounty Programs**: Financial rewards for significant contributions

## 📞 Getting Help

- **Discord**: Join our community Discord
- **GitHub Issues**: For bug reports and feature requests
- **Documentation**: Check our docs directory
- **Email**: info@soroban-scanner.dev

## 📄 License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Soroban Security Scanner! Your help makes the Stellar ecosystem safer for everyone. 🚀

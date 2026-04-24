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
=======
# 🤝 Contributing to Stellar Security Scanner

Thank you for your interest in contributing to the Stellar Security Scanner! This guide will help you get started and understand our contribution process.

## 🎯 Mission

Our mission is to create the most comprehensive security scanning tool for Stellar smart contracts, helping developers build secure and reliable applications on the Stellar network.

## 🚀 Getting Started

### **Prerequisites**

- Rust 1.70+ installed
- Git configured
- GitHub account
- Discord account (for community communication)

### **Step 1: Setup Development Environment**

```bash
# Clone the repository
git clone https://github.com/your-org/stellar-security-scanner.git
cd stellar-security-scanner

# Install dependencies
cargo build

# Run tests to verify setup
cargo test

# Install development tools
cargo install cargo-watch cargo-tarpaulin
```

### **Step 2: Join the Community**

1. **Discord**: Join our [Discord server](https://discord.gg/stellar-security)
2. **Introductions**: Post in `#introductions` channel
3. **Contributors**: Join `#contributors` for project discussions

### **Step 3: Choose Your Contribution Type**

## 📋 Contribution Types

### **🔒 Security Research (High Priority)**

**What**: Discover and document new vulnerability patterns for Stellar contracts

**Reward**: 200-500 USDC per vulnerability pattern

**Requirements**:
- Detailed vulnerability analysis
- Proof of concept exploit
- Test cases demonstrating the issue
- Recommended mitigation strategies

**Example Issues**:
- `feat: Add detection for flash loan attacks`
- `feat: Implement oracle manipulation pattern detection`
- `feat: Add cross-contract reentrancy detection`

**Process**:
1. Research Stellar contract vulnerabilities
2. Create detailed issue with findings
3. Develop detection pattern
4. Write comprehensive tests
5. Submit pull request with documentation

### **🧪 Development Tasks (Medium Priority)**

**What**: Implement new features and improvements

**Reward**: 100-300 USDC per feature

**Requirements**:
- Working implementation
- Comprehensive test suite
- Documentation updates
- Performance benchmarks (if applicable)

**Example Issues**:
- `feat: Add JSON output format support`
- `feat: Implement parallel scanning for large projects`
- `feat: Add VS Code extension integration`
- `feat: Create web-based reporting dashboard`

**Process**:
1. Claim an issue from the project board
2. Create feature branch
3. Implement solution
4. Add tests and documentation
5. Submit pull request

### **📚 Documentation (Low Priority)**

**What**: Improve project documentation and examples

**Reward**: 50-150 USDC per contribution

**Requirements**:
- Clear, accurate content
- Code examples
- Screenshots/diagrams (if applicable)
- User testing feedback

**Example Issues**:
- `docs: Add tutorial for custom vulnerability patterns`
- `docs: Create integration guide for CI/CD pipelines`
- `docs: Write best practices guide`
- `docs: Add video tutorials`

**Process**:
1. Identify documentation gaps
2. Create content outline
3. Write comprehensive documentation
4. Get community feedback
5. Submit improvements

### **🐛 Bug Fixes (Variable Priority)**

**What**: Fix reported bugs and issues

**Reward**: 50-400 USDC per fix

**Based on**: Severity, impact, and complexity

**Requirements**:
- Bug reproduction steps
- Root cause analysis
- Fix implementation
- Regression tests

**Example Issues**:
- `fix: Handle edge case in integer overflow detection`
- `fix: Resolve memory leak in large file scanning`
- `fix: Correct false positive in access control detection`

## 🔄 Development Workflow

### **1. Claim an Issue**

```bash
# Check available issues
gh issue list --label "help wanted"

# Comment on issue to claim it
# "I'd like to work on this issue. I have experience with..."
```

### **2. Create Branch**

```bash
# Create feature branch
git checkout -b feat/your-feature-name

# Or for bug fixes
git checkout -b fix/issue-number-description
```

### **3. Development**

```bash
# Watch for changes and run tests
cargo watch -x test

# Check code coverage
cargo tarpaulin --out Html

# Format code
cargo fmt

# Run lints
cargo clippy -- -D warnings
```

### **4. Testing**

```bash
# Run all tests
cargo test

# Run specific test module
cargo test vulnerabilities

# Run integration tests
cargo test --test integration_tests

# Check performance
cargo bench
```

### **5. Submit Pull Request**

```bash
# Push changes
git push origin feat/your-feature-name

# Create pull request
gh pr create --title "Brief description" --body "Detailed description"
```

## 📝 Pull Request Guidelines

### **PR Template**

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Security improvement
- [ ] Performance optimization

## Testing
- [ ] All tests pass
- [ ] New tests added
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Performance impact considered

## Funding Request
- Amount: XXX USDC
- Hours spent: XX hours
- Complexity: Low/Medium/High
```

### **Code Quality Standards**

- **Test Coverage**: Minimum 90%
- **Documentation**: All public functions documented
- **Performance**: No significant performance regressions
- **Security**: Follow security best practices

## 💰 Funding Process

### **Step 1: Submit Proposal**

1. Create detailed issue description
2. Estimate time and complexity
3. Specify funding amount
4. Wait for maintainer approval

### **Step 2: Get Approved**

- Maintainers review proposal
- Community feedback considered
- Funding amount confirmed
- Issue assigned to contributor

### **Step 3: Development**

- Receive 50% upfront payment
- Start development work
- Provide regular updates
- Meet agreed timeline

### **Step 4: Completion**

- Submit pull request
- Code review and approval
- Documentation updated
- Tests passing

### **Step 5: Final Payment**

- PR merged to main branch
- Remaining 50% payment released
- Contributor badge awarded
- Impact metrics recorded

## 🏅 Recognition System

### **Contributor Badges**

- 🥉 **Bronze**: 0-500 USDC contributed
- 🥈 **Silver**: 500-2,000 USDC contributed
- 🥇 **Gold**: 2,000-5,000 USDC contributed
- 💎 **Platinum**: 5,000+ USDC contributed

### **Monthly Recognition**

- **Top Contributor**: Highest impact contribution
- **Security Champion**: Best security research
- **Innovation Award**: Most creative solution
- **Community Hero**: Helpful and supportive

### **Annual Awards**

- **Contributor of the Year**: Overall excellence
- **Security Researcher**: Outstanding vulnerability research
- **Community Builder**: Fostering community growth

## 📞 Getting Help

### **Resources**

- **Documentation**: [Project Docs](https://docs.stellar-security-scanner.io)
- **API Reference**: [Rust Docs](https://docs.rs/stellar-security-scanner)
- **Examples**: [GitHub Examples](https://github.com/your-org/stellar-security-scanner/tree/main/examples)

### **Community Support**

- **Discord**: #help channel for technical questions
- **GitHub Discussions**: Feature discussions and ideas
- **Issues**: Bug reports and feature requests

### **Maintainer Contact**

- **Technical Issues**: Create GitHub issue
- **Funding Questions**: Drips Network platform
- **Community Matters**: Discord #moderators

## 🎯 Success Metrics

### **Quality Metrics**

- Code coverage > 90%
- Zero critical security issues
- Performance benchmarks met
- User satisfaction > 4.5/5

### **Community Metrics**

- Active contributors per month
- Issues resolved per month
- Documentation completeness
- Community engagement rate

### **Impact Metrics**

- Vulnerabilities prevented
- Contracts secured
- Developer adoption
- Security awareness raised

## 🚀 Next Steps

1. **Join our Discord** and introduce yourself
2. **Browse available issues** on GitHub
3. **Start with a good first issue** to get familiar
4. **Apply for funding** through Drips Network
5. **Make your mark** on Stellar security!

---

**Thank you for contributing to Stellar security! 🌟**

Your work helps make the entire Stellar ecosystem safer and more reliable for everyone.

# 🌟 Soroban Security Scanner

A comprehensive security scanning platform for Soroban smart contracts on the Stellar network. This platform enables invariant-driven development by enforcing core business logic and state consistency properties to prevent logic vulnerabilities.

## 🏗️ Architecture

This project uses a microservices architecture with the following components:

- **🌐 Frontend** - Modern web interface built with Next.js
- **⚙️ Backend** - Rust-based API server with Axum
- **🔍 Core Scanner** - Security analysis engine
- **🔒 Smart Contracts** - Soroban contracts for on-chain functionality

## 🚀 Quick Start

### Prerequisites
- Node.js 18+
- Rust 1.70+
- Soroban CLI
- Docker & Docker Compose

### Installation

1. Clone the repository:
```bash
git clone https://github.com/connect-boiz/soroban-security-scanner.git
cd soroban-security-scanner
```

2. Install dependencies:
```bash
# Frontend
cd frontend
npm install

# Backend
cd ../backend
cargo build

# Smart Contract
cd ../contracts
cargo build
```

3. Start the development environment:
```bash
docker-compose up -d
```

## 📦 Repository Structure

```
soroban-security-scanner/
├── frontend/                 # Next.js web application
├── backend/                  # Rust API server
├── core-scanner/            # Security analysis engine
├── contracts/               # Soroban smart contracts
├── docs/                    # Documentation
├── scripts/                 # Development scripts
├── docker-compose.yml       # Development environment
└── README.md               # This file
```

## 🔍 Supported Vulnerability Types

### Access Control
- Missing Access Control
- Weak Access Control
- Unauthorized Mint/Burn
- Admin Function Exposure

### Token Economics
- Infinite Mint
- Inflation Bugs
- Reentrancy Attacks
- Integer Overflow/Underflow

### Logic Vulnerabilities
- Frozen Funds
- Broken Invariants
- Race Conditions
- Front-running Susceptibility

### Stellar-Specific
- Insufficient Fee Bump
- Invalid Time Bounds
- Weak Signature Verification
- Stellar Asset Manipulation

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### For Developers
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

### For Security Researchers
- Join our bounty program
- Report vulnerabilities responsibly
- Help improve our detection patterns

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📞 Support

- [Discord Community](https://discord.gg/soroban)
- [GitHub Issues](https://github.com/connect-boiz/soroban-security-scanner/issues)
- [Documentation](https://docs.soroban-scanner.dev)

## 🎉 Join Us in Securing Stellar!

Help us build the most comprehensive security platform for Soroban smart contracts. Every contribution makes the Stellar ecosystem safer for everyone.

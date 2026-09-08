# 🌟 Soroban Security Scanner

A comprehensive security scanning platform for **Soroban smart contracts** on the **Stellar** network. The platform combines a Rust-powered static analysis engine, a Next.js frontend, and on-chain Soroban contracts (bounty marketplace, escrow, role-based access control, multi-signature governance) to help teams build and verify safe smart contracts.

![Build & Coverage](https://github.com/connect-boiz/soroban-security-scanner/actions/workflows/main.yml/badge.svg)
![Contract coverage](https://img.shields.io/badge/contract%20coverage-96%25-success)
![Coverage gate](https://img.shields.io/badge/coverage%20gate-%E2%89%A580%25-blue)
![Vercel](https://img.shields.io/badge/deployment-Vercel-black)
![License](https://img.shields.io/badge/license-MIT-green)

---

## 📦 Repository Structure

```
soroban-security-scanner/
├── contracts/                 # Soroban smart contracts (Rust, soroban-sdk 26)
│   ├── scanner/               #   SecurityScanner contract (bounties, escrow, RBAC, multi-sig)
│   │   ├── src/lib.rs         #   Contract implementation
│   │   ├── src/test.rs        #   Unit tests
│   │   └── tests/             #   Integration tests (e.g. ed25519 release signatures)
│   └── scripts/               #   Testnet deployment scripts
├── frontend/                  # Next.js 14 web application
├── component-library/         # Shared React component library (@soroban-scanner/ui-components)
├── src/                       # Rust backend: static analysis engine, auth, API (axum)
├── tests/                     # Backend & e2e test suites
├── scripts/                   # Ops/dev scripts
├── .github/workflows/         # CI/CD (main.yml, deploy-frontend.yml, ci.yml)
└── docs/                      # Deep-dive documentation
```

## 🏗️ Architecture

```
                         ┌──────────────────────────────┐
                         │         Next.js Frontend      │
                         │  Scanner UI · Dashboard ·     │
                         │  MultiSig Wizard · Escrows    │
                         └──────────────┬───────────────┘
                                        │ REST / HTTP
                         ┌──────────────▼───────────────┐
                         │       Rust Backend (axum)     │
                         │  Static analysis · AST/Syn    │
                         │  Auth/JWT · Rate limiting ·   │
                         │  Batch ops · Audit trail      │
                         └──────────────┬───────────────┘
                                        │ Soroban RPC
                         ┌──────────────▼───────────────┐
                         │   Soroban Smart Contracts    │
                         │  Bounty pool · Escrow · RBAC │
                         │  Multi-sig · Emergency alerts│
                         └──────────────────────────────┘
```

### Smart contract highlights (`contracts/scanner`)

- **Bounty marketplace** — report vulnerabilities, verify them, and pay bounties from a funded pool.
- **Escrow engine** — time-locked escrows with optional ed25519 release signers; releases are replay-safe via per-escrow nonces and canonical signed messages.
- **Role-based access control** — `SuperAdmin`, `Verifier`, `EscrowManager`, `TreasuryManager` with granular permissions.
- **Multi-signature governance** — high bounties, emergency verifications and role grants require multi-sig proposals with approval thresholds and time-locks.
- **Structured 200-code error system** — every failure state maps to a stable numerical code via `ContractError::code()` (domains: auth `1-19`, validation `20-39`, lookup `40-59`, funds `60-79`, escrow `80-99`, proposals `100-119`, roles `120-139`, emergency `140-159`, arithmetic `160-179`), with reverse lookup via `ContractError::from_code()`.
- **Gas optimizations** — enum-like values (status/purpose/severity) are stored as `Symbol`s instead of `String`s; storage maps are loaded once per operation instead of repeatedly; multi-sig execution skips redundant nested `require_auth` calls; checked arithmetic throughout.

## 🚀 Prerequisites

- **Node.js 18+** and npm
- **Rust** (stable) with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- **Stellar CLI** (formerly soroban-cli) for contract deployment: <https://github.com/stellar/stellar-cli>
- **Docker & Docker Compose** (optional, for local backend services)

## 💻 Local Setup

```bash
git clone https://github.com/connect-boiz/soroban-security-scanner.git
cd soroban-security-scanner
```

### Smart contract

```bash
cd contracts
cargo build -p security_scanner          # host build
cargo build --target wasm32v1-none --release -p security_scanner   # WASM build
```

### Frontend

The frontend depends on the local component library, so build it first:

```bash
cd component-library
npm ci && npm run build
cd ../frontend
npm ci
npm run dev        # http://localhost:3000
```

### Backend

```bash
npm ci             # root workspace (scripts, node backend, tests)
npm run build      # see package.json for the full build
```

## 🧪 Tests & Coverage

### Smart contract (Rust)

```bash
cd contracts
cargo test -p security_scanner                    # 48 unit + 5 integration tests
```

Coverage report with [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) (line coverage is **96%**):

```bash
cargo install cargo-llvm-cov
cargo llvm-cov -p security_scanner                # terminal summary
cargo llvm-cov -p security_scanner --open         # HTML report
cargo llvm-cov -p security_scanner --fail-under-lines 80   # CI gate
```

CI enforces a **minimum 80% line-coverage gate** on the contract in `.github/workflows/main.yml`.

### Frontend (Jest + React Testing Library)

```bash
cd frontend
npm test                      # run tests
npm run test:coverage         # coverage report (coverage/)
```

Line/statement coverage is **~81%** (387 tests across 44 suites). CI enforces a **minimum 80% coverage gate** (`coverageThreshold` in `frontend/jest.config.js`) — `npx jest --coverage` fails the build if statements or lines drop below 80%.

### Node backend

```bash
npm test                      # root workspace jest suite (62 tests)
npx jest --coverage           # coverage report
```

### E2E / accessibility

```bash
npm run test:e2e              # Playwright
npm run test:a11y             # axe-core accessibility checks
```

## 🚢 Deployment

### 1. Smart contract — Stellar testnet

1. Install the Stellar CLI and generate a testnet-funded keypair:

   ```bash
   stellar keys generate alice --network testnet --fund
   ```

2. Deploy and initialize the contract:

   ```bash
   cd contracts
   ./scripts/deploy_testnet.sh
   ```

   The script builds the WASM, deploys it, and calls `initialize` with the admin address and a payment token (native XLM asset contract by default). Configure via env vars:

   | Variable        | Default            | Purpose                                   |
   |-----------------|--------------------|-------------------------------------------|
   | `KEY_ALIAS`     | `alice`            | Source-account keypair alias              |
   | `NETWORK`       | `testnet`          | Stellar CLI network alias                 |
   | `ADMIN_ADDRESS` | keypair's address  | `G...` address that owns the contract     |
   | `TOKEN_ADDRESS` | native asset ctr.  | `C...` payment token contract id          |
   | `CONTRACT_ALIAS`| `security_scanner` | Alias under which the id is stored        |

   Manual equivalent:

   ```bash
   stellar contract deploy \
     --wasm target/wasm32v1-none/release/security_scanner.wasm \
     --source-account alice --network testnet --alias security_scanner

   stellar contract invoke --id security_scanner --source-account alice --network testnet -- \
     initialize --admin G... --token C...
   ```

3. **Deployed testnet contract:** `TODO — run deploy_testnet.sh and paste the contract id here`

### 2. Frontend — Vercel

The repo ships `frontend/vercel.json`, which builds the local `component-library` dependency during install:

```json
{
  "framework": "nextjs",
  "installCommand": "npm ci --prefix ../component-library --no-audit --no-fund && npm run build --prefix ../component-library && npm ci --no-audit --no-fund",
  "buildCommand": "npm run build"
}
```

**Dashboard setup (easiest):** import the repository into Vercel, set **Root Directory** to `frontend` — the config is picked up automatically.

**CLI setup:** link the project and deploy:

```bash
npx vercel link          # once — creates .vercel/project.json
npx vercel deploy --prod
```

**Automated deploys:** `.github/workflows/deploy-frontend.yml` deploys a **preview** on every pull request and **production** on pushes to `main`/`develop`. Add these repository secrets:

| Secret               | Where to get it                                       |
|----------------------|-------------------------------------------------------|
| `VERCEL_TOKEN`       | Vercel → Account → Settings → Tokens                  |
| `VERCEL_ORG_ID`      | `npx vercel teams ls` / project settings              |
| `VERCEL_PROJECT_ID`  | `npx vercel project ls` / project settings            |

**Live URL:** `TODO — paste your Vercel deployment URL here`

## 🔄 CI/CD

- **`.github/workflows/main.yml`** — runs on every push/PR: contract tests (fmt, clippy, unit + integration), the **80% coverage gate** (`cargo llvm-cov --fail-under-lines 80`), frontend tests + coverage, node tests + coverage, and uploads all coverage artifacts.
- **`.github/workflows/deploy-frontend.yml`** — Vercel preview/production deploys.
- **`.github/workflows/ci.yml`** — full matrix: contracts, Rust backend, node backend, frontend, component library.
- **`.github/workflows/security-idor-tests.yml`** — IDOR prevention tests and static security analysis.

## 📚 Documentation

- [Smart contract deep-dive](docs/)
- [Accessibility testing](docs/ACCESSIBILITY_TESTING.md)
- [API versioning](docs/API_VERSIONING.md)
- [Incident response](docs/INCIDENT_RESPONSE.md)

## 🤝 Contributing

1. Fork the repo and create a feature branch.
2. Run the full test matrix locally (`cargo test`, `npm test`) and keep contract line coverage ≥ 80%.
3. Open a PR — CI runs tests and the coverage gate automatically.

## 📄 License

MIT — see [LICENSE](LICENSE).

---

**Built with ❤️ for the Stellar community.**
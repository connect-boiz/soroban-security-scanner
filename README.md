# 🌟 Soroban Security Scanner

A comprehensive security scanning platform for **Soroban smart contracts** on the **Stellar** network. The platform combines a Rust-powered static analysis engine, a Next.js frontend, and on-chain Soroban contracts (bounty marketplace, escrow, role-based access control, multi-signature governance) to help teams build and verify safe smart contracts.

![Build & Coverage](https://github.com/connect-boiz/soroban-security-scanner/actions/workflows/main.yml/badge.svg)
![Contract coverage](https://img.shields.io/badge/contract%20coverage-96%25-success)
![Coverage gate](https://img.shields.io/badge/coverage%20gate-%E2%89%A580%25-blue)
![Vercel](https://img.shields.io/badge/deployment-Vercel-black)
![Vercel deploy](https://github.com/connect-boiz/soroban-security-scanner/actions/workflows/deploy-frontend.yml/badge.svg)
![License](https://img.shields.io/badge/license-MIT-green)

---

## 📦 Repository Structure

```
soroban-security-scanner/
├── contracts/                 # Soroban smart contracts (Rust, soroban-sdk 27)
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
cargo build -p security_scanner   # host build (tests, clippy)
stellar contract build            # WASM build (soroban-sdk 27; requires stellar-cli v27.1.0+)
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

Coverage is **~89% statements / 82% branches / 87% functions / 90% lines** (413 tests across 47 suites). CI enforces a **minimum 80% gate on all four metrics** (`coverageThreshold` in `frontend/jest.config.js`) — `npx jest --coverage` fails the build if any metric drops below 80%.

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
     initialize --admin GCP3H546OU3IHGIFLT764EBRTA4GH2TNOBNF67CDLTLHCNFW7TTGP4CM \
                --token CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
   ```

   (The `initialize` values above are the verified live ones; `deploy_testnet.sh` also prints the new contract id in hex, which is the form to pass as `--contract_id` in later calls — see the CLI quirk note below.)

3. **Live testnet deployment** (2026-09-08):

   | Item          | Value                                                                              |
   |---------------|------------------------------------------------------------------------------------|    | Contract      | `CCFQFAVBDLBR2XH74LYG7DPTRHE4IIUDCYFTSOSM2T6HSHP5KSM7HYLH` (soroban-sdk 27)      |
   | Admin         | `GCP3H546OU3IHGIFLT764EBRTA4GH2TNOBNF67CDLTLHCNFW7TTGP4CM` (testnet keypair `alice`) |
   | Token         | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` (native XLM asset)     |
   | Explorer      | https://stellar.expert/explorer/testnet/contract/CCFQFAVBDLBR2XH74LYG7DPTRHE4IIUDCYFTSOSM2T6HSHP5KSM7HYLH |

   > **Note:** the contract was pinned to soroban-sdk 27.0.6 (stable, Protocol 27 mainnet-compatible).

   **CLI quirk — pass `BytesN` args as raw hex.** Some stellar-cli versions encode a `C...` strkey argument for a `BytesN<32>` parameter in a way the host rejects (`Error(WasmVm, InvalidAction)` / `UnreachableCodeReached` during contract decode), while the same value as 64-char hex works. Always pass `--contract_id <64-hex>` (and `--release_signer <64-hex>` for escrows with a release signer) instead of the strkey.

   Sanity check:

   ```bash
   stellar contract invoke --id security_scanner --source-account alice --network testnet -- get_bounty_pool
   # "49100000"  (funded + partially paid out during E2E verification)
   stellar contract invoke --id security_scanner --source-account alice --network testnet -- \
     get_user_roles --user GCP3H546OU3IHGIFLT764EBRTA4GH2TNOBNF67CDLTLHCNFW7TTGP4CM
   # ["SuperAdmin"]
   ```

   **Live E2E verification (2026-09-09):** the deployed contract was exercised with real testnet transactions — funded the bounty pool (+50M XLM via the native asset contract), reported + verified a vulnerability (bounty paid, reputation updated), ran the full escrow lifecycle (create → conditions-met → release, beneficiary balance +2M XLM on-chain), created/approved multi-sig proposals (high-bounty + emergency) and verified the gates (`#24 ProposalNotReady`, `#15 MultiSigRequired`), and confirmed the structured error codes live (`#10 InsufficientPermissions`, `#16 AlreadyApproved`, `#26 EscrowAlreadyReleased`, `#11 ProposalNotFound`, `#30 ZeroAmount`). Multi-sig *execution* needs ≥2 distinct approvers with a permission; bootstrapping the second approver requires a role grant that itself needs 2 approvals (unit tests grant roles directly), so live execution stops at the approval-count gate.

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

**Automated deploys:** `.github/workflows/deploy-frontend.yml` deploys a **preview** on every pull request and **production** on pushes to `main`/`develop`. It needs three repository secrets:

| Secret               | Where to get it                                       |
|----------------------|-------------------------------------------------------|
| `VERCEL_TOKEN`       | Vercel → Account → Settings → Tokens                  |
| `VERCEL_ORG_ID`      | `npx vercel teams ls` / project settings              |
| `VERCEL_PROJECT_ID`  | `npx vercel project ls` / project settings            |

**How the secrets gate works:** GitHub Actions does not allow `secrets` in job-level `if:` conditions, so the workflow uses a small `check-config` job: it writes `configured=true`/`false` to `$GITHUB_OUTPUT` depending on whether all three secrets are non-empty, and the `preview`/`production` jobs gate on that output (`if: needs.check-config.outputs.configured == 'true'`).

- **Secrets not set (default):** the `Check Vercel configuration` job runs in ~2s and both deploy jobs **skip** — the run stays green instead of failing on every push.
- **Secrets set:** deploys start automatically on the **next** push/PR to `main`/`develop` — no workflow changes needed.

To enable real deploys, add the three secrets under **Settings → Secrets and variables → Actions** and push (or re-run the workflow).

**Live URL:** **https://soroban-security-scanner.vercel.app** (production, deployed 2026-09-09).

To move deploys to CI, add the three secrets under **Settings → Secrets and variables → Actions** and push — the `production` job deploys automatically from then on (the values are `VERCEL_TOKEN` = a Vercel account token from the link above, `VERCEL_ORG_ID` = your team id, `VERCEL_PROJECT_ID` = the project id from `frontend`-linked project).

## 🔄 CI/CD

- **`.github/workflows/main.yml`** — runs on every push/PR: contract tests (fmt, clippy, unit + integration), the **80% coverage gate** (`cargo llvm-cov --fail-under-lines 80`), frontend tests + coverage (80% gate on statements/branches/functions/lines), node tests + coverage, and uploads all coverage artifacts.
- **`.github/workflows/deploy-frontend.yml`** — Vercel preview (PR) and production (push) deploys. A `check-config` job verifies the `VERCEL_TOKEN`/`VERCEL_ORG_ID`/`VERCEL_PROJECT_ID` secrets are set, and the deploy jobs skip until they are (see [Vercel deployment](#2-frontend--vercel)).
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
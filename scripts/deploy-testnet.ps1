# Deploy SecurityScanner to Stellar Testnet (PowerShell)
# Prereqs: stellar CLI 27+, rust, VS Build Tools 2022 (C++), or WSL
# Tested contract: contracts/src/lib.rs -> CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK

param(
  [string]$Network = "testnet",
  [string]$Source = "deployer",
  [string]$Alias = "security_scanner",
  [string]$Wasm = "target/wasm32v1-none/release/security_scanner.wasm"
)

$ErrorActionPreference = "Stop"

Write-Host "=== Soroban Security Scanner - Testnet Deploy ===" -ForegroundColor Cyan
Write-Host "Network: $Network  Source: $Source  Alias: $Alias"

# 1. Check toolchain
stellar --version
rustc --version
try { Get-Command link.exe -ErrorAction Stop | Out-Null; Write-Host "link.exe found" -ForegroundColor Green } catch { Write-Host "WARNING: link.exe missing - install VS Build Tools 2022 C++ or use WSL" -ForegroundColor Yellow }

# 2. Ensure identity funded
Write-Host "`n[1/4] Checking account $Source ..." -ForegroundColor Yellow
$addr = stellar keys address $Source 2>&1 | Select-Object -Last 1
Write-Host "Address: $addr"
stellar keys fund $Source --network $Network 2>&1 | Out-Null
Write-Host "Funded (Friendbot) on testnet" -ForegroundColor Green

# 3. Build
Write-Host "`n[2/4] Building contract ..." -ForegroundColor Yellow
# If using the temp template with proper workspace:
# stellar contract build  # run from C:\Users\dell\AppData\Local\Temp\opencode\security-test
# For this repo, create workspace Cargo.toml if missing:
if (-not (Test-Path "contracts/Cargo.toml")) {
  Write-Host "contracts/Cargo.toml missing - create via: stellar contract init C:\tmp\security-test --name security-scanner; copy lib.rs" -ForegroundColor Yellow
  Write-Host "Attempting cargo build for existing wasm..."
}
# Try stellar build if Cargo workspace exists
try {
  stellar contract build 2>&1 | Out-Host
  if (Test-Path $Wasm) { Write-Host "WASM built: $Wasm" -ForegroundColor Green }
} catch { Write-Host "Build skipped/failed: $_" -ForegroundColor Yellow }

# 4. Deploy
Write-Host "`n[3/4] Deploying ..." -ForegroundColor Yellow
# If wasm exists, deploy that wasm:
if (Test-Path $Wasm) {
  $id = stellar contract deploy --wasm $Wasm --source-account $Source --network $Network --alias $Alias 2>&1 | Select-Object -Last 1
  Write-Host "Deployed ID: $id" -ForegroundColor Green
  $ContractId = $id.Trim()
} else {
  # else reuse existing deployed ID
  $ContractId = "CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK"
  stellar contract alias add --alias $Alias --id $ContractId --network $Network 2>&1 | Out-Null
  Write-Host "Using existing ContractId: $ContractId (no wasm to deploy)" -ForegroundColor Green
}

# 5. Initialize if not initialized
Write-Host "`n[4/4] Initializing & verifying ..." -ForegroundColor Yellow
try {
  stellar contract invoke --id $ContractId --network $Network --source-account $Source -- initialize --admin $addr 2>&1 | Out-Host
  Write-Host "Initialized with admin $addr" -ForegroundColor Green
} catch { Write-Host "Initialize skipped (already initialized): $_" -ForegroundColor Yellow }

# Verify views
stellar contract invoke --id $ContractId --network $Network --source-account $Source --send=no -- get_bounty_pool
stellar contract invoke --id $ContractId --network $Network --source-account $Source --send=no -- get_escrow_pool_balance
stellar contract invoke --id $ContractId --network $Network --source-account $Source --send=no -- get_user_roles --user $addr

Write-Host "`nDone. Explorer: https://stellar.expert/explorer/testnet/contract/$ContractId" -ForegroundColor Cyan
Write-Host "Alias: stellar contract invoke --id $Alias --network $Network --source-account $Source -- --help"

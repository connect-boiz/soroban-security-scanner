#!/usr/bin/env bash
#
# Deploy the SecurityScanner Soroban contract to the Stellar testnet and
# initialize it with the configured admin + payment token.
#
# Prerequisites
# -------------
#   1. Rust toolchain with the `wasm32v1-none` target:
#        rustup target add wasm32v1-none
#   2. The Stellar CLI (formerly soroban-cli):
#        curl -fsSL https://github.com/stellar/stellar-cli/releases | # or:
#        brew install stellar/tap/stellar-cli   (macOS)
#   3. A testnet-funded keypair:
#        stellar keys generate alice --network testnet --fund
#
# Usage
# -----
#   ./deploy_testnet.sh
#
# Environment variables (all optional):
#   KEY_ALIAS        stellar CLI keypair alias            (default: alice)
#   NETWORK          stellar CLI network alias            (default: testnet)
#   ADMIN_ADDRESS    strkey (G...) owning the contract    (default: KEY_ALIAS public key)
#   TOKEN_ADDRESS    payment token contract id (C...)     (default: deploy native asset contract)
#   CONTRACT_ALIAS   alias under which the id is stored   (default: security_scanner)
#
# On success the deployed contract id is printed and stored under the
# CONTRACT_ALIAS so later invocations can use `--id <alias>`.
set -euo pipefail

cd "$(dirname "$0")/.."

KEY_ALIAS="${KEY_ALIAS:-alice}"
NETWORK="${NETWORK:-testnet}"
CONTRACT_ALIAS="${CONTRACT_ALIAS:-security_scanner}"

echo "==> Building contract WASM (wasm32v1-none, release)"
cargo build --target wasm32v1-none --release -p security_scanner
WASM="target/wasm32v1-none/release/security_scanner.wasm"
if [ ! -f "$WASM" ]; then
  echo "error: wasm artifact not found at $WASM" >&2
  exit 1
fi

echo "==> Deploying to '$NETWORK' with source account '$KEY_ALIAS'"
CONTRACT_ID="$(stellar contract deploy \
  --wasm "$WASM" \
  --source-account "$KEY_ALIAS" \
  --network "$NETWORK" \
  --alias "$CONTRACT_ALIAS")"
echo "    deployed: $CONTRACT_ID"

ADMIN_ADDRESS="${ADMIN_ADDRESS:-$(stellar keys address "$KEY_ALIAS")}"
if [ -z "${TOKEN_ADDRESS:-}" ]; then
  echo "==> TOKEN_ADDRESS unset — deploying the native (XLM) asset contract"
  TOKEN_ADDRESS="$(stellar contract asset deploy \
    --asset native \
    --source-account "$KEY_ALIAS" \
    --network "$NETWORK")"
  echo "    token:   $TOKEN_ADDRESS"
fi

echo "==> Initializing contract (admin=$ADMIN_ADDRESS, token=$TOKEN_ADDRESS)"
stellar contract invoke \
  --id "$CONTRACT_ALIAS" \
  --source-account "$KEY_ALIAS" \
  --network "$NETWORK" \
  -- \
  initialize \
  --admin "$ADMIN_ADDRESS" \
  --token "$TOKEN_ADDRESS"

echo
echo "✅ Contract live on $NETWORK:"
echo "   Contract id : $CONTRACT_ID"
echo "   Admin       : $ADMIN_ADDRESS"
echo "   Token       : $TOKEN_ADDRESS"
echo
echo "Interact (e.g. fund the bounty pool):"
echo "  stellar contract invoke --id $CONTRACT_ALIAS --source-account $KEY_ALIAS --network $NETWORK -- fund_bounty_pool --funder \"$ADMIN_ADDRESS\" --amount 1000000"
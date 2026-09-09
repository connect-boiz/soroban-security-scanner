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
# BytesN arguments (CLI v28 quirk)
# -------------------------------
# stellar-cli v28 encodes a `C...` strkey passed for a `BytesN<32>` parameter
# (e.g. --contract_id in report_vulnerability / report_emergency_vulnerability,
# --release_signer in create_escrow) in a way the testnet host rejects during
# contract decode (Error(WasmVm, InvalidAction) / UnreachableCodeReached).
# Always pass those args as 64-char lowercase hex — the same value as a strkey
# traps. This script therefore prints the deployed contract id in BOTH the
# strkey and hex forms; use the hex form for any --contract_id argument.
#
# On success the deployed contract id is printed (strkey + hex) and stored
# under the CONTRACT_ALIAS so later invocations can use `--id <alias>`.
set -euo pipefail

cd "$(dirname "$0")/.."

KEY_ALIAS="${KEY_ALIAS:-alice}"
NETWORK="${NETWORK:-testnet}"
CONTRACT_ALIAS="${CONTRACT_ALIAS:-security_scanner}"

echo "==> Building contract WASM (wasm32v1-none, release)"
# soroban-sdk 28+ requires building via `stellar contract build` (CLI v25.2.0+),
# which emits target/wasm32v1-none/release/<crate>.wasm
stellar contract build
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

# Print the 64-char hex form too: BytesN<32> args must be passed as hex (see
# header note), so callers need this form for e.g. --contract_id.
CONTRACT_ID_HEX=""
if command -v python3 >/dev/null 2>&1; then
  CONTRACT_ID_HEX="$(python3 - "$CONTRACT_ID" <<'PY'
import base64, sys
s = sys.argv[1]
raw = base64.b32decode(s + "=" * ((8 - len(s) % 8) % 8))
print(raw[:-2][1:].hex())  # drop 2-byte CRC + 1-byte version prefix
PY
)"
  echo "    deployed (hex): $CONTRACT_ID_HEX"
else
  echo "    (python3 not found — skipping hex form; decode the strkey offline)" >&2
fi

ADMIN_ADDRESS="${ADMIN_ADDRESS:-$(stellar keys address "$KEY_ALIAS")}"
if [ -z "${TOKEN_ADDRESS:-}" ]; then
  echo "==> TOKEN_ADDRESS unset — resolving the native (XLM) asset contract"
  # The builtin native asset contract already exists on testnet; deploying it
  # fails with Storage/ExistingValue, so look up its id instead.
  TOKEN_ADDRESS="$(stellar contract id asset \
    --asset native \
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
if [ -n "$CONTRACT_ID_HEX" ]; then
  echo "   Contract hex: $CONTRACT_ID_HEX"
fi
echo "   Admin       : $ADMIN_ADDRESS"
echo "   Token       : $TOKEN_ADDRESS"
echo
echo "Interact (e.g. fund the bounty pool):"
echo "  stellar contract invoke --id $CONTRACT_ALIAS --source-account $KEY_ALIAS --network $NETWORK -- fund_bounty_pool --funder \"$ADMIN_ADDRESS\" --amount 1000000"
echo
echo "Reminder: pass BytesN args as 64-char hex, not strkeys:"
echo "  ... -- report_vulnerability --contract_id $CONTRACT_ID_HEX --severity high ..."
# Deployments – Stellar Testnet

## SecurityScannerContract (Soroban)

| Field | Value |
|-------|-------|
| **Contract ID (latest)** | `CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK` |
| **Source** | `contracts/src/lib.rs:154` `SecurityScannerContract` |
| **Deployer** | `GA7XICWOWSRNOFLMBIAUI54UAFQLLT5YBAT6AA2HPOBDNLL54QY3NRP7` (stellar identity `deployer`) |
| **Network** | Test SDF Network ; September 2015 |
| **RPC** | `https://soroban-testnet.stellar.org` |
| **Horizon** | `https://horizon-testnet.stellar.org` |
| **Explorer** | https://stellar.expert/explorer/testnet/contract/CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK |
| **Lab** | https://lab.stellar.org/smart-contracts?network=testnet&contractId=CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK |
| **Salt (hex)** | `070494fb2c0b39584be2fa5004c4b087eac6b5d9bbbe11e81927a51585b39f0f` |
| **Deploy tx** | `8c57ed562a16e1117f31e256e7ba64247afbf1a1edbdc0da89151d9152d98b32` (2026-07-10) |
| **Init tx** | `7c2cf027617b2734f8a2b4a53453e8b2c0077efbf8ebbd90692cb44b481b0733` (`initialize` admin `GA7XIC...`) |
| **SDK** | `soroban-sdk 26.1.0` / `stellar-cli 27.0.0` / protocol v26 |

### Live state (2026-09-10, after session writes)

- `get_bounty_pool` → `1000000` (tx `7dac26658fd97f6c71fdb224b74cdf23ffff0d215058a721d4028a5a269207c6` – `fund_bounty_pool 1M`)
- `get_escrow_pool_balance` → `50000` (tx `ce1d6dac744ecfe5e3d217d5979146b0b651998dcadfddc4131179b9893124f7` – `create_escrow 50k` id 1 pending)
- `get_emergency_pool_balance` → `2000000` (tx `304a67e337e4016991e6eea4da30839abd7447237ecbea4f1344df979a702637` – `fund_emergency_pool 2M`)
- `get_user_roles(deployer)` → `["SuperAdmin"]`

### Access (verified `stellar 27.0.0`)

```bash
stellar contract alias add --alias security_scanner --id CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK --network testnet

# views (no fee)
stellar contract invoke --id CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK --network testnet --source-account deployer --send=no -- get_bounty_pool
stellar contract invoke --id CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK --network testnet --source-account deployer --send=no -- get_escrow --escrow_id 1

# writes
stellar contract invoke --id CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK --network testnet --source-account deployer -- fund_bounty_pool --funder GA7XICWOWSRNOFLMBIAUI54UAFQLLT5YBAT6AA2HPOBDNLL54QY3NRP7 --amount 1000

# all fns
stellar contract invoke --id CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK --network testnet --source-account deployer -- --help
```

Helpers: `scripts/testnet-access.mjs` and `scripts/deploy-testnet.ps1`.

### Other instances from same deployer

- `CAY5VUJBZU6KQRKXH4UM62SCEGBXMGFMZ3W6LCEBSJYVU4G5VURPIQEQ` (salt `1f65e6...af1c`)
- `CC2IIYHDYN6J4PGRARC6ZVSLEH3BFOCTNN6HXGNGNG6ELTWZYFPF5CPT` – federated-learning demo (salt `e26309...`)
- `CBGJEPGSYG2NVTJILDHO3OU5IILR5TPU77LDQA23B4HJQQNZ65UX2YRE`
- `CDUSF32RXYV7VQD272XKF24RCNYFWSV6Y6CGFCLUVDOPRJLI7BOK5G3V`

### Known bug & fix

`contracts/src/lib.rs:596` used `Symbol::short(&format!("REP_{:?}", addr))` >9 chars → trap on `report_vulnerability`/`get_reputation`. Fixed in `develop@9fae0010` to `Map<Address,Reputation>` under `REPUTATION`. Requires rebuild (`stellar contract build` needs VS Build Tools 2022 C++ `link.exe` or WSL) and redeploy with new salt; existing `CAR5Z...` still serves bounty/escrow until upgraded.

### JS SDK

`@stellar/stellar-sdk ^14.6.1` required for protocol v26; `frontend` uses `^12.0.0` (will trap on XDR parse). Example in `scripts/testnet-access.mjs`.

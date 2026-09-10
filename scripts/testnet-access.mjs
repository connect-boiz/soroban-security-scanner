#!/usr/bin/env node
// Access deployed SecurityScanner on Stellar Testnet
// Verified via stellar CLI 27 + Horizon – primary ground truth is CLI.
// SDK note: frontend @stellar/stellar-sdk ^12.0.0 may not parse protocol v26; use ^14.6.1 (root package.json:34) for full support.
// Contract: CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK
// Deployer: GA7XICWOWSRNOFLMBIAUI54UAFQLLT5YBAT6AA2HPOBDNLL54QY3NRP7
// RPC: https://soroban-testnet.stellar.org
// Usage: node scripts/testnet-access.mjs [get|horizon]
const CONTRACT_ID = process.env.CONTRACT_ID || "CAR5ZOFMMRRMRTZXIE4DN63KQV6QUHAQW4TN77WHNBT4JRITJXQKIEHK";
const RPC_URL = "https://soroban-testnet.stellar.org";
const HORIZON = "https://horizon-testnet.stellar.org";
const NETWORK = "Test SDF Network ; September 2015";

async function horizonAccount(addr) {
  const r = await fetch(`${HORIZON}/accounts/${addr}`);
  return r.json();
}
async function cliHelp() {
  console.log(`
--- Verified CLI (ground truth, stellar 27.0.0) ---

# view (no fee, send=no) – WORKS on testnet now
stellar contract invoke --id ${CONTRACT_ID} --network testnet --source-account deployer --send=no -- get_bounty_pool
# -> "0"
stellar contract invoke --id ${CONTRACT_ID} --network testnet --source-account deployer --send=no -- get_escrow_pool_balance
stellar contract invoke --id ${CONTRACT_ID} --network testnet --source-account deployer --send=no -- get_emergency_pool_balance
stellar contract invoke --id ${CONTRACT_ID} --network testnet --source-account deployer --send=no -- get_user_roles --user GA7XICWOWSRNOFLMBIAUI54UAFQLLT5YBAT6AA2HPOBDNLL54QY3NRP7
# -> ["SuperAdmin"]

# write
stellar contract invoke --id ${CONTRACT_ID} --network testnet --source-account deployer -- report_vulnerability --reporter GA7XICWOWSRNOFLMBIAUI54UAFQLLT5YBAT6AA2HPOBDNLL54QY3NRP7 --contract_id 0000000000000000000000000000000000000000000000000000000000000000 --vulnerability_type reentrancy --severity high --description "test" --location "lib.rs:10"

# all fns
stellar contract invoke --id ${CONTRACT_ID} --network testnet --source-account deployer -- --help
`);
}

async function main() {
  const cmd = process.argv[2] || "all";
  console.log(`Contract: ${CONTRACT_ID}\nRPC: ${RPC_URL}\nHorizon: ${HORIZON}\nNetwork: ${NETWORK}\n`);
  const deployer = "GA7XICWOWSRNOFLMBIAUI54UAFQLLT5YBAT6AA2HPOBDNLL54QY3NRP7";
  try {
    const acc = await horizonAccount(deployer);
    console.log(`Deployer ${deployer}: balance ${acc.balances?.[0]?.balance} XLM, seq ${acc.sequence}`);
  } catch(e){ console.log("Horizon fetch failed", e.message); }
  console.log(`Explorer: https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}`);
  console.log(`Lab: https://lab.stellar.org/smart-contracts?network=testnet&contractId=${CONTRACT_ID}`);
  await cliHelp();

  if (cmd==="horizon") {
    const ops = await fetch(`${HORIZON}/accounts/${deployer}/operations?limit=5&order=desc`).then(r=>r.json());
    console.log(JSON.stringify(ops._embedded.records.slice(0,3), null, 2));
  }

  console.log(`
--- JS SDK (requires @stellar/stellar-sdk ^14.6.1 for protocol v26) ---
# npm install @stellar/stellar-sdk@^14.6.1  # root already has it in package.json:35
import { Contract, rpc, TransactionBuilder, Networks, scValToNative } from '@stellar/stellar-sdk';
const server = new rpc.Server("${RPC_URL}");
const c = new Contract("${CONTRACT_ID}");
const acc = await server.getAccount("${deployer}");
const tx = new TransactionBuilder(acc, {fee:"1000", networkPassphrase: Networks.TESTNET})
  .addOperation(c.call("get_bounty_pool")).setTimeout(30).build();
const sim = await server.simulateTransaction(tx);
console.log(scValToNative(sim.result.retval));
# If you use frontend SDK 12.0.0 you will hit XDR Bad union switch on v26 – upgrade.
`);
}

main().catch(e=>{console.error(e); process.exit(1)});

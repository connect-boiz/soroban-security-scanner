/**
 * Typed help content for various features in the Soroban Security Scanner.
 * These strings are used in tooltips and help panels throughout the application.
 */
export const HELP_CONTENT = {
  scan: {
    title: "Scan Submission",
    description: "Submit your Soroban smart contracts for comprehensive security analysis.",
    contractId: "The unique identifier of your deployed Soroban contract on the Stellar network. This is usually a 56-character string starting with 'C'.",
    vulnerabilityTypes: "Select which vulnerability categories to scan for. You can choose specific types like Reentrancy, Arithmetic Overflows, or choose 'All' for a complete audit.",
    scanDepth: "Controls how thoroughly the scanner analyses your contract. 'Basic' is fast, 'Deep' performs symbolic execution, and 'Exhaustive' runs formal verification.",
  },
  vulnerability: {
    title: "Vulnerability Review",
    description: "Detailed analysis of detected security issues and recommended remediations.",
    severity: "Severity ratings (Low, Medium, High, Critical) indicate the risk level based on the potential impact and ease of exploitation.",
    cvssScore: "Common Vulnerability Scoring System (CVSS) — a standardised 0–10 scale that provides a numerical score reflecting the severity of a vulnerability.",
    invariant: "An invariant is a security property that must never be violated. Violated invariants often point to deep logical flaws in the contract.",
    remediation: "Step-by-step instructions on how to patch the vulnerability and prevent similar issues in the future.",
  },
  timeTravelDebugger: {
    title: "Time Travel Debugger",
    description: "Step through contract execution against historical ledger states.",
    ledgerSequence: "The specific ledger sequence number to fork from. This allows you to recreate the exact network state at that point in time.",
    contractUpgrade: "Simulates the deployment of a new WASM version of your contract, allowing you to test upgrade logic and state migrations before going live.",
    forking: "Forking creates a local instance of the network state, allowing you to run transactions without affecting the main network.",
  },
  batchOperations: {
    title: "Batch Operations",
    description: "Efficiently process multiple security tasks in a single workflow.",
    escrowRelease: "Processes multiple escrow releases in one transaction, significantly reducing gas costs and administrative overhead.",
    verification: "Verifies multiple vulnerabilities simultaneously across different contracts or ledger states.",
    batchSize: "The number of operations to include in a single batch. Larger batches are more efficient but may hit transaction limits.",
  },
} as const;

export type HelpTopic = keyof typeof HELP_CONTENT;

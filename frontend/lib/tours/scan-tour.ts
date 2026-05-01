import { Step } from 'react-joyride';

export const SCAN_TOUR_STEPS: Step[] = [
  {
    target: 'body',
    placement: 'center',
    title: 'Welcome to the Scanner',
    content: "Let's walk through submitting your first security scan for a Soroban smart contract.",
  },
  {
    target: '#contract-code',
    title: 'Contract Code',
    content: "Paste your Rust or WASM contract code here. Our engine will analyze it for common vulnerabilities.",
  },
  {
    target: '#vulnerability-types',
    title: 'Vulnerability Categories',
    content: "Select specific categories to focus the scan, or keep 'All' for a comprehensive audit.",
  },
  {
    target: '#scan-depth',
    title: 'Scan Depth',
    content: "Choose how deep you want the analysis to go. Deeper scans use symbolic execution to find edge cases.",
  },
  {
    target: '#submit-scan-btn',
    title: 'Submit Scan',
    content: "Once you're ready, click here to start the analysis. You'll see progress updates in real-time.",
  },
  {
    target: 'h2:contains("Scan Results")', // Note: This target might only appear after scan
    title: 'View Results',
    content: "After the scan completes, your results will appear here with detailed severity ratings.",
  }
];

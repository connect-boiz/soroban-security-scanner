import { Step } from 'react-joyride';

export const TIME_TRAVEL_TOUR_STEPS: Step[] = [
  {
    target: '#ledger-sequence',
    title: 'Ledger Sequence',
    content: 'Enter the specific ledger number you want to fork the network state from.',
  },
  {
    target: '[aria-label="Help: Contract Upgrade"]',
    title: 'Contract Upgrade',
    content: 'Enable this to simulate deploying a new WASM version on top of the historical state.',
  },
  {
    target: 'button:contains("Fork Network State")',
    title: 'Fork Action',
    content: 'Clicking this creates a virtual fork of the Stellar network for isolated testing.',
  },
];

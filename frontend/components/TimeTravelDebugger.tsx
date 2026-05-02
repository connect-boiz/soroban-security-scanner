'use client';

import React, { useState } from 'react';
import HelpIcon from './help/HelpIcon';
import { HELP_CONTENT } from '../lib/help-content';

const TimeTravelDebugger: React.FC = () => {
  const [ledgerSequence, setLedgerSequence] = useState('123456');

  return (
    <div className="bg-white rounded-lg shadow-md p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900">Time Travel Debugger</h2>
      </div>
      
      <div className="space-y-4">
        <div>
          <label htmlFor="ledger-sequence" className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2">
            Ledger Sequence
            <HelpIcon content={HELP_CONTENT.timeTravelDebugger.ledgerSequence} label="Ledger Sequence" />
          </label>
          <input
            id="ledger-sequence"
            type="number"
            value={ledgerSequence}
            onChange={(e) => setLedgerSequence(e.target.value)}
            className="input"
            placeholder="Enter ledger sequence..."
          />
        </div>

        <div className="p-4 bg-gray-50 rounded-lg border border-gray-200">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-medium text-gray-800">Advanced Options</h3>
            <HelpIcon content={HELP_CONTENT.timeTravelDebugger.contractUpgrade} label="Contract Upgrade" />
          </div>
          <label className="flex items-center space-x-3">
            <input type="checkbox" className="w-4 h-4 text-blue-600 border-gray-300 rounded" />
            <span className="text-sm text-gray-700">Simulate Contract Upgrade</span>
          </label>
        </div>

        <button className="btn btn-primary w-full">
          Fork Network State
        </button>
      </div>
    </div>
  );
};

export default TimeTravelDebugger;

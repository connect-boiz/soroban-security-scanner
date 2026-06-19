'use client';

import React, { useState } from 'react';
import HelpIcon from './help/HelpIcon';
import { HELP_CONTENT } from '../lib/help-content';

const BatchOperations: React.FC = () => {
  const [batchSize, setBatchSize] = useState(10);

  return (
    <div className="bg-white rounded-lg shadow-md p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900">Batch Operations</h2>
      </div>

      <div className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label
              htmlFor="batch-size"
              className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2"
            >
              Batch Size
              <HelpIcon content={HELP_CONTENT.batchOperations.batchSize} label="Batch Size" />
            </label>
            <input
              id="batch-size"
              type="number"
              value={batchSize}
              onChange={e => setBatchSize(parseInt(e.target.value))}
              className="input"
            />
          </div>
          <div>
            <label
              htmlFor="executor"
              className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2"
            >
              Executor Address
              <HelpIcon
                content="The address that will sign and execute the batch transaction."
                label="Executor"
              />
            </label>
            <input id="executor" type="text" className="input" placeholder="G..." />
          </div>
        </div>

        <div>
          <label
            htmlFor="escrow-ids"
            className="flex items-center gap-1 text-sm font-medium text-gray-700 mb-2"
          >
            Escrow IDs
            <HelpIcon content={HELP_CONTENT.batchOperations.escrowRelease} label="Escrow IDs" />
          </label>
          <textarea
            id="escrow-ids"
            className="w-full h-24 p-3 border border-gray-300 rounded-md"
            placeholder="Enter comma-separated escrow IDs..."
          />
        </div>

        <button className="btn btn-primary w-full">Execute Batch Release</button>
      </div>
    </div>
  );
};

export default BatchOperations;

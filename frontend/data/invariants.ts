import { SorobanVariable, InvariantTemplate } from '@/types/invariant';

// Standard Soroban state variables
export const SOROBAN_VARIABLES: SorobanVariable[] = [
  // Balance variables
  {
    id: 'token_balance',
    name: 'Token Balance',
    category: 'balance',
    type: 'number',
    description: 'Balance of a specific token for an address',
    examples: ['1000', '0', '500000000']
  },
  {
    id: 'native_balance',
    name: 'Native XLM Balance',
    category: 'balance',
    type: 'number',
    description: 'Native XLM balance for an address',
    examples: ['100', '0.5', '10000000']
  },
  {
    id: 'total_supply',
    name: 'Total Token Supply',
    category: 'token',
    type: 'number',
    description: 'Total supply of a token',
    examples: ['1000000000', 'unlimited']
  },
  {
    id: 'circulating_supply',
    name: 'Circulating Supply',
    category: 'token',
    type: 'number',
    description: 'Circulating supply of a token',
    examples: ['500000000', '750000000']
  },
  {
    id: 'token_decimals',
    name: 'Token Decimals',
    category: 'token',
    type: 'number',
    description: 'Number of decimal places for a token',
    examples: ['7', '18', '0']
  },
  {
    id: 'allowance',
    name: 'Token Allowance',
    category: 'token',
    type: 'number',
    description: 'Allowance amount for token spending',
    examples: ['1000', 'unlimited']
  },
  
  // Contract state variables
  {
    id: 'contract_balance',
    name: 'Contract Balance',
    category: 'contract',
    type: 'number',
    description: 'Balance held by a contract',
    examples: ['1000', '0', '500000']
  },
  {
    id: 'contract_owner',
    name: 'Contract Owner',
    category: 'contract',
    type: 'address',
    description: 'Owner address of a contract',
    examples: ['GD...', 'GAB...']
  },
  {
    id: 'contract_paused',
    name: 'Contract Paused State',
    category: 'contract',
    type: 'boolean',
    description: 'Whether the contract is paused',
    examples: ['true', 'false']
  },
  {
    id: 'contract_initialized',
    name: 'Contract Initialized',
    category: 'contract',
    type: 'boolean',
    description: 'Whether the contract has been initialized',
    examples: ['true', 'false']
  },
  {
    id: 'user_count',
    name: 'User Count',
    category: 'contract',
    type: 'number',
    description: 'Number of unique users interacting with contract',
    examples: ['100', '1000', '0']
  },
  {
    id: 'transaction_count',
    name: 'Transaction Count',
    category: 'contract',
    type: 'number',
    description: 'Total number of transactions',
    examples: ['10000', '50000', '0']
  },
  
  // Custom variables
  {
    id: 'custom_string',
    name: 'Custom String Value',
    category: 'custom',
    type: 'string',
    description: 'Custom string variable from contract state',
    examples: ['"active"', '"paused"', '"pending"']
  },
  {
    id: 'custom_number',
    name: 'Custom Number Value',
    category: 'custom',
    type: 'number',
    description: 'Custom number variable from contract state',
    examples: ['100', '0.5', '1000000']
  },
  {
    id: 'custom_address',
    name: 'Custom Address Value',
    category: 'custom',
    type: 'address',
    description: 'Custom address variable from contract state',
    examples: ['GD...', 'GAB...']
  },
  {
    id: 'custom_boolean',
    name: 'Custom Boolean Value',
    category: 'custom',
    type: 'boolean',
    description: 'Custom boolean variable from contract state',
    examples: ['true', 'false']
  }
];

// Pre-built DeFi invariant templates
export const DEFI_TEMPLATES: InvariantTemplate[] = [
  {
    id: 'token_supply_balance',
    name: 'Token Supply = Sum of Balances',
    description: 'Total token supply must equal the sum of all user balances',
    category: 'Token Economics',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'total_supply')!,
        operator: 'equals',
        value: 'sum_of_balances',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Ensures no token inflation or deflation bugs'
  },
  {
    id: 'no_negative_balances',
    name: 'No Negative Balances',
    description: 'All user balances must be non-negative',
    category: 'Balance Safety',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'token_balance')!,
        operator: 'greater_than_or_equal',
        value: '0',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Prevents underflow attacks and negative balance bugs'
  },
  {
    id: 'allowance_limit',
    name: 'Allowance ≤ Balance',
    description: 'Token allowance cannot exceed actual balance',
    category: 'Token Safety',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'allowance')!,
        operator: 'less_than_or_equal',
        value: 'token_balance',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Prevents overspending allowances'
  },
  {
    id: 'contract_not_empty',
    name: 'Contract Balance > 0',
    description: 'Contract must maintain minimum balance',
    category: 'Contract Safety',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'contract_balance')!,
        operator: 'greater_than',
        value: '0',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Ensures contract has enough funds for operations'
  },
  {
    id: 'owner_protection',
    name: 'Owner Cannot Drain Contract',
    description: 'Contract owner cannot withdraw all funds',
    category: 'Access Control',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'contract_balance')!,
        operator: 'greater_than',
        value: 'minimum_reserve',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Prevents rug pull attacks'
  },
  {
    id: 'pause_protection',
    name: 'Critical Functions When Paused',
    description: 'Critical functions must be disabled when contract is paused',
    category: 'Emergency Controls',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'contract_paused')!,
        operator: 'equals',
        value: 'true',
        valueType: 'boolean'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Ensures emergency pause functionality works correctly'
  },
  {
    id: 'single_owner',
    name: 'Single Contract Owner',
    description: 'Contract must have exactly one owner',
    category: 'Access Control',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'contract_owner')!,
        operator: 'is_not_empty',
        value: '',
        valueType: 'address'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Prevents ownership confusion and attacks'
  },
  {
    id: 'decimals_consistency',
    name: 'Token Decimals Consistency',
    description: 'Token decimals must be consistent across operations',
    category: 'Token Economics',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'token_decimals')!,
        operator: 'greater_than_or_equal',
        value: '0',
        valueType: 'number'
      },
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'token_decimals')!,
        operator: 'less_than_or_equal',
        value: '18',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Ensures proper decimal handling in calculations'
  },
  {
    id: 'circulating_supply_limit',
    name: 'Circulating Supply ≤ Total Supply',
    description: 'Circulating supply cannot exceed total supply',
    category: 'Token Economics',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'circulating_supply')!,
        operator: 'less_than_or_equal',
        value: 'total_supply',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Prevents supply manipulation attacks'
  },
  {
    id: 'minimum_reserve',
    name: 'Minimum Reserve Maintenance',
    description: 'Contract must maintain minimum reserve ratio',
    category: 'DeFi Safety',
    conditions: [
      {
        variable: SOROBAN_VARIABLES.find(v => v.id === 'contract_balance')!,
        operator: 'greater_than_or_equal',
        value: 'total_supply * 0.1',
        valueType: 'number'
      }
    ],
    logicOperator: 'AND',
    useCase: 'Ensures liquidity and solvency requirements'
  }
];

// Helper functions
export const getVariableById = (id: string): SorobanVariable | undefined => {
  return SOROBAN_VARIABLES.find(v => v.id === id);
};

export const getVariablesByCategory = (category: string): SorobanVariable[] => {
  return SOROBAN_VARIABLES.filter(v => v.category === category);
};

export const getTemplateById = (id: string): InvariantTemplate | undefined => {
  return DEFI_TEMPLATES.find(t => t.id === id);
};

export const getTemplatesByCategory = (category: string): InvariantTemplate[] => {
  return DEFI_TEMPLATES.filter(t => t.category === category);
};
